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
fn object_prefix_and_settings_actions_keep_the_bucket_region_and_target() {
    let bucket = resource("selected-bucket", json!({"BucketRegion":"eu-west-1"}));
    let snapshot = Snapshot::empty(crate::model::Service::S3);
    for (panel, id, field) in [
        (1, "folder/file --literal.txt", "Key"),
        (2, "folder/", "Prefix"),
        (3, "location", "Bucket"),
    ] {
        let row = resource(id, json!({}));
        let actions = available(panel, &row, &bucket, &snapshot);
        assert!(actions.len() > 1);
        for action in &actions {
            assert_eq!(action.region.as_deref(), Some("eu-west-1"));
            assert_eq!(action.input["Bucket"], "selected-bucket");
        }
        assert_eq!(
            actions[0].input[field],
            if panel == 3 { "selected-bucket" } else { id }
        );
        let json_action = actions
            .iter()
            .find(|action| action.parameter == Some(Parameter::JsonFields));
        if let Some(action) = json_action {
            assert!(action.args(r#"{"Bucket":"other"}"#).is_err());
        }
    }
    assert!(available(1, &resource("", json!({})), &bucket, &snapshot).is_empty());
    assert!(available(2, &resource("", json!({})), &bucket, &snapshot).is_empty());
}

#[test]
fn versioning_setting_row_uses_the_known_versioning_state() {
    let bucket = resource("bucket", json!({"BucketRegion":"eu-west-1"}));
    let row = resource("versioning", json!({"Status":"Enabled"}));
    let mut snapshot = Snapshot::empty(crate::model::Service::S3);
    snapshot.panels[3].items.push(row.clone());
    let actions = available(3, &row, &bucket, &snapshot);
    let versioning = actions
        .iter()
        .find(|action| action.name == "put-bucket-versioning")
        .unwrap();
    assert_eq!(
        versioning.input["VersioningConfiguration"]["Status"],
        "Suspended"
    );
    assert!(
        !available(3, &resource("location", json!({})), &bucket, &snapshot)
            .iter()
            .any(|action| action.name == "put-bucket-versioning")
    );
}
