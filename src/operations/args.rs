use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::{Map, Value, json};

use crate::config::{LAMBDA_ASYNC_PAYLOAD_BYTES, LAMBDA_SYNC_PAYLOAD_BYTES, S3_MAX_UPLOAD_BYTES};

use super::{InvokeMode, Operation};

pub(super) fn json_fields(input: &mut Value, value: &str) -> Result<(), String> {
    let fields: Map<String, Value> =
        serde_json::from_str(value).map_err(|error| format!("Enter a JSON object: {error}"))?;
    if fields.is_empty() {
        return Err("Enter a non-empty JSON object".into());
    }

    let input = object(input)?;
    for key in fields.keys() {
        if input.contains_key(key) || key == "registryId" {
            return Err(format!("Cannot replace the captured field `{key}`"));
        }
    }
    input.extend(fields);
    Ok(())
}

pub(super) fn tag_value(
    operation: &Operation,
    input: &mut Value,
    value: &str,
) -> Result<(), String> {
    let field = match (operation.service, operation.name) {
        ("ec2", "create-tags") => "Tags",
        ("ecr", "tag-resource") => "tags",
        _ => return Err("Tag value is only valid for EC2 or ECR tags".into()),
    };
    let tag = input
        .get_mut(field)
        .and_then(Value::as_array_mut)
        .filter(|tags| tags.len() == 1)
        .and_then(|tags| tags[0].as_object_mut())
        .filter(|tag| tag.get("Key").and_then(Value::as_str).is_some())
        .ok_or("Tag input must contain one captured key")?;
    tag.insert("Value".into(), json!(value));
    Ok(())
}

pub(super) fn lambda_tag_value(
    operation: &Operation,
    input: &mut Value,
    value: &str,
) -> Result<(), String> {
    require(
        operation,
        "lambda",
        "tag-resource",
        "Lambda tag value is only valid for Lambda tags",
    )?;
    let tags = input
        .get_mut("Tags")
        .and_then(Value::as_object_mut)
        .filter(|tags| tags.len() == 1)
        .ok_or("Tag input must contain one captured key")?;
    let key = tags
        .keys()
        .next()
        .cloned()
        .ok_or("Tag input must contain one captured key")?;
    tags.insert(key, json!(value));
    Ok(())
}

pub(super) fn invoke(
    operation: &Operation,
    mode: InvokeMode,
    value: &str,
) -> Result<Vec<String>, String> {
    require(
        operation,
        "lambda",
        "invoke",
        "Invoke payload is only valid for Lambda invocations",
    )?;
    let function = operation
        .input
        .get("FunctionName")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or("Invoke input must contain a function name")?;
    let payload = value.trim();
    let limit = match mode {
        InvokeMode::Async => LAMBDA_ASYNC_PAYLOAD_BYTES,
        InvokeMode::Sync | InvokeMode::SyncWithLogs => LAMBDA_SYNC_PAYLOAD_BYTES,
    };
    if !payload.is_empty() {
        if payload.len() > limit {
            return Err("Invoke payload exceeds the Lambda size limit".into());
        }
        serde_json::from_str::<Value>(payload)
            .map_err(|error| format!("Enter valid JSON payload or leave empty: {error}"))?;
    }

    let mut args = vec![
        operation.service.to_string(),
        operation.name.to_string(),
        "--function-name".into(),
        function.to_string(),
    ];
    if !payload.is_empty() {
        // AWS CLI v2 reads blob arguments as base64 unless told otherwise.
        args.extend([
            "--payload".into(),
            payload.to_string(),
            "--cli-binary-format".into(),
            "raw-in-base64-out".into(),
        ]);
    }
    match mode {
        InvokeMode::Sync => {}
        InvokeMode::SyncWithLogs => args.extend(["--log-type".into(), "Tail".into()]),
        InvokeMode::Async => args.extend(["--invocation-type".into(), "Event".into()]),
    }
    args.push(crate::aws::INVOKE_OUTPUT_PLACEHOLDER.into());
    Ok(args)
}

