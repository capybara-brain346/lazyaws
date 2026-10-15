use std::fs;

use crate::operations::Operation;

use super::{FAKE_CLI_LOCK, FakeCli};

#[test]
fn execute_accepts_empty_success_output() {
    let _guard = FAKE_CLI_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let fake = FakeCli::new("exit 0");
    let operation = Operation {
        label: "Delete volume",
        service: "ec2",
        name: "delete-volume",
        input: serde_json::json!({"VolumeId":"vol-1"}),
        region: None,
        target: "vol-1".into(),
        warning: "Deletes the volume.",
        parameter: None,
    };
    let mut logs = Vec::new();

    assert_eq!(
        fake.client().execute(&operation, "", &mut logs).unwrap(),
        serde_json::json!({})
    );
    assert!(logs[0].contains("[ok]"));
}

#[test]
fn identity_requires_account_arn_and_user_id() {
    let _guard = FAKE_CLI_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let fake = FakeCli::new(
        "printf '%s\\n' '{\"Account\":\"123456789012\",\"Arn\":\"arn:aws:iam::123456789012:user/test\",\"UserId\":\"AIDATEST\"}'",
    );
    let mut logs = Vec::new();
    let identity = fake.client().identity(&mut logs).unwrap();
    assert_eq!(identity.account, "123456789012");
    assert_eq!(identity.user_id, "AIDATEST");

    let fake =
        FakeCli::new("printf '%s\\n' '{\"Account\":\"\",\"Arn\":\"arn:test\",\"UserId\":\"u\"}'");
    let error = fake.client().identity(&mut logs).unwrap_err();
    assert!(error.contains("Account"));
}

#[test]
fn ecr_batch_delete_reports_per_image_failures() {
    let _guard = FAKE_CLI_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let fake = FakeCli::new(
        "printf '%s\\n' '{\"imageIds\":[],\"failures\":[{\"imageId\":{\"imageTag\":\"latest\"},\"failureCode\":\"ImageNotFound\",\"failureReason\":\"missing\"}]}'",
    );
    let operation = Operation {
        label: "Remove image tag",
        service: "ecr",
        name: "batch-delete-image",
        input: serde_json::json!({"repositoryName":"team/app","imageIds":[{"imageTag":"latest"}]}),
        region: None,
        target: "latest".into(),
        warning: "Removes this image tag.",
        parameter: None,
    };
    let mut logs = Vec::new();

    let error = fake
        .client()
        .execute(&operation, "", &mut logs)
        .unwrap_err();
    assert!(error.contains("latest: ImageNotFound: missing"), "{error}");
    assert!(logs[0].contains("failed"));
}

#[test]
fn ecr_batch_delete_rejects_malformed_failures() {
    let _guard = FAKE_CLI_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let fake = FakeCli::new("printf '%s\\n' '{\"failures\":null}'");
    let operation = Operation {
        label: "Remove image tag",
        service: "ecr",
        name: "batch-delete-image",
        input: serde_json::json!({"repositoryName":"team/app","imageIds":[{"imageTag":"latest"}]}),
        region: None,
        target: "latest".into(),
        warning: "Removes this image tag.",
        parameter: None,
    };

    let error = fake
        .client()
        .execute(&operation, "", &mut Vec::new())
        .unwrap_err();
    assert!(error.contains("`failures` must be an array"), "{error}");
}

#[test]
fn download_stages_output_and_never_overwrites_a_local_file() {
    let _guard = FAKE_CLI_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let fake = FakeCli::new(
        "for last do :; done\nprintf downloaded > \"$last\"\nprintf '%s\\n' '{\"ETag\":\"etag\"}'",
    );
    let destination = fake.dir.join("object.txt");
    let operation = Operation {
        label: "Download object",
        service: "s3api",
        name: "get-object",
        input: serde_json::json!({"Bucket":"bucket","Key":"folder/object.txt"}),
        region: Some("ap-south-1".into()),
        target: "folder/object.txt".into(),
        warning: "Downloads this object.",
        parameter: Some("Download file path"),
    };
    let mut logs = Vec::new();

    fake.client()
        .execute(&operation, destination.to_str().unwrap(), &mut logs)
        .unwrap();
    assert_eq!(fs::read_to_string(&destination).unwrap(), "downloaded");
    assert!(fs::read_dir(&fake.dir).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".lazyaws-download-")
    }));

    let existing = fake.dir.join("existing.txt");
    fs::write(&existing, "keep").unwrap();
    let error = fake
        .client()
        .execute(&operation, existing.to_str().unwrap(), &mut logs)
        .unwrap_err();
    assert!(error.contains("already exists"));
    assert_eq!(fs::read_to_string(existing).unwrap(), "keep");
}

#[test]
fn failed_download_removes_partial_staging_output() {
    let _guard = FAKE_CLI_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let fake =
        FakeCli::new("for last do :; done\nprintf partial > \"$last\"\necho denied >&2\nexit 1");
    let destination = fake.dir.join("object.txt");
    let operation = Operation {
        label: "Download object",
        service: "s3api",
        name: "get-object",
        input: serde_json::json!({"Bucket":"bucket","Key":"object.txt"}),
        region: Some("ap-south-1".into()),
        target: "object.txt".into(),
        warning: "Downloads this object.",
        parameter: Some("Download file path"),
    };
    let error = fake
        .client()
        .execute(&operation, destination.to_str().unwrap(), &mut Vec::new())
        .unwrap_err();

    assert!(error.contains("denied"));
    assert!(!destination.exists());
    assert!(fs::read_dir(&fake.dir).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".lazyaws-download-")
    }));
}
