use serde_json::json;

use super::*;

#[cfg(test)]
#[path = "ecr_tests.rs"]
mod tests;

pub(super) fn available(panel: usize, resource: &Resource, primary: &Resource) -> Vec<Operation> {
    let id = resource.id.as_str();
    if id.is_empty() {
        return Vec::new();
    }
    let repository = string(&primary.detail, &["repositoryName"]).unwrap_or(&primary.id);

    match panel {
        0 => {
            let mut actions = vec![operation(
                "Delete repository",
                "ecr",
                "delete-repository",
                json!({"repositoryName":repository,"force":false}),
                id,
                "Deletes this repository only when it is empty.",
                None,
            )];
            let mutability = string(&resource.detail, &["imageTagMutability"]);
            let exclusion_filters = resource
                .detail
                .get("imageTagMutabilityExclusionFilters")
                .and_then(Value::as_array);
            let mutability_target = match mutability.map(str::to_ascii_uppercase).as_deref() {
                Some("IMMUTABLE") => Some(("Make tags mutable", "MUTABLE", false)),
                Some("MUTABLE") => Some(("Make tags immutable", "IMMUTABLE", false)),
                Some("IMMUTABLE_WITH_EXCLUSION")
                    if exclusion_filters.is_some_and(|filters| !filters.is_empty()) =>
                {
                    Some((
                        "Make tags mutable with exclusions",
                        "MUTABLE_WITH_EXCLUSION",
                        true,
                    ))
                }
                Some("MUTABLE_WITH_EXCLUSION")
                    if exclusion_filters.is_some_and(|filters| !filters.is_empty()) =>
                {
                    Some((
                        "Make tags immutable with exclusions",
                        "IMMUTABLE_WITH_EXCLUSION",
                        true,
                    ))
                }
                _ => None,
            };
            if let Some((label, value, preserve_filters)) = mutability_target {
                let mut input = json!({"repositoryName":repository,"imageTagMutability":value});
                if preserve_filters {
                    input["imageTagMutabilityExclusionFilters"] =
                        json!(exclusion_filters.expect("checked above"));
                }
                actions.push(operation(
                    label,
                    "ecr",
                    "put-image-tag-mutability",
                    input,
                    id,
                    "Changes tag overwrite rules for new image pushes.",
                    None,
                ));
            }

            actions.push(operation(
                "Get repository policy",
                "ecr",
                "get-repository-policy",
                json!({"repositoryName":repository}),
                id,
                "Reads this repository's access policy.",
                None,
            ));
            actions.push(operation(
                "Set repository policy",
                "ecr",
                "set-repository-policy",
                json!({"repositoryName":repository,"force":false}),
                id,
                "Sets who can access this repository. AWS checks access before it accepts the policy.",
                Some(Parameter::JsonFields),
            ));
            actions.push(operation(
                "Delete repository policy",
                "ecr",
                "delete-repository-policy",
                json!({"repositoryName":repository}),
                id,
                "Removes this repository's access policy.",
                None,
            ));
            actions.push(operation(
                "Get lifecycle policy",
                "ecr",
                "get-lifecycle-policy",
                json!({"repositoryName":repository}),
                id,
                "Reads this repository's image lifecycle policy.",
                None,
            ));
            actions.push(operation(
                "Set lifecycle policy",
                "ecr",
                "put-lifecycle-policy",
                json!({"repositoryName":repository}),
                id,
                "Sets image expiry rules. Check which images the policy can expire.",
                Some(Parameter::JsonFields),
            ));
            actions.push(operation(
                "Delete lifecycle policy",
                "ecr",
                "delete-lifecycle-policy",
                json!({"repositoryName":repository}),
                id,
                "Stops this repository's automatic image lifecycle rules.",
                None,
            ));

            if let Some(scan_on_push) = resource
                .detail
                .pointer("/imageScanningConfiguration/scanOnPush")
                .and_then(Value::as_bool)
            {
                let (label, value, warning) = if scan_on_push {
                    (
                        "Disable scan on push",
                        false,
                        "Disables the deprecated repository scan setting. Registry-level scanning is preferred.",
                    )
                } else {
                    (
                        "Enable scan on push",
                        true,
                        "Enables the deprecated repository scan setting. Registry-level scanning is preferred.",
                    )
                };
                actions.push(operation(
                    label,
                    "ecr",
                    "put-image-scanning-configuration",
                    json!({"repositoryName":repository,"imageScanningConfiguration":{"scanOnPush":value}}),
                    id,
                    warning,
                    None,
                ));
            }

            if let Some(arn) = string(&resource.detail, &["repositoryArn"]) {
                actions.push(operation(
                    "Add repository tags",
                    "ecr",
                    "tag-resource",
                    json!({"resourceArn":arn}),
                    id,
                    "Enter a tags list with Key and Value. Existing matching tag keys are replaced.",
                    Some(Parameter::JsonFields),
                ));
            }

            actions
        }
        1 => {
            let mut actions = vec![
                operation(
                    "Scan image",
                    "ecr",
                    "start-image-scan",
                    json!({"repositoryName":repository,"imageId":{"imageDigest":id}}),
                    id,
                    "Starts a basic scan. ECR allows one basic scan per image each day.",
                    None,
                ),
                operation(
                    "View image scan findings",
                    "ecr",
                    "describe-image-scan-findings",
                    json!({"repositoryName":repository,"imageId":{"imageDigest":id}}),
                    id,
                    "Reads scan findings for this image.",
                    None,
                )
                .paginated(),
                operation(
                    "Delete image by digest",
                    "ecr",
                    "batch-delete-image",
                    json!({"repositoryName":repository,"imageIds":[{"imageDigest":id}]}),
                    id,
                    "Deletes this image and its tags. Workloads that use its digest can fail.",
                    None,
                ),
            ];
            actions.push(
                operation(
                    "Describe image by digest",
                    "ecr",
                    "describe-images",
                    json!({"repositoryName":repository,"imageIds":[{"imageDigest":id}]}),
                    id,
                    "Reads metadata for this image digest.",
                    None,
                )
                .paginated(),
            );
            actions.push(operation(
                "Get image manifest by digest",
                "ecr",
                "batch-get-image",
                json!({"repositoryName":repository,"imageIds":[{"imageDigest":id}]}),
                id,
                "Reads the image manifest. The response can be large.",
                None,
            ));
            actions
        }
        2 => {
            let tag = string(&resource.detail, &["imageTag", "tag"]).unwrap_or(id);
            let mut actions = vec![operation(
                "Remove image tag",
                "ecr",
                "batch-delete-image",
                json!({"repositoryName":repository,"imageIds":[{"imageTag":tag}]}),
                id,
                "Removes this tag. ECR deletes the image if it has no other tags.",
                None,
            )];
            actions.push(
                operation(
                    "Describe image by tag",
                    "ecr",
                    "describe-images",
                    json!({"repositoryName":repository,"imageIds":[{"imageTag":tag}]}),
                    id,
                    "Reads metadata for this image tag.",
                    None,
                )
                .paginated(),
            );
            actions.push(operation(
                "Get image manifest by tag",
                "ecr",
                "batch-get-image",
                json!({"repositoryName":repository,"imageIds":[{"imageTag":tag}]}),
                id,
                "Reads the image manifest. The response can be large.",
                None,
            ));
            actions
        }
        3 => {
            let key = string(&resource.detail, &["Key", "key"]).unwrap_or(id);
            let arn = string(&primary.detail, &["repositoryArn"]).unwrap_or(&primary.id);
            let mut actions = vec![operation(
                "Remove repository tag",
                "ecr",
                "untag-resource",
                json!({"resourceArn":arn,"tagKeys":[key]}),
                id,
                "Removes this tag from the repository.",
                None,
            )];
            actions.push(operation(
                "Update repository tag value",
                "ecr",
                "tag-resource",
                json!({"resourceArn":arn,"tags":[{"Key":key}]}),
                id,
                "Replaces this tag's value. A blank value is allowed by AWS.",
                Some(Parameter::TagValue),
            ));
            actions.push(operation(
                "Add repository tag",
                "ecr",
                "tag-resource",
                json!({"resourceArn":arn}),
                id,
                "Adds or replaces tags on this repository. Existing matching keys are replaced.",
                Some(Parameter::JsonFields),
            ));
            actions
        }
        _ => Vec::new(),
    }
}
