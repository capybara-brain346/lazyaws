mod catalog;
pub use catalog::available;

#[cfg(test)]
mod tests;

use std::{fs, path::Path};

use serde_json::{Value, json};

use crate::config::{MAX_ITEMS, S3_MAX_UPLOAD_BYTES};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountContext {
    pub account: String,
    pub arn: String,
    pub user_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operation {
    pub label: &'static str,
    pub service: &'static str,
    pub name: &'static str,
    pub input: Value,
    pub region: Option<String>,
    pub target: String,
    pub warning: &'static str,
    pub parameter: Option<&'static str>,
}

impl Operation {
    pub fn args(&self, value: &str) -> Result<Vec<String>, String> {
        let mut input = self.input.clone();
        let mut extra = Vec::new();

        match self.parameter {
            Some("JSON fields") => {
                let fields: serde_json::Map<String, Value> = serde_json::from_str(value)
                    .map_err(|error| format!("Enter a JSON object: {error}"))?;
                if fields.is_empty() {
                    return Err("Enter a non-empty JSON object".into());
                }
                let input = input
                    .as_object_mut()
                    .ok_or("Operation input must be a JSON object")?;
                for key in fields.keys() {
                    if input.contains_key(key) || key == "registryId" {
                        return Err(format!("Cannot replace the captured field `{key}`"));
                    }
                }
                input.extend(fields);
            }
            Some("Tag value") => {
                let field = match (self.service, self.name) {
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
            }
            Some("Lambda tag value") => {
                if self.service != "lambda" || self.name != "tag-resource" {
                    return Err("Lambda tag value is only valid for Lambda tags".into());
                }
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
            }
            Some("Invoke payload" | "Invoke payload with logs" | "Invoke payload async") => {
                if self.service != "lambda" || self.name != "invoke" {
                    return Err("Invoke payload is only valid for Lambda invocations".into());
                }
                let function = input
                    .get("FunctionName")
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
                    .ok_or("Invoke input must contain a function name")?
                    .to_string();
                let payload = value.trim();
                let limit = if self.parameter == Some("Invoke payload async") {
                    1024 * 1024
                } else {
                    6 * 1024 * 1024
                };
                if !payload.is_empty() {
                    serde_json::from_str::<Value>(payload).map_err(|error| {
                        format!("Enter valid JSON payload or leave empty: {error}")
                    })?;
                    if payload.len() > limit {
                        return Err("Invoke payload exceeds the Lambda size limit".into());
                    }
                }
                let mut args: Vec<String> = vec![
                    self.service.to_string(),
                    self.name.to_string(),
                    "--function-name".into(),
                    function,
                ];
                if !payload.is_empty() {
                    args.extend(["--payload".into(), payload.to_string()]);
                }
                if self.parameter == Some("Invoke payload with logs") {
                    args.extend(["--log-type".into(), "Tail".into()]);
                }
                if self.parameter == Some("Invoke payload async") {
                    args.extend(["--invocation-type".into(), "Event".into()]);
                }
                args.push(crate::aws::INVOKE_OUTPUT_PLACEHOLDER.into());
                return Ok(args);
            }
            Some("Reserved concurrency") => {
                if self.service != "lambda" || self.name != "put-function-concurrency" {
                    return Err(
                        "Reserved concurrency is only valid for Lambda concurrency limits".into(),
                    );
                }
                let count = value.trim().parse::<i32>().map_err(|_| {
                    "Reserved concurrency must be a non-negative integer".to_string()
                })?;
                if count < 0 {
                    return Err("Reserved concurrency must be a non-negative integer".into());
                }
                input
                    .as_object_mut()
                    .ok_or("Operation input must be a JSON object")?
                    .insert("ReservedConcurrentExecutions".into(), json!(count));
            }
            Some("Provisioned concurrency") => {
                if self.service != "lambda" || self.name != "put-provisioned-concurrency-config" {
                    return Err(
                        "Provisioned concurrency is only valid for Lambda versions or aliases"
                            .into(),
                    );
                }
                let count = value.trim().parse::<i32>().map_err(|_| {
                    "Provisioned concurrency must be a positive integer".to_string()
                })?;
                if count < 1 {
                    return Err("Provisioned concurrency must be a positive integer".into());
                }
                input
                    .as_object_mut()
                    .ok_or("Operation input must be a JSON object")?
                    .insert("ProvisionedConcurrentExecutions".into(), json!(count));
            }
            Some("Alias version") => {
                if self.service != "lambda" || self.name != "update-alias" {
                    return Err("Alias version is only valid for Lambda alias updates".into());
                }
                let version = value.trim();
                let valid = version == "$LATEST"
                    || version.parse::<u32>().is_ok_and(|revision| revision > 0);
                if !valid {
                    return Err("Enter a published version number or $LATEST".into());
                }
                input
                    .as_object_mut()
                    .ok_or("Operation input must be a JSON object")?
                    .insert("FunctionVersion".into(), json!(version));
            }
            Some("Task definition revision") => {
                if self.service != "ecs" || self.name != "update-service" {
                    return Err(
                        "Task definition revision is only valid for ECS service updates".into(),
                    );
                }
                let definition = value.trim();
                let family_revision = if definition.starts_with("arn:") {
                    let (context, revision) = definition
                        .split_once(":task-definition/")
                        .ok_or("Enter a complete ECS task definition ARN")?;
                    let selected_context = self
                        .target
                        .split_once(":service/")
                        .map(|(context, _)| context)
                        .ok_or(
                            "Use family:revision when the selected service ARN is unavailable",
                        )?;
                    if context != selected_context || context.split(':').nth(2) != Some("ecs") {
                        return Err("Task definition ARN must match the selected service account and region".into());
                    }
                    revision
                } else {
                    definition
                };
                let valid = family_revision
                    .split_once(':')
                    .is_some_and(|(family, revision)| {
                        !family.is_empty()
                            && family
                                .chars()
                                .all(|c| c.is_ascii_alphanumeric() || "-_".contains(c))
                            && revision.parse::<u32>().is_ok_and(|revision| revision > 0)
                    });
                if !valid || (definition.starts_with("arn:") && family_revision == definition) {
                    return Err(
                        "Enter a task definition family:revision or ARN with a positive revision"
                            .into(),
                    );
                }
                input
                    .as_object_mut()
                    .ok_or("Operation input must be a JSON object")?
                    .insert("taskDefinition".into(), json!(definition));
            }
            Some("Service deployment ARN") => {
                if self.service != "ecs" || self.name != "stop-service-deployment" {
                    return Err("Service deployment ARN is only valid for ECS rollback".into());
                }
                let deployment = value.trim();
                let (account, service) = self
                    .target
                    .split_once(":service/")
                    .filter(|(account, service)| {
                        account.starts_with("arn:") && service.split('/').count() == 2
                    })
                    .ok_or("The selected service must have a cluster-qualified ARN")?;
                let prefix = format!("{account}:service-deployment/{service}/");
                if !deployment.strip_prefix(&prefix).is_some_and(|id| {
                    !id.is_empty()
                        && id
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || "-_".contains(c))
                }) {
                    return Err(
                        "Deployment ARN must belong to the selected service, account, and region"
                            .into(),
                    );
                }
                input
                    .as_object_mut()
                    .ok_or("Operation input must be a JSON object")?
                    .insert("serviceDeploymentArn".into(), json!(deployment));
            }
            Some("Desired task count") => {
                if self.service != "ecs" || self.name != "update-service" {
                    return Err("Desired task count is only valid for ECS service updates".into());
                }
                let count = value
                    .trim()
                    .parse::<i32>()
                    .map_err(|_| "Desired task count must be a non-negative integer".to_string())?;
                if count < 0 {
                    return Err("Desired task count must be a non-negative integer".into());
                }
                input
                    .as_object_mut()
                    .ok_or("Operation input must be a JSON object")?
                    .insert("desiredCount".into(), json!(count));
            }
            Some("Upload file path") => {
                if self.service != "s3api" || self.name != "put-object" {
                    return Err("Upload file path is only valid for S3 object uploads".into());
                }
                let path = upload_path(value)?;
                if !input.get("Key").is_some_and(|key| key.as_str().is_some()) {
                    let name = path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .filter(|name| !name.is_empty())
                        .ok_or("Upload file name must be valid UTF-8")?;
                    input
                        .as_object_mut()
                        .ok_or("Operation input must be a JSON object")?
                        .insert("Key".into(), Value::String(name.into()));
                }
                extra.extend(["--body".into(), path_string(&path)?]);
            }
            Some("Download file path") => {
                if self.service != "s3api" || self.name != "get-object" {
                    return Err("Download file path is only valid for S3 object downloads".into());
                }
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
                return Ok(vec![
                    self.service.into(),
                    self.name.into(),
                    format!("--bucket={bucket}"),
                    format!("--key={key}"),
                    path_string(&path)?,
                ]);
            }
            Some(_) => return Err("Unknown operation parameter".into()),
            None => {}
        }

        let input = serde_json::to_string(&input)
            .map_err(|error| format!("cannot serialize AWS input: {error}"))?;
        let mut args = vec![
            self.service.into(),
            self.name.into(),
            "--cli-input-json".into(),
            input,
        ];
        args.extend(extra);
        if matches!(
            (self.service, self.name),
            (
                "ecs",
                "list-services"
                    | "list-tasks"
                    | "list-task-definitions"
                    | "list-container-instances"
            ) | ("ecr", "describe-images" | "describe-image-scan-findings")
                | (
                    "ec2",
                    "describe-instance-status"
                        | "describe-volume-status"
                        | "describe-volumes-modifications"
                        | "describe-security-group-rules"
                        | "describe-tags"
                )
                | (
                    "s3api",
                    "list-objects-v2" | "list-object-versions" | "list-multipart-uploads"
                )
                | (
                    "lambda",
                    "list-functions"
                        | "list-versions-by-function"
                        | "list-aliases"
                        | "list-event-source-mappings"
                        | "list-function-event-invoke-configs"
                        | "list-function-url-configs"
                        | "list-provisioned-concurrency-configs"
                )
        ) {
            args.extend(["--max-items".into(), MAX_ITEMS.to_string()]);
        }
        Ok(args)
    }
}

fn upload_path(value: &str) -> Result<std::path::PathBuf, String> {
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

fn download_path(value: &str) -> Result<std::path::PathBuf, String> {
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
