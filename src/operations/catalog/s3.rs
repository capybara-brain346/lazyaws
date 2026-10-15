use serde_json::json;

use super::*;

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
            actions
        }
        1 => vec![
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
        ],
        _ => Vec::new(),
    }
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