pub(super) fn reserved_concurrency(
    operation: &Operation,
    input: &mut Value,
    value: &str,
) -> Result<(), String> {
    require(
        operation,
        "lambda",
        "put-function-concurrency",
        "Reserved concurrency is only valid for Lambda concurrency limits",
    )?;
    let count = bounded_count(
        value,
        0,
        "Reserved concurrency must be a non-negative integer",
    )?;
    object(input)?.insert("ReservedConcurrentExecutions".into(), json!(count));
    Ok(())
}

pub(super) fn provisioned_concurrency(
    operation: &Operation,
    input: &mut Value,
    value: &str,
) -> Result<(), String> {
    require(
        operation,
        "lambda",
        "put-provisioned-concurrency-config",
        "Provisioned concurrency is only valid for Lambda versions or aliases",
    )?;
    let count = bounded_count(
        value,
        1,
        "Provisioned concurrency must be a positive integer",
    )?;
    object(input)?.insert("ProvisionedConcurrentExecutions".into(), json!(count));
    Ok(())
}

pub(super) fn alias_version(
    operation: &Operation,
    input: &mut Value,
    value: &str,
) -> Result<(), String> {
    require(
        operation,
        "lambda",
        "update-alias",
        "Alias version is only valid for Lambda alias updates",
    )?;
    let version = value.trim();
    let valid = version == "$LATEST" || version.parse::<u32>().is_ok_and(|revision| revision > 0);
    if !valid {
        return Err("Enter a published version number or $LATEST".into());
    }

    object(input)?.insert("FunctionVersion".into(), json!(version));
    Ok(())
}

pub(super) fn task_definition_revision(
    operation: &Operation,
    input: &mut Value,
    value: &str,
) -> Result<(), String> {
    require(
        operation,
        "ecs",
        "update-service",
        "Task definition revision is only valid for ECS service updates",
    )?;
    let definition = value.trim();
    let family_revision = if definition.starts_with("arn:") {
        let (context, revision) = definition
            .split_once(":task-definition/")
            .ok_or("Enter a complete ECS task definition ARN")?;
        let selected_context = operation
            .target
            .split_once(":service/")
            .map(|(context, _)| context)
            .ok_or("Use family:revision when the selected service ARN is unavailable")?;
        if context != selected_context || context.split(':').nth(2) != Some("ecs") {
            return Err(
                "Task definition ARN must match the selected service account and region".into(),
            );
        }
        revision
    } else {
        definition
    };

    let valid = family_revision
        .split_once(':')
        .is_some_and(|(family, revision)| {
            is_identifier(family) && revision.parse::<u32>().is_ok_and(|revision| revision > 0)
        });
    if !valid || (definition.starts_with("arn:") && family_revision == definition) {
        return Err(
            "Enter a task definition family:revision or ARN with a positive revision".into(),
        );
    }

    object(input)?.insert("taskDefinition".into(), json!(definition));
    Ok(())
}

pub(super) fn service_deployment_arn(
    operation: &Operation,
    input: &mut Value,
    value: &str,
) -> Result<(), String> {
    require(
        operation,
        "ecs",
        "stop-service-deployment",
        "Service deployment ARN is only valid for ECS rollback",
    )?;
    let deployment = value.trim();
    let (account, service) = operation
        .target
        .split_once(":service/")
        .filter(|(account, service)| account.starts_with("arn:") && service.split('/').count() == 2)
        .ok_or("The selected service must have a cluster-qualified ARN")?;
    let prefix = format!("{account}:service-deployment/{service}/");
    if !deployment.strip_prefix(&prefix).is_some_and(is_identifier) {
        return Err(
            "Deployment ARN must belong to the selected service, account, and region".into(),
        );
    }

    object(input)?.insert("serviceDeploymentArn".into(), json!(deployment));
    Ok(())
}

