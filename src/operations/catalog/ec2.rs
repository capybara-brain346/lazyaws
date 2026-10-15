use serde_json::json;

use super::*;

pub(super) fn available(panel: usize, resource: &Resource, primary: &Resource) -> Vec<Operation> {
    let id = resource.id.as_str();
    if id.is_empty() {
        return Vec::new();
    }

    match panel {
        0 => {
            let state = nested_string(&resource.detail, &["State", "Name"])
                .or_else(|| string(&resource.detail, &["state"]));
            let mut actions = Vec::new();
            if state == Some("stopped") {
                actions.push(operation(
                    "Start instance",
                    "ec2",
                    "start-instances",
                    json!({"InstanceIds":[id]}),
                    id,
                    "Starts the instance. Compute charges apply.",
                    None,
                ));
            }
            if state == Some("running") {
                actions.push(operation(
                    "Stop instance",
                    "ec2",
                    "stop-instances",
                    json!({"InstanceIds":[id]}),
                    id,
                    "Stops the instance. Data in memory and instance store is lost.",
                    None,
                ));
                actions.push(operation(
                    "Reboot instance",
                    "ec2",
                    "reboot-instances",
                    json!({"InstanceIds":[id]}),
                    id,
                    "Reboots the instance. This can interrupt its work.",
                    None,
                ));
            }
            if !matches!(state, Some("terminated" | "shutting-down")) {
                actions.push(operation(
                    "Terminate instance",
                    "ec2",
                    "terminate-instances",
                    json!({"InstanceIds":[id]}),
                    id,
                    "Terminates this instance. AWS may also delete attached EBS volumes.",
                    None,
                ));
            }
            actions
        }
        1 => {
            let mut actions = vec![operation(
                "Create volume snapshot",
                "ec2",
                "create-snapshot",
                json!({"VolumeId":id}),
                id,
                "Creates a snapshot. Snapshot storage can incur charges.",
                None,
            )];
            if string(&resource.detail, &["State", "state"]) == Some("available") {
                actions.push(operation(
                    "Delete volume",
                    "ec2",
                    "delete-volume",
                    json!({"VolumeId":id}),
                    id,
                    "Permanently deletes this unattached volume and its data.",
                    None,
                ));
            }
            actions
        }
        2 => vec![operation(
            "Delete security group",
            "ec2",
            "delete-security-group",
            json!({"GroupId":id}),
            id,
            "Deletes this security group. AWS rejects it while it is in use.",
            None,
        )],
        3 => {
            let key = string(&resource.detail, &["Key", "key"]).unwrap_or(id);
            vec![operation(
                "Remove instance tag",
                "ec2",
                "delete-tags",
                json!({"Resources":[primary.id],"Tags":[{"Key":key}]}),
                id,
                "Removes this tag from the selected instance.",
                None,
            )]
        }
        _ => Vec::new(),
    }
}
