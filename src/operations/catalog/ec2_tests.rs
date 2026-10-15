use super::*;

fn resource(id: &str, detail: Value) -> Resource {
    Resource {
        id: id.into(),
        label: id.into(),
        summary: String::new(),
        detail,
    }
}

fn action<'a>(actions: &'a [Operation], name: &str) -> &'a Operation {
    actions.iter().find(|action| action.name == name).unwrap()
}

#[test]
fn instance_actions_respect_root_storage_and_api_protection() {
    let instance_store = resource(
        "i-store",
        json!({
            "State":{"Name":"running"},
            "RootDeviceType":"instance-store"
        }),
    );
    let actions = available(0, &instance_store, &instance_store);
    let names = actions.iter().map(|action| action.name).collect::<Vec<_>>();
    assert!(!names.contains(&"stop-instances"));
    assert!(names.contains(&"terminate-instances"));
    assert!(names.contains(&"describe-instance-status"));
    assert!(names.contains(&"get-console-output"));

    let stopped = resource(
        "i-stopped",
        json!({"State":{"Name":"stopped"},"RootDeviceType":"ebs"}),
    );
    let actions = available(0, &stopped, &stopped);
    assert!(
        actions
            .iter()
            .any(|action| action.name == "start-instances")
    );
    assert!(!actions.iter().any(|action| action.name == "stop-instances"));

    let terminated = resource(
        "i-terminated",
        json!({"State":{"Name":"terminated"},"RootDeviceType":"ebs"}),
    );
    let actions = available(0, &terminated, &terminated);
    assert!(
        !actions
            .iter()
            .any(|action| action.name == "terminate-instances")
    );

    let protected = resource(
        "i-protected",
        json!({
            "State":{"Name":"running"},
            "RootDeviceType":"ebs",
            "DisableApiStop":true,
            "DisableApiTermination":true
        }),
    );
    let actions = available(0, &protected, &protected);
    assert!(!actions.iter().any(|action| action.name == "stop-instances"));
    assert!(
        !actions
            .iter()
            .any(|action| action.name == "terminate-instances")
    );
    assert!(
        actions
            .iter()
            .any(|action| action.name == "reboot-instances")
    );
}

#[test]
fn volume_resize_is_pinned_and_detach_uses_only_the_selected_instances_attachment() {
    let primary = resource(
        "i-selected",
        json!({"State":{"Name":"running"},"RootDeviceName":"/dev/sda1"}),
    );
    let volume = resource(
        "vol-data",
        json!({
            "State":"in-use",
            "Size":40,
            "Attachments":[
                {"InstanceId":"i-other","Device":"/dev/sdg","State":"attached"},
                {"InstanceId":"i-selected","Device":"/dev/sdf","State":"attached"}
            ]
        }),
    );
    let actions = available(1, &volume, &primary);
    let resize = action(&actions, "modify-volume");
    assert_eq!(resize.input, json!({"VolumeId":"vol-data"}));
    assert_eq!(resize.parameter, Some("JSON fields"));

    let detach = action(&actions, "detach-volume");
    assert_eq!(
        detach.input,
        json!({"VolumeId":"vol-data","InstanceId":"i-selected","Device":"/dev/sdf"})
    );

    let wrong_primary = resource(
        "i-not-attached",
        json!({"State":{"Name":"running"},"RootDeviceName":"/dev/sda1"}),
    );
    let actions = available(1, &volume, &wrong_primary);
    assert!(!actions.iter().any(|action| action.name == "detach-volume"));

    let root_volume = resource(
        "vol-root",
        json!({
            "State":"in-use",
            "Attachments":[
                {"InstanceId":"i-root","Device":"/dev/sda1","State":"attached"}
            ]
        }),
    );
    let running_root = resource(
        "i-root",
        json!({"State":{"Name":"running"},"RootDeviceName":"/dev/sda1"}),
    );
    let actions = available(1, &root_volume, &running_root);
    assert!(!actions.iter().any(|action| action.name == "detach-volume"));

    let stopped_root = resource(
        "i-root",
        json!({"State":{"Name":"stopped"},"RootDeviceName":"/dev/sda1"}),
    );
    let actions = available(1, &root_volume, &stopped_root);
    assert!(actions.iter().any(|action| action.name == "detach-volume"));
}

#[test]
fn security_group_rules_and_tag_edits_keep_their_targets_fixed() {
    let group = resource("sg-123", json!({}));
    let group_actions = available(2, &group, &group);
    for name in [
        "authorize-security-group-ingress",
        "revoke-security-group-ingress",
        "authorize-security-group-egress",
        "revoke-security-group-egress",
    ] {
        let operation = action(&group_actions, name);
        assert_eq!(operation.input, json!({"GroupId":"sg-123"}));
        assert_eq!(operation.parameter, Some("JSON fields"));
    }

    let primary = resource("i-123", json!({}));
    let tag = resource("Environment", json!({"Key":"Environment","Value":"test"}));
    let tag_actions = available(3, &tag, &primary);
    let update = action(&tag_actions, "create-tags");
    assert_eq!(
        update.input,
        json!({"Resources":["i-123"],"Tags":[{"Key":"Environment"}]})
    );
    assert_eq!(update.parameter, Some("Tag value"));

    let add = tag_actions
        .iter()
        .find(|operation| operation.label == "Add or update instance tags")
        .unwrap();
    assert_eq!(add.input, json!({"Resources":["i-123"]}));
    assert_eq!(add.parameter, Some("JSON fields"));
}
