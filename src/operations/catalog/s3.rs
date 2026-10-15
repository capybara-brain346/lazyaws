use serde_json::json;

use super::*;

#[cfg(test)]
#[path = "s3_tests.rs"]
mod tests;

pub(super) fn available(
    panel: usize,
    resource: &Resource,
    primary: &Resource,
    snapshot: &Snapshot,
) -> Vec<Operation> {
    let Some(region) = bucket_region(primary, snapshot) else {
        return Vec::new();
    };
    let bucket = primary.id.as_str();
    if bucket.is_empty() {
        return Vec::new();
    }

    match panel {
        0 => {
            let mut actions = Vec::new();
            if let Some(status) = versioning_status(snapshot, primary) {
                let (label, status) = if status.eq_ignore_ascii_case("enabled") {
                    ("Suspend bucket versioning", "Suspended")
                } else {
                    ("Enable bucket versioning", "Enabled")
                };
                actions.push(s3_operation(
                    label,
                    "put-bucket-versioning",
                    json!({"Bucket":bucket,"VersioningConfiguration":{"Status":status}}),
                    bucket,
                    "Changes versioning for new objects in this bucket.",
                    None,
                    &region,
                ));
            }
            actions.extend([
                s3_operation(
                    "Delete empty bucket",
                    "delete-bucket",
                    json!({"Bucket":bucket}),
                    bucket,
                    "Deletes this bucket. S3 rejects it while it contains objects or versions.",
                    None,
                    &region,
                ),
                s3_operation(
                    "Upload file",
                    "put-object",
                    json!({"Bucket":bucket}),
                    bucket,
                    "Uploads one file up to 5 GB. It replaces an object with the same key.",
                    Some("Upload file path"),
                    &region,
                ),
            ]);
            actions.extend(bucket_tags(bucket, &region));
            actions
        }
        1 if !resource.id.is_empty() => {
            let mut actions = vec![
                s3_operation(
                    "Download object",
                    "get-object",
                    json!({"Bucket":bucket,"Key":resource.id}),
                    &resource.id,
                    "Downloads this object to a new local file.",
                    Some("Download file path"),
                    &region,
                ),
                s3_operation(
                    "Replace object",
                    "put-object",
                    json!({"Bucket":bucket,"Key":resource.id}),
                    &resource.id,
                    "Replaces this object with one local file up to 5 GB.",
                    Some("Upload file path"),
                    &region,
                ),
                s3_operation(
                    "Delete object",
                    "delete-object",
                    json!({"Bucket":bucket,"Key":resource.id}),
                    &resource.id,
                    "Deletes this object. In a versioned bucket, S3 adds a delete marker.",
                    None,
                    &region,
                ),
                s3_operation(
                    "View object metadata",
                    "head-object",
                    json!({"Bucket":bucket,"Key":resource.id}),
                    &resource.id,
                    "Reads metadata for this object.",
                    None,
                    &region,
                ),
            ];
            actions.extend([
                s3_operation(
                    "View object tags",
                    "get-object-tagging",
                    json!({"Bucket":bucket,"Key":resource.id}),
                    &resource.id,
                    "Reads tags for the current object version.",
                    None,
                    &region,
                ),
                s3_operation(
                    "Replace object tags",
                    "put-object-tagging",
                    json!({"Bucket":bucket,"Key":resource.id}),
                    &resource.id,
                    "Replaces all tags for the current object version. Enter Tagging with TagSet.",
                    Some("JSON fields"),
                    &region,
                ),
                s3_operation(
                    "Remove all object tags",
                    "delete-object-tagging",
                    json!({"Bucket":bucket,"Key":resource.id}),
                    &resource.id,
                    "Removes all tags from the current object version.",
                    None,
                    &region,
                ),
                s3_operation(
                    "List versions by key prefix",
                    "list-object-versions",
                    json!({"Bucket":bucket,"Prefix":resource.id}),
                    &resource.id,
                    "Lists versions and delete markers. Other keys with this prefix can appear.",
                    None,
                    &region,
                ),
            ]);
            actions
        }
        2 if !resource.id.is_empty() => [
            ("List prefix objects", "list-objects-v2"),
            ("List prefix versions", "list-object-versions"),
            ("List prefix multipart uploads", "list-multipart-uploads"),
        ]
        .into_iter()
        .map(|(label, name)| {
            s3_operation(
                label,
                name,
                json!({"Bucket":bucket,"Prefix":resource.id}),
                &resource.id,
                "Reads entries with this key prefix. A prefix is not a separate S3 resource.",
                None,
                &region,
            )
        })
        .collect(),
        3 => {
            let mut actions: Vec<_> = [
                ("View bucket versioning", "get-bucket-versioning"),
                ("View bucket encryption", "get-bucket-encryption"),
                ("View public access block", "get-public-access-block"),
                ("View bucket policy", "get-bucket-policy"),
                (
                    "View bucket lifecycle",
                    "get-bucket-lifecycle-configuration",
                ),
                ("View bucket CORS", "get-bucket-cors"),
                ("View bucket logging", "get-bucket-logging"),
                (
                    "View bucket ownership controls",
                    "get-bucket-ownership-controls",
                ),
                ("View bucket replication", "get-bucket-replication"),
                (
                    "View bucket notifications",
                    "get-bucket-notification-configuration",
                ),
            ]
            .into_iter()
            .map(|(label, name)| {
                s3_operation(
                    label,
                    name,
                    json!({"Bucket":bucket}),
                    bucket,
                    "Reads this bucket setting. AWS can return an error if it is not set.",
                    None,
                    &region,
                )
            })
            .collect();
            actions.extend(bucket_tags(bucket, &region));
            for (label, name, warning) in [
                (
                    "Set bucket encryption",
                    "put-bucket-encryption",
                    "Changes encryption for new objects. Enter ServerSideEncryptionConfiguration. KMS charges can apply.",
                ),
                (
                    "Set bucket lifecycle",
                    "put-bucket-lifecycle-configuration",
                    "Replaces all lifecycle rules. Enter LifecycleConfiguration. Rules can delete data or add storage charges.",
                ),
                (
                    "Set bucket CORS",
                    "put-bucket-cors",
                    "Replaces all CORS rules. Enter CORSConfiguration. This changes browser access to the bucket.",
                ),
            ] {
                actions.push(s3_operation(
                    label,
                    name,
                    json!({"Bucket":bucket}),
                    bucket,
                    warning,
                    Some("JSON fields"),
                    &region,
                ));
            }
            if resource.id == "versioning"
                && let Some(status) = versioning_status(snapshot, primary)
            {
                let (label, status) = if status.eq_ignore_ascii_case("enabled") {
                    ("Suspend bucket versioning", "Suspended")
                } else {
                    ("Enable bucket versioning", "Enabled")
                };
                actions.push(s3_operation(
                    label,
                    "put-bucket-versioning",
                    json!({"Bucket":bucket,"VersioningConfiguration":{"Status":status}}),
                    bucket,
                    "Changes versioning for new objects in this bucket.",
                    None,
                    &region,
                ));
            }
            actions
        }
        _ => Vec::new(),
    }
}

