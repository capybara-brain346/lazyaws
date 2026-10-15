use std::{fs, fs::File, path::PathBuf};

use serde_json::json;

use super::*;

fn resource(id: &str, detail: Value) -> Resource {
    Resource {
        id: id.into(),
        label: id.into(),
        summary: String::new(),
        detail,
    }
}

#[test]
fn instance_actions_follow_state_and_keep_captured_ids_in_json() {
    let running = resource("i-test-1", json!({"State":{"Name":"running"}}));
    let actions = available(
        Service::Ec2,
        0,
        &running,
        &running,
        &Snapshot::empty(Service::Ec2),
    );
    assert_eq!(
        actions.iter().map(|action| action.name).collect::<Vec<_>>(),
        ["stop-instances", "reboot-instances", "terminate-instances"]
    );
    assert_eq!(actions[0].target, "i-test-1");
    assert_eq!(actions[0].input["InstanceIds"], json!(["i-test-1"]));
}

#[test]
fn ecs_and_ecr_actions_keep_selected_resource_ids_in_json() {
    let cluster = resource(
        "arn:aws:ecs:ap-south-1:123456789012:cluster/prod",
        json!({"clusterArn":"arn:aws:ecs:ap-south-1:123456789012:cluster/prod"}),
    );
    let service = resource(
        "arn:aws:ecs:ap-south-1:123456789012:service/prod/api",
        json!({"serviceName":"api"}),
    );
    let ecs_actions = available(
        Service::Ecs,
        1,
        &service,
        &cluster,
        &Snapshot::empty(Service::Ecs),
    );
    assert_eq!(ecs_actions[0].input["cluster"], cluster.id);
    assert_eq!(ecs_actions[0].input["service"], "api");
    assert_eq!(ecs_actions[2].input["force"], false);

    let repository = resource(
        "team/api",
        json!({"repositoryName":"team/api","repositoryArn":"arn:aws:ecr:ap-south-1:123456789012:repository/team/api"}),
    );
    let digest = "sha256:abc123";
    let image = resource(digest, json!({"imageDigest":digest}));
    let image_actions = available(
        Service::Ecr,
        1,
        &image,
        &repository,
        &Snapshot::empty(Service::Ecr),
    );
    assert_eq!(image_actions[0].input["repositoryName"], repository.id);
    assert_eq!(image_actions[0].input["imageId"]["imageDigest"], digest);
    assert_eq!(image_actions[2].input["imageIds"][0]["imageDigest"], digest);

    let tag = resource(
        "sha256:abc123:latest",
        json!({"digest":digest,"tag":"latest"}),
    );
    let tag_actions = available(
        Service::Ecr,
        2,
        &tag,
        &repository,
        &Snapshot::empty(Service::Ecr),
    );
    assert_eq!(tag_actions[0].input["imageIds"][0]["imageTag"], "latest");
}

#[test]
fn task_count_validation_is_nonnegative_i32_and_sets_the_json_value() {
    let action = operation(
        "Set desired task count",
        "ecs",
        "update-service",
        json!({"desiredCount":1}),
        "service",
        "warning",
        Some("Desired task count"),
    );
    assert_eq!(action.args("0").unwrap()[3], r#"{"desiredCount":0}"#);
    assert!(action.args("-1").is_err());
    assert!(action.args("2147483648").is_err());
    assert!(action.args("1.5").is_err());
}

#[test]
fn s3_operations_require_and_use_the_selected_bucket_region() {
    let bucket = resource("a-bucket", json!({}));
    let mut snapshot = Snapshot::empty(Service::S3);
    snapshot.panels[3]
        .items
        .push(resource("location", json!({"region":"ap-south-1"})));
    snapshot.panels[3]
        .items
        .push(resource("versioning", json!({"Status":"Enabled"})));
    let actions = available(Service::S3, 0, &bucket, &bucket, &snapshot);
    assert_eq!(actions[0].region.as_deref(), Some("ap-south-1"));
    assert_eq!(
        actions[0].input["VersioningConfiguration"]["Status"],
        "Suspended"
    );
    assert!(
        available(
            Service::S3,
            0,
            &bucket,
            &bucket,
            &Snapshot::empty(Service::S3)
        )
        .is_empty()
    );
}

#[test]
fn upload_args_keep_keys_literal_and_require_a_regular_file() {
    let dir = std::env::temp_dir().join(format!("lazyaws-operations-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir(&dir).unwrap();
    let path = dir.join("hello world.txt");
    fs::write(&path, "data").unwrap();
    let action = operation(
        "Replace object",
        "s3api",
        "put-object",
        json!({"Bucket":"bucket","Key":"folder/--cli-input-json file://literal"}),
        "object",
        "warning",
        Some("Upload file path"),
    );
    let args = action.args(path.to_str().unwrap()).unwrap();
    let input: Value = serde_json::from_str(&args[3]).unwrap();
    assert_eq!(input["Key"], "folder/--cli-input-json file://literal");
    assert_eq!(args[4], "--body");
    assert_eq!(PathBuf::from(&args[5]), fs::canonicalize(&path).unwrap());
    assert!(action.args(dir.to_str().unwrap()).is_err());

    let bucket_upload = operation(
        "Upload file",
        "s3api",
        "put-object",
        json!({"Bucket":"bucket"}),
        "bucket",
        "warning",
        Some("Upload file path"),
    );
    let args = bucket_upload.args(path.to_str().unwrap()).unwrap();
    let input: Value = serde_json::from_str(&args[3]).unwrap();
    assert_eq!(input["Key"], "hello world.txt");

    let oversized = dir.join("oversized.bin");
    File::create(&oversized)
        .unwrap()
        .set_len(crate::config::S3_MAX_UPLOAD_BYTES + 1)
        .unwrap();
    assert!(
        bucket_upload
            .args(oversized.to_str().unwrap())
            .unwrap_err()
            .contains("5 GB")
    );
    let _ = fs::remove_dir_all(dir);
}
