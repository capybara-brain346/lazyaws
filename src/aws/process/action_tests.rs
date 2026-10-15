use std::fs;

use crate::operations::Operation;

use super::*;

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
fn action_logs_do_not_expose_json_input_values() {
    let _guard = FAKE_CLI_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let fake = FakeCli::new("exit 0");
    let operation = Operation {
        label: "Run task",
        service: "ecs",
        name: "run-task",
        input: serde_json::json!({"cluster":"prod","taskDefinition":"api:1","overrides":{"containerOverrides":[{"name":"api","environment":[{"name":"TOKEN","value":"sensitive-example"}]}]}}),
        region: None,
        target: "prod".into(),
        warning: "Starts a task.",
        parameter: None,
    };
    let mut logs = Vec::new();
    fake.client().execute(&operation, "", &mut logs).unwrap();
    assert!(logs[0].contains("run-task"));
    assert!(logs[0].contains("[JSON input hidden]"));
    assert!(!logs[0].contains("sensitive-example"));
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
fn new_task_and_manifest_actions_report_api_failures_after_cli_success() {
    let _guard = FAKE_CLI_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    for (service, name, response, expected) in [
        (
            "ecs",
            "run-task",
            r#"{"tasks":[],"failures":[{"arn":"arn:task","reason":"RESOURCE:MEMORY","detail":"capacity"}]}"#,
            "ECS could not start task(s): arn:task: RESOURCE:MEMORY: capacity",
        ),
        (
            "ecr",
            "batch-get-image",
            r#"{"images":[],"failures":[{"imageId":{"imageTag":"latest"},"failureCode":"ImageNotFound","failureReason":"missing"}]}"#,
            "ECR could not read image(s): latest: ImageNotFound: missing",
        ),
    ] {
        let fake = FakeCli::new(&format!("printf '%s\\n' '{response}'"));
        let operation = Operation {
            label: "Test action",
            service,
            name,
            input: serde_json::json!({}),
            region: None,
            target: "target".into(),
            warning: "warning",
            parameter: None,
        };
        let mut logs = Vec::new();
        let error = fake
            .client()
            .execute(&operation, "", &mut logs)
            .unwrap_err();
        assert_eq!(error, expected);
        assert!(logs[0].contains("failed"));
    }
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

#[test]
fn lambda_invoke_merges_payload_and_reports_function_errors() {
    let _guard = FAKE_CLI_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let fake = FakeCli::new(
        "for last do :; done\nprintf '{\"ok\":true}' > \"$last\"\nprintf '%s\\n' '{\"StatusCode\":200}'",
    );
    let operation = Operation {
        label: "Invoke function",
        service: "lambda",
        name: "invoke",
        input: serde_json::json!({"FunctionName": "api"}),
        region: None,
        target: "api".into(),
        warning: "Invokes the function.",
        parameter: Some("Invoke payload"),
    };
    let mut logs = Vec::new();
    let response = fake
        .client()
        .execute(&operation, r#"{"key":"value"}"#, &mut logs)
        .unwrap();
    assert_eq!(response["StatusCode"], 200);
    assert_eq!(response["Payload"], serde_json::json!({"ok": true}));
    assert!(logs[0].contains("[JSON input hidden]"));
    assert!(!logs[0].contains("key"));

    let failing = FakeCli::new(
        "for last do :; done\nprintf '{\"error\":\"boom\"}' > \"$last\"\nprintf '%s\\n' '{\"StatusCode\":200,\"FunctionError\":\"Unhandled\"}'",
    );
    let error = failing
        .client()
        .execute(&operation, "{}", &mut Vec::new())
        .unwrap_err();
    assert!(error.contains("Unhandled"), "{error}");
    assert!(error.contains("boom"), "{error}");
}

#[test]
fn lambda_invoke_with_log_tail_decodes_logs() {
    let _guard = FAKE_CLI_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    // "hello logs\n" base64 is "aGVsbG8gbG9ncwo=".
    let fake = FakeCli::new(
        "for last do :; done\nprintf 'null' > \"$last\"\nprintf '%s\\n' '{\"StatusCode\":200,\"LogResult\":\"aGVsbG8gbG9ncwo=\"}'",
    );
    let operation = Operation {
        label: "Invoke with log tail",
        service: "lambda",
        name: "invoke",
        input: serde_json::json!({"FunctionName": "api"}),
        region: None,
        target: "api".into(),
        warning: "Invokes the function.",
        parameter: Some("Invoke payload with logs"),
    };
    let response = fake
        .client()
        .execute(&operation, "", &mut Vec::new())
        .unwrap();
    assert_eq!(response["LogResultDecoded"], "hello logs\n");
}

#[test]
fn lambda_fetch_scopes_related_calls_to_the_selected_function() {
    let _guard = FAKE_CLI_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let fake = FakeCli::new(
        r#"
printf '%s\n' "$*" >> LOG_PATH
case "$*" in
  *"lambda list-functions"*) printf '%s\n' '{"Functions":[{"FunctionName":"other","FunctionArn":"arn:aws:lambda:eu-west-1:111111111111:function:other","Runtime":"python3.12"},{"FunctionName":"selected","FunctionArn":"arn:aws:lambda:eu-west-1:111111111111:function:selected","Runtime":"nodejs22.x"}]}' ;;
  *"lambda list-versions-by-function"*) printf '%s\n' '{"Versions":[{"FunctionArn":"arn:aws:lambda:eu-west-1:111111111111:function:selected:4","Version":"4"}]}' ;;
  *"lambda list-aliases"*) printf '%s\n' '{"Aliases":[{"AliasArn":"arn:aws:lambda:eu-west-1:111111111111:function:selected:prod","Name":"prod","FunctionVersion":"4"}]}' ;;
  *"lambda list-tags"*) printf '%s\n' '{"Tags":{"Team":"platform"}}' ;;
  *) echo 'unexpected Lambda command' >&2; exit 2 ;;
esac
"#
        .trim(),
    );
    let transcript = fake.enable_transcript();
    let snapshot = fake
        .client()
        .fetch(
            crate::model::Service::Lambda,
            Some("arn:aws:lambda:eu-west-1:111111111111:function:selected"),
        )
        .unwrap();

    assert_eq!(snapshot.panels[0].items.len(), 2);
    assert_eq!(
        snapshot.panels[1].items[0].id,
        "arn:aws:lambda:eu-west-1:111111111111:function:selected:4"
    );
    assert_eq!(
        snapshot.panels[2].items[0].id,
        "arn:aws:lambda:eu-west-1:111111111111:function:selected:prod"
    );
    assert_eq!(snapshot.panels[3].items[0].id, "Team");
    assert!(snapshot.panels.iter().all(|panel| panel.error.is_none()));
    let commands = fs::read_to_string(transcript).unwrap();
    assert_eq!(
        commands
            .lines()
            .filter(|line| line.contains("list-versions-by-function"))
            .count(),
        1
    );
    assert!(commands.contains("arn:aws:lambda:eu-west-1:111111111111:function:selected"));
    assert!(!commands.contains("function:other:"));
}
