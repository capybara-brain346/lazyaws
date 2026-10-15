use super::*;

fn resource(id: &str, detail: Value) -> Resource {
    Resource {
        id: id.into(),
        label: id.into(),
        summary: String::new(),
        detail,
    }
}

fn action<'a>(
    actions: &'a [crate::operations::Operation],
    name: &str,
) -> &'a crate::operations::Operation {
    actions
        .iter()
        .find(|action| action.name == name)
        .unwrap_or_else(|| panic!("missing operation {name}"))
}

#[test]
fn repository_actions_capture_context_and_keep_mutability_exclusions() {
    let repository = resource(
        "team/api",
        json!({
            "repositoryName":"team/api",
            "repositoryArn":"arn:aws:ecr:ap-south-1:123456789012:repository/team/api",
            "imageTagMutability":"MUTABLE_WITH_EXCLUSION",
            "imageTagMutabilityExclusionFilters":[{"filterType":"WILDCARD","filter":"release-*"}],
            "imageScanningConfiguration":{"scanOnPush":false}
        }),
    );
    let actions = available(0, &repository, &repository);

    let mutability = action(&actions, "put-image-tag-mutability");
    assert_eq!(mutability.input["repositoryName"], "team/api");
    assert_eq!(
        mutability.input["imageTagMutability"],
        "IMMUTABLE_WITH_EXCLUSION"
    );
    assert_eq!(
        mutability.input["imageTagMutabilityExclusionFilters"],
        json!([{"filterType":"WILDCARD","filter":"release-*"}])
    );

    let get_policy = action(&actions, "get-repository-policy");
    assert_eq!(get_policy.input["repositoryName"], "team/api");
    let set_policy = action(&actions, "set-repository-policy");
    assert_eq!(set_policy.parameter, Some("JSON fields"));
    assert_eq!(set_policy.input["repositoryName"], "team/api");
    let args = set_policy
        .args(r#"{"policyText":"{\"Version\":\"2012-10-17\",\"Statement\":[]}"}"#)
        .unwrap();
    let input: Value = serde_json::from_str(&args[3]).unwrap();
    assert_eq!(input["repositoryName"], "team/api");
    assert_eq!(
        input["policyText"],
        r#"{"Version":"2012-10-17","Statement":[]}"#
    );
    assert!(
        set_policy
            .args(r#"{"repositoryName":"team/other"}"#)
            .is_err()
    );
    let lifecycle = action(&actions, "put-lifecycle-policy");
    assert_eq!(lifecycle.parameter, Some("JSON fields"));
    assert_eq!(lifecycle.input["repositoryName"], "team/api");

    let scan = action(&actions, "put-image-scanning-configuration");
    assert_eq!(scan.input["repositoryName"], "team/api");
    assert_eq!(scan.input["imageScanningConfiguration"]["scanOnPush"], true);
    assert!(scan.warning.contains("deprecated"));

    let missing_filters = resource(
        "team/api",
        json!({"imageTagMutability":"IMMUTABLE_WITH_EXCLUSION"}),
    );
    assert!(
        available(0, &missing_filters, &missing_filters)
            .iter()
            .all(|action| action.name != "put-image-tag-mutability")
    );
}

#[test]
fn image_reads_use_digest_or_tag_from_the_selected_row() {
    let repository = resource("team/api", json!({"repositoryName":"team/api"}));
    let digest = "sha256:abc123";
    let image = resource(digest, json!({"imageDigest":digest}));
    let image_actions = available(1, &image, &repository);

    assert_eq!(
        action(&image_actions, "describe-images").input["imageIds"],
        json!([{"imageDigest":digest}])
    );
    assert_eq!(
        action(&image_actions, "batch-get-image").input["imageIds"],
        json!([{"imageDigest":digest}])
    );

    let tag = resource(
        "sha256:abc123:latest",
        json!({"digest":digest,"tag":"latest"}),
    );
    let tag_actions = available(2, &tag, &repository);
    assert_eq!(
        action(&tag_actions, "describe-images").input["imageIds"],
        json!([{"imageTag":"latest"}])
    );
    assert_eq!(
        action(&tag_actions, "batch-get-image").input["imageIds"],
        json!([{"imageTag":"latest"}])
    );
}

#[test]
fn repository_tag_updates_keep_the_arn_and_selected_key() {
    let repository = resource(
        "team/api",
        json!({"repositoryArn":"arn:aws:ecr:ap-south-1:123456789012:repository/team/api"}),
    );
    let tag = resource("owner", json!({"Key":"owner","Value":"platform"}));
    let actions = available(3, &tag, &repository);

    let update = action(&actions, "tag-resource");
    assert_eq!(update.parameter, Some("Tag value"));
    let args = update.args("new owner").unwrap();
    let input: Value = serde_json::from_str(&args[3]).unwrap();
    assert_eq!(input["resourceArn"], repository.detail["repositoryArn"]);
    assert_eq!(input["tags"], json!([{"Key":"owner","Value":"new owner"}]));

    let add = actions
        .iter()
        .find(|action| action.name == "tag-resource" && action.parameter == Some("JSON fields"))
        .unwrap();
    let args = add
        .args(r#"{"tags":[{"Key":"team","Value":"platform"}]}"#)
        .unwrap();
    let input: Value = serde_json::from_str(&args[3]).unwrap();
    assert_eq!(input["resourceArn"], repository.detail["repositoryArn"]);
    assert_eq!(input["tags"], json!([{"Key":"team","Value":"platform"}]));
    assert!(
        add.args(r#"{"resourceArn":"arn:aws:ecr:us-east-1:000000000000:repository/other"}"#)
            .is_err()
    );
}
