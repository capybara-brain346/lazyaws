use serde_json::json;

use super::*;

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
            if let Some((label, value)) = string(&resource.detail, &["imageTagMutability"])
                .and_then(|mutability| {
                    if mutability.eq_ignore_ascii_case("immutable") {
                        Some(("Make tags mutable", "MUTABLE"))
                    } else if mutability.eq_ignore_ascii_case("mutable") {
                        Some(("Make tags immutable", "IMMUTABLE"))
                    } else {
                        None
                    }
                })
            {
                actions.push(operation(
                    label,
                    "ecr",
                    "put-image-tag-mutability",
                    json!({"repositoryName":repository,"imageTagMutability":value}),
                    id,
                    "Changes whether new image pushes can reuse a tag.",
                    None,
                ));
            }
            actions
        }
        1 => vec![
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
            ),
            operation(
                "Delete image by digest",
                "ecr",
                "batch-delete-image",
                json!({"repositoryName":repository,"imageIds":[{"imageDigest":id}]}),
                id,
                "Deletes this image and its tags. Workloads that use its digest can fail.",
                None,
            ),
        ],
        2 => {
            let tag = string(&resource.detail, &["imageTag", "tag"]).unwrap_or(id);
            vec![operation(
                "Remove image tag",
                "ecr",
                "batch-delete-image",
                json!({"repositoryName":repository,"imageIds":[{"imageTag":tag}]}),
                id,
                "Removes this tag. ECR deletes the image if it has no other tags.",
                None,
            )]
        }
        3 => {
            let key = string(&resource.detail, &["Key", "key"]).unwrap_or(id);
            let arn = string(&primary.detail, &["repositoryArn"]).unwrap_or(&primary.id);
            vec![operation(
                "Remove repository tag",
                "ecr",
                "untag-resource",
                json!({"resourceArn":arn,"tagKeys":[key]}),
                id,
                "Removes this tag from the repository.",
                None,
            )]
        }
        _ => Vec::new(),
    }
}
