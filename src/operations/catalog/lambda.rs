use serde_json::json;

use super::*;

#[cfg(test)]
#[path = "lambda_tests.rs"]
mod tests;

pub(super) fn available(panel: usize, resource: &Resource, primary: &Resource) -> Vec<Operation> {
    let id = resource.id.as_str();
    if id.is_empty() {
        return Vec::new();
    }
    let function_arn = primary.id.as_str();
    if function_arn.is_empty() {
        return Vec::new();
    }
    let function_name = string(&primary.detail, &["FunctionName"]).unwrap_or(function_arn);

    match panel {
        0 => function_actions(id, function_arn),
        1 => version_actions(resource, primary, function_arn, function_name),
        2 => alias_actions(resource, function_arn, function_name),
        3 => tag_actions(resource, function_arn),
        _ => Vec::new(),
    }
}

fn function_actions(id: &str, function_arn: &str) -> Vec<Operation> {
    vec![
        operation(
            "Invoke function",
            "lambda",
            "invoke",
            json!({"FunctionName": id}),
            id,
            "Invokes the function synchronously and shows its response payload in Details. An empty input sends no payload.",
            Some("Invoke payload"),
        ),
        operation(
            "Invoke with log tail",
            "lambda",
            "invoke",
            json!({"FunctionName": id}),
            id,
            "Invokes synchronously and returns the last 4 KB of logs decoded in Details. An empty input sends no payload.",
            Some("Invoke payload with logs"),
        ),
        operation(
            "Invoke asynchronously",
            "lambda",
            "invoke",
            json!({"FunctionName": id}),
            id,
            "Invokes without waiting for a response. Use this for Event invocations up to 1 MB of payload.",
            Some("Invoke payload async"),
        ),
        operation(
            "Get function",
            "lambda",
            "get-function",
            json!({"FunctionName": id}),
            id,
            "Reads configuration, code location, and tags for this function.",
            None,
        ),
        operation(
            "Get function configuration",
            "lambda",
            "get-function-configuration",
            json!({"FunctionName": id}),
            id,
            "Reads runtime, memory, timeout, environment, and VPC settings.",
            None,
        ),
        operation(
            "Get policy",
            "lambda",
            "get-policy",
            json!({"FunctionName": id}),
            id,
            "Reads the resource policy. AWS returns an error when no policy is set.",
            None,
        ),
        operation(
            "Get reserved concurrency",
            "lambda",
            "get-function-concurrency",
            json!({"FunctionName": id}),
            id,
            "Reads the reserved concurrency limit. An empty response means no limit is set.",
            None,
        ),
        operation(
            "Get runtime management",
            "lambda",
            "get-runtime-management-config",
            json!({"FunctionName": id}),
            id,
            "Reads runtime update mode and ARN for this function.",
            None,
        ),
        operation(
            "Get recursion config",
            "lambda",
            "get-function-recursion-config",
            json!({"FunctionName": id}),
            id,
            "Reads the recursive-loop detection setting for this function.",
            None,
        ),
        operation(
            "List versions",
            "lambda",
            "list-versions-by-function",
            json!({"FunctionName": id}),
            id,
            "Lists published versions plus $LATEST for this function.",
            None,
        ),
        operation(
            "List aliases",
            "lambda",
            "list-aliases",
            json!({"FunctionName": id}),
            id,
            "Lists aliases that point at this function's versions.",
            None,
        ),
        operation(
            "List event source mappings",
            "lambda",
            "list-event-source-mappings",
            json!({"FunctionName": id}),
            id,
            "Lists stream and queue mappings that invoke this function.",
            None,
        ),
        operation(
            "List async invoke configs",
            "lambda",
            "list-function-event-invoke-configs",
            json!({"FunctionName": id}),
            id,
            "Lists retry and destination settings for async invocations.",
            None,
        ),
        operation(
            "List URL configs",
            "lambda",
            "list-function-url-configs",
            json!({"FunctionName": id}),
            id,
            "Lists function URL endpoints for this function.",
            None,
        ),
        operation(
            "Publish version",
            "lambda",
            "publish-version",
            json!({"FunctionName": id}),
            id,
            "Publishes a new immutable version from $LATEST code and configuration.",
            Some("JSON fields"),
        ),
        operation(
            "Update configuration",
            "lambda",
            "update-function-configuration",
            json!({"FunctionName": id}),
            id,
            "Updates settings such as MemorySize, Timeout, or Environment. Enter only changed fields; the function restarts.",
            Some("JSON fields"),
        ),
        operation(
            "Set reserved concurrency",
            "lambda",
            "put-function-concurrency",
            json!({"FunctionName": id}),
            id,
            "Limits simultaneous executions. Zero blocks all invocations; use Remove to clear the limit.",
            Some("Reserved concurrency"),
        ),
        operation(
            "Remove reserved concurrency",
            "lambda",
            "delete-function-concurrency",
            json!({"FunctionName": id}),
            id,
            "Removes the reserved limit so the function shares account concurrency.",
            None,
        ),
        operation(
            "Create alias",
            "lambda",
            "create-alias",
            json!({"FunctionName": id}),
            id,
            "Creates an alias. Enter Name and FunctionVersion, for example {\"Name\":\"prod\",\"FunctionVersion\":\"3\"}.",
            Some("JSON fields"),
        ),
        operation(
            "Add function tags",
            "lambda",
            "tag-resource",
            json!({"Resource": function_arn}),
            id,
            "Adds tags. Enter a Tags object such as {\"Tags\":{\"Team\":\"platform\"}}. Matching keys are replaced.",
            Some("JSON fields"),
        ),
        operation(
            "Delete function",
            "lambda",
            "delete-function",
            json!({"FunctionName": id}),
            id,
            "Permanently deletes this function and its versions. Event mappings must be removed first.",
            None,
        ),
    ]
}

