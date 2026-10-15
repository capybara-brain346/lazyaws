use serde_json::json;

use super::{Operation, catalog::operation};

fn action(parameter: &'static str, service: &'static str, name: &'static str) -> Operation {
    operation(
        "Test action",
        service,
        name,
        json!({}),
        "target",
        "warning",
        Some(parameter),
    )
}

#[test]
fn json_fields_cannot_replace_captured_targets_or_options() {
    let mut action = action("JSON fields", "ecr", "set-repository-policy");
    action.input = json!({"repositoryName":"team/app","force":false});
    let args = action.args(r#"{"policyText":"{}"}"#).unwrap();
    let input: serde_json::Value = serde_json::from_str(&args[3]).unwrap();
    assert_eq!(input["repositoryName"], "team/app");
    assert_eq!(input["policyText"], "{}");
    for value in [
        r#"{"repositoryName":"other"}"#,
        r#"{"force":true}"#,
        r#"{"registryId":"999999999999"}"#,
        "[]",
        "{}",
        "null",
        "bad",
    ] {
        assert!(action.args(value).is_err(), "{value}");
    }
}

#[test]
fn paginated_actions_use_the_configured_item_limit() {
    let mut action = action("JSON fields", "s3api", "list-object-versions");
    action.parameter = None;
    let args = action.args("").unwrap();
    assert_eq!(
        &args[4..],
        ["--max-items", &crate::config::MAX_ITEMS.to_string()]
    );
    action.name = "head-object";
    assert_eq!(action.args("").unwrap().len(), 4);
    action.service = "ecs";
    action.name = "list-service-deployments";
    assert_eq!(action.args("").unwrap().len(), 4);
}

#[test]
fn tag_value_keeps_the_selected_key_and_supports_empty_values() {
    for (service, name, field) in [
        ("ec2", "create-tags", "Tags"),
        ("ecr", "tag-resource", "tags"),
    ] {
        let mut action = action("Tag value", service, name);
        action.input = json!({field:[{"Key":"owner"}]});
        for value in ["", "new owner"] {
            let args = action.args(value).unwrap();
            let input: serde_json::Value = serde_json::from_str(&args[3]).unwrap();
            assert_eq!(input[field][0], json!({"Key":"owner","Value":value}));
        }
        action.input = json!({field:[]});
        assert!(action.args("value").is_err());
    }
}

#[test]
fn task_definition_update_requires_an_explicit_positive_revision() {
    let mut action = action("Task definition revision", "ecs", "update-service");
    action.target = "arn:aws:ecs:eu-west-1:123456789012:service/prod/api".into();
    for value in [
        "api:12",
        "arn:aws:ecs:eu-west-1:123456789012:task-definition/api:12",
    ] {
        let args = action.args(value).unwrap();
        let input: serde_json::Value = serde_json::from_str(&args[3]).unwrap();
        assert_eq!(input["taskDefinition"], value);
    }
    for value in [
        "api",
        "api:0",
        "api:-1",
        "api:1:2",
        "arn:invalid",
        "bad family:1",
        "arn:bad:task-definition/api:12",
        "arn:aws:ecs:us-east-1:123456789012:task-definition/api:12",
        "arn:aws:ecs:eu-west-1:999999999999:task-definition/api:12",
    ] {
        assert!(action.args(value).is_err(), "{value}");
    }
}

#[test]
fn rollback_arn_must_match_the_captured_service_account_and_region() {
    let mut action = action("Service deployment ARN", "ecs", "stop-service-deployment");
    action.target = "arn:aws:ecs:eu-west-1:123456789012:service/prod/api".into();
    action.input = json!({"stopType":"ROLLBACK"});
    let arn = "arn:aws:ecs:eu-west-1:123456789012:service-deployment/prod/api/deployment-1";
    let args = action.args(arn).unwrap();
    let input: serde_json::Value = serde_json::from_str(&args[3]).unwrap();
    assert_eq!(
        input,
        json!({"stopType":"ROLLBACK","serviceDeploymentArn":arn})
    );
    for value in [
        arn.replace("/api/", "/other/"),
        arn.replace("eu-west-1", "us-east-1"),
        arn.replace("123456789012", "999999999999"),
        format!("{arn}/extra"),
    ] {
        assert!(action.args(&value).is_err(), "{value}");
    }
}

#[test]
fn download_provides_required_cli_flags_without_interpreting_object_keys_as_options() {
    let mut action = action("Download file path", "s3api", "get-object");
    action.input = json!({"Bucket":"selected-bucket","Key":"--cli-input-json file://literal"});
    let destination = std::env::temp_dir().join(format!(
        "lazyaws-required-download-{}.txt",
        std::process::id()
    ));
    let args = action.args(destination.to_str().unwrap()).unwrap();
    assert_eq!(args[2], "--bucket=selected-bucket");
    assert_eq!(args[3], "--key=--cli-input-json file://literal");
    assert_eq!(args[4], destination.to_str().unwrap());
    assert!(!args.iter().any(|arg| arg == "--cli-input-json"));
    action.input = json!({"Bucket":"selected-bucket"});
    assert!(action.args(destination.to_str().unwrap()).is_err());
}

#[test]
fn lambda_invoke_builds_flag_args_with_a_placeholder_outfile() {
    for (parameter, extra) in [
        ("Invoke payload", Vec::new()),
        (
            "Invoke payload with logs",
            vec!["--log-type".to_string(), "Tail".to_string()],
        ),
        (
            "Invoke payload async",
            vec!["--invocation-type".to_string(), "Event".to_string()],
        ),
    ] {
        let mut action = action(parameter, "lambda", "invoke");
        action.input = json!({"FunctionName": "api"});
        let args = action.args(r#"{"key":"value"}"#).unwrap();
        assert_eq!(args[0], "lambda");
        assert_eq!(args[1], "invoke");
        assert_eq!(args[2], "--function-name");
        assert_eq!(args[3], "api");
        assert_eq!(args[4], "--payload");
        assert_eq!(args[5], r#"{"key":"value"}"#);
        for (index, flag) in extra.iter().enumerate() {
            assert_eq!(&args[6 + index], flag);
        }
        assert_eq!(args.last().unwrap(), crate::aws::INVOKE_OUTPUT_PLACEHOLDER);
        assert!(!args.iter().any(|arg| arg == "--cli-input-json"));

        let empty = action.args("").unwrap();
        assert!(!empty.iter().any(|arg| arg == "--payload"));
        assert!(action.args("not json").is_err());
        action.input = json!({});
        assert!(action.args("{}").is_err());
    }
}

#[test]
fn lambda_concurrency_alias_and_tag_inputs_are_validated() {
    let mut reserved = action("Reserved concurrency", "lambda", "put-function-concurrency");
    reserved.input = json!({"FunctionName": "api"});
    let args = reserved.args("10").unwrap();
    let input: serde_json::Value = serde_json::from_str(&args[3]).unwrap();
    assert_eq!(input["ReservedConcurrentExecutions"], 10);
    assert!(reserved.args("-1").is_err());

    let mut provisioned = action(
        "Provisioned concurrency",
        "lambda",
        "put-provisioned-concurrency-config",
    );
    provisioned.input = json!({"FunctionName": "api", "Qualifier": "3"});
    let args = provisioned.args("5").unwrap();
    let input: serde_json::Value = serde_json::from_str(&args[3]).unwrap();
    assert_eq!(input["ProvisionedConcurrentExecutions"], 5);
    assert!(provisioned.args("0").is_err());

    let mut alias = action("Alias version", "lambda", "update-alias");
    alias.input = json!({"FunctionName": "api", "Name": "prod"});
    for valid in ["3", "$LATEST"] {
        let args = alias.args(valid).unwrap();
        let input: serde_json::Value = serde_json::from_str(&args[3]).unwrap();
        assert_eq!(input["FunctionVersion"], valid);
    }
    for invalid in ["", "v3", "0", "prod"] {
        assert!(alias.args(invalid).is_err(), "{invalid}");
    }

    let mut tag = action("Lambda tag value", "lambda", "tag-resource");
    tag.input = json!({"Resource": "arn:function", "Tags": {"Team": ""}});
    for value in ["", "platform"] {
        let args = tag.args(value).unwrap();
        let input: serde_json::Value = serde_json::from_str(&args[3]).unwrap();
        assert_eq!(input["Tags"]["Team"], value);
    }
    tag.input = json!({"Resource": "arn:function", "Tags": {}});
    assert!(tag.args("value").is_err());
}

#[test]
fn lambda_paginated_actions_use_the_configured_item_limit() {
    for name in [
        "list-functions",
        "list-versions-by-function",
        "list-aliases",
        "list-event-source-mappings",
        "list-function-event-invoke-configs",
        "list-function-url-configs",
        "list-provisioned-concurrency-configs",
    ] {
        let mut action = action("JSON fields", "lambda", name);
        action.parameter = None;
        let args = action.args("").unwrap();
        assert_eq!(
            &args[4..],
            ["--max-items", &crate::config::MAX_ITEMS.to_string()],
            "{name}"
        );
    }
}