fn bucket_tags(bucket: &str, region: &str) -> Vec<Operation> {
    vec![
        s3_operation(
            "View bucket tags",
            "get-bucket-tagging",
            json!({"Bucket":bucket}),
            bucket,
            "Reads bucket tags. AWS can return an error if no tags are set.",
            None,
            region,
        ),
        s3_operation(
            "Replace bucket tags",
            "put-bucket-tagging",
            json!({"Bucket":bucket}),
            bucket,
            "Replaces all bucket tags. Enter Tagging with TagSet.",
            Some("JSON fields"),
            region,
        ),
    ]
}

fn s3_operation(
    label: &'static str,
    name: &'static str,
    input: Value,
    target: &str,
    warning: &'static str,
    parameter: Option<&'static str>,
    region: &str,
) -> Operation {
    let mut operation = operation(label, "s3api", name, input, target, warning, parameter);
    operation.region = Some(region.into());
    operation
}

fn bucket_region(primary: &Resource, snapshot: &Snapshot) -> Option<String> {
    string(&primary.detail, &["BucketRegion", "region"])
        .or_else(|| {
            snapshot.panels[3]
                .items
                .iter()
                .find(|item| item.id == "location")
                .and_then(|item| string(&item.detail, &["region"]))
        })
        .filter(|region| !region.is_empty())
        .map(str::to_string)
}

fn versioning_status(snapshot: &Snapshot, primary: &Resource) -> Option<String> {
    string(&primary.detail, &["versioning"])
        .map(str::to_string)
        .or_else(|| {
            snapshot.panels[3]
                .items
                .iter()
                .find(|item| item.id == "versioning")
                .map(|item| {
                    string(&item.detail, &["Status", "status"])
                        .unwrap_or("Disabled")
                        .to_string()
                })
        })
}