pub(super) fn desired_task_count(
    operation: &Operation,
    input: &mut Value,
    value: &str,
) -> Result<(), String> {
    require(
        operation,
        "ecs",
        "update-service",
        "Desired task count is only valid for ECS service updates",
    )?;
    let count = bounded_count(
        value,
        0,
        "Desired task count must be a non-negative integer",
    )?;
    object(input)?.insert("desiredCount".into(), json!(count));
    Ok(())
}

pub(super) fn upload_file(
    operation: &Operation,
    input: &mut Value,
    value: &str,
) -> Result<Vec<String>, String> {
    require(
        operation,
        "s3api",
        "put-object",
        "Upload file path is only valid for S3 object uploads",
    )?;
    let path = upload_path(value)?;
    if !input.get("Key").is_some_and(|key| key.as_str().is_some()) {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .ok_or("Upload file name must be valid UTF-8")?;
        object(input)?.insert("Key".into(), Value::String(name.into()));
    }

    Ok(vec!["--body".into(), path_string(&path)?])
}

pub(super) fn download_file(
    operation: &Operation,
    input: &Value,
    value: &str,
) -> Result<Vec<String>, String> {
    require(
        operation,
        "s3api",
        "get-object",
        "Download file path is only valid for S3 object downloads",
    )?;
    let path = download_path(value)?;
    let bucket = input
        .get("Bucket")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or("Download input must contain a bucket")?;
    let key = input
        .get("Key")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or("Download input must contain an object key")?;

    // The streaming GetObject command does not support --cli-input-json.
    Ok(vec![
        operation.service.into(),
        operation.name.into(),
        format!("--bucket={bucket}"),
        format!("--key={key}"),
        path_string(&path)?,
    ])
}

fn require(
    operation: &Operation,
    service: &str,
    name: &str,
    message: &'static str,
) -> Result<(), String> {
    if operation.service == service && operation.name == name {
        Ok(())
    } else {
        Err(message.into())
    }
}

fn object(input: &mut Value) -> Result<&mut Map<String, Value>, String> {
    input
        .as_object_mut()
        .ok_or_else(|| "Operation input must be a JSON object".into())
}

fn bounded_count(value: &str, minimum: i32, message: &str) -> Result<i32, String> {
    value
        .trim()
        .parse::<i32>()
        .ok()
        .filter(|count| *count >= minimum)
        .ok_or_else(|| message.to_string())
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_".contains(c))
}

fn upload_path(value: &str) -> Result<PathBuf, String> {
    if value.is_empty() {
        return Err("Upload file path must not be empty".into());
    }

    let path = fs::canonicalize(value)
        .map_err(|error| format!("cannot open upload file `{value}`: {error}"))?;
    if !path.is_file() {
        return Err("Upload path must be a regular file".into());
    }
    if fs::metadata(&path)
        .map_err(|error| format!("cannot read upload file metadata: {error}"))?
        .len()
        > S3_MAX_UPLOAD_BYTES
    {
        return Err("Upload file exceeds the 5 GB single-request limit".into());
    }
    Ok(path)
}

fn download_path(value: &str) -> Result<PathBuf, String> {
    if value.is_empty() {
        return Err("Download file path must not be empty".into());
    }

    let requested = Path::new(value);
    let name = requested
        .file_name()
        .filter(|name| !name.is_empty())
        .ok_or("Download path must name a file")?;
    let parent = requested
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent = fs::canonicalize(parent)
        .map_err(|error| format!("cannot open download folder: {error}"))?;
    let destination = parent.join(name);
    match fs::symlink_metadata(&destination) {
        Ok(_) => return Err("Download path already exists".into()),
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            return Err(format!("cannot check download path: {error}"));
        }
        Err(_) => {}
    }
    Ok(destination)
}

fn path_string(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_string)
        .ok_or("File path must be valid UTF-8".into())
}
