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
fn function_actions_include_invoke_reads_and_lifecycle() {
    let function = resource(
        "arn:aws:lambda:eu-west-1:123456789012:function:api",
        json!({"FunctionName": "api"}),
    );
    let actions = available(0, &function, &function);
    let names = actions.iter().map(|action| action.name).collect::<Vec<_>>();

    assert!(names.contains(&"invoke"));
    assert!(names.contains(&"get-function"));
    assert!(names.contains(&"get-function-configuration"));
    assert!(names.contains(&"get-policy"));
    assert!(names.contains(&"publish-version"));
    assert!(names.contains(&"update-function-configuration"));
    assert!(names.contains(&"put-function-concurrency"));
    assert!(names.contains(&"delete-function-concurrency"));
    assert!(names.contains(&"create-alias"));
    assert!(names.contains(&"tag-resource"));
    assert!(names.contains(&"delete-function"));

    let invoke = actions
        .iter()
        .find(|action| action.label == "Invoke function")
        .unwrap();
    assert_eq!(
        invoke.parameter,
        Some(Parameter::InvokePayload(InvokeMode::Sync))
    );
    assert_eq!(invoke.input["FunctionName"], function.id);

    let tag = actions
        .iter()
        .find(|action| action.label == "Add function tags")
        .unwrap();
    assert_eq!(tag.input["Resource"], function.id);
    assert_eq!(tag.parameter, Some(Parameter::JsonFields));
}

#[test]
fn version_actions_gate_latest_for_capacity_and_delete() {
    let primary = resource(
        "arn:aws:lambda:eu-west-1:123456789012:function:api",
        json!({"FunctionName": "api"}),
    );
    let version = resource(
        "arn:aws:lambda:eu-west-1:123456789012:function:api:3",
        json!({"Version": "3"}),
    );
    let actions = available(1, &version, &primary);
    let names = actions.iter().map(|action| action.name).collect::<Vec<_>>();

    assert!(names.contains(&"invoke"));
    assert!(names.contains(&"get-function"));
    assert!(names.contains(&"get-provisioned-concurrency-config"));
    assert!(names.contains(&"put-provisioned-concurrency-config"));
    assert!(names.contains(&"delete-provisioned-concurrency-config"));
    assert!(names.contains(&"delete-function"));

    let latest = resource(
        "arn:aws:lambda:eu-west-1:123456789012:function:api:$LATEST",
        json!({"Version": "$LATEST"}),
    );
    let latest_names = available(1, &latest, &primary)
        .iter()
        .map(|action| action.name)
        .collect::<Vec<_>>();
    assert!(!latest_names.contains(&"delete-function"));
    assert!(!latest_names.contains(&"put-provisioned-concurrency-config"));
}

#[test]
fn alias_and_tag_actions_keep_captured_targets() {
    let primary = resource(
        "arn:aws:lambda:eu-west-1:123456789012:function:api",
        json!({"FunctionName": "api"}),
    );
    let alias = resource(
        "arn:aws:lambda:eu-west-1:123456789012:function:api:prod",
        json!({"Name": "prod", "FunctionVersion": "3"}),
    );
    let actions = available(2, &alias, &primary);
    let update = actions
        .iter()
        .find(|action| action.name == "update-alias")
        .unwrap();
    assert_eq!(update.input["FunctionName"], "api");
    assert_eq!(update.input["Name"], "prod");
    assert_eq!(update.parameter, Some(Parameter::AliasVersion));

    let tag = resource("Team", json!({"Key": "Team", "Value": "commerce"}));
    let tag_actions = available(3, &tag, &primary);
    let remove = tag_actions
        .iter()
        .find(|action| action.name == "untag-resource")
        .unwrap();
    assert_eq!(remove.input["Resource"], primary.id);
    assert_eq!(remove.input["TagKeys"], json!(["Team"]));
    let edit = tag_actions
        .iter()
        .find(|action| {
            action.name == "tag-resource" && action.parameter == Some(Parameter::LambdaTagValue)
        })
        .unwrap();
    assert_eq!(edit.input["Tags"], json!({"Team": ""}));
}