fn version_actions(
    resource: &Resource,
    primary: &Resource,
    _function_arn: &str,
    function_name: &str,
) -> Vec<Operation> {
    let version_arn = resource.id.as_str();
    let version = string(&resource.detail, &["Version"]).unwrap_or("");
    let is_latest = version == "$LATEST" || version_arn.ends_with(":$LATEST");
    let qualifier = if version.is_empty() {
        version_arn.to_string()
    } else {
        version.to_string()
    };
    let mut actions = vec![
        operation(
            "Invoke version",
            "lambda",
            "invoke",
            json!({"FunctionName": version_arn}),
            version_arn,
            "Invokes this published version synchronously. An empty input sends no payload.",
            Some("Invoke payload"),
        ),
        operation(
            "Get version",
            "lambda",
            "get-function",
            json!({"FunctionName": version_arn}),
            version_arn,
            "Reads configuration and code for this version.",
            None,
        ),
        operation(
            "Get version configuration",
            "lambda",
            "get-function-configuration",
            json!({"FunctionName": version_arn}),
            version_arn,
            "Reads runtime, memory, and timeout for this version.",
            None,
        ),
    ];
    if !is_latest {
        actions.extend([
            operation(
                "Get provisioned concurrency",
                "lambda",
                "get-provisioned-concurrency-config",
                json!({"FunctionName": function_name, "Qualifier": qualifier}),
                version_arn,
                "Reads provisioned capacity for this version. AWS returns an error when none is set.",
                None,
            ),
            operation(
                "Set provisioned concurrency",
                "lambda",
                "put-provisioned-concurrency-config",
                json!({"FunctionName": function_name, "Qualifier": qualifier}),
                version_arn,
                "Keeps execution environments warm. Provisioned capacity adds charges even without invocations.",
                Some("Provisioned concurrency"),
            ),
            operation(
                "Remove provisioned concurrency",
                "lambda",
                "delete-provisioned-concurrency-config",
                json!({"FunctionName": function_name, "Qualifier": qualifier}),
                version_arn,
                "Removes provisioned capacity for this version.",
                None,
            ),
            operation(
                "Delete version",
                "lambda",
                "delete-function",
                json!({"FunctionName": primary.id, "Qualifier": qualifier}),
                version_arn,
                "Permanently deletes this version. Remove aliases that point at it first.",
                None,
            ),
        ]);
    } else {
        let _ = primary;
    }
    actions
}

fn alias_actions(resource: &Resource, _function_arn: &str, function_name: &str) -> Vec<Operation> {
    let alias_arn = resource.id.as_str();
    let alias = string(&resource.detail, &["Name"]).unwrap_or(&resource.label);
    vec![
        operation(
            "Invoke alias",
            "lambda",
            "invoke",
            json!({"FunctionName": alias_arn}),
            alias_arn,
            "Invokes the version this alias points at. An empty input sends no payload.",
            Some("Invoke payload"),
        ),
        operation(
            "Get alias",
            "lambda",
            "get-alias",
            json!({"FunctionName": function_name, "Name": alias}),
            alias_arn,
            "Reads routing configuration for this alias.",
            None,
        ),
        operation(
            "Move alias to version",
            "lambda",
            "update-alias",
            json!({"FunctionName": function_name, "Name": alias}),
            alias_arn,
            "Points this alias at another version. Enter a version number or $LATEST.",
            Some("Alias version"),
        ),
        operation(
            "Get provisioned concurrency",
            "lambda",
            "get-provisioned-concurrency-config",
            json!({"FunctionName": function_name, "Qualifier": alias}),
            alias_arn,
            "Reads provisioned capacity for this alias. AWS returns an error when none is set.",
            None,
        ),
        operation(
            "Set provisioned concurrency",
            "lambda",
            "put-provisioned-concurrency-config",
            json!({"FunctionName": function_name, "Qualifier": alias}),
            alias_arn,
            "Keeps execution environments warm. Provisioned capacity adds charges even without invocations.",
            Some("Provisioned concurrency"),
        ),
        operation(
            "Delete alias",
            "lambda",
            "delete-alias",
            json!({"FunctionName": function_name, "Name": alias}),
            alias_arn,
            "Deletes this alias. The underlying version is kept.",
            None,
        ),
    ]
}

fn tag_actions(resource: &Resource, function_arn: &str) -> Vec<Operation> {
    let key = string(&resource.detail, &["Key", "key"]).unwrap_or(&resource.id);
    vec![
        operation(
            "Remove function tag",
            "lambda",
            "untag-resource",
            json!({"Resource": function_arn, "TagKeys": [key]}),
            resource.id.as_str(),
            "Removes this tag from the function.",
            None,
        ),
        operation(
            "Update function tag value",
            "lambda",
            "tag-resource",
            json!({"Resource": function_arn, "Tags": {key: ""}}),
            resource.id.as_str(),
            "Replaces this tag's value. An empty value is allowed.",
            Some("Lambda tag value"),
        ),
        operation(
            "Add function tags",
            "lambda",
            "tag-resource",
            json!({"Resource": function_arn}),
            resource.id.as_str(),
            "Adds tags. Enter a Tags object such as {\"Tags\":{\"Team\":\"platform\"}}.",
            Some("JSON fields"),
        ),
    ]
}
