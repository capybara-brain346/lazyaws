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
            let root_device_type = string(&resource.detail, &["RootDeviceType"]);
            let api_stop_disabled = resource
                .detail
                .get("DisableApiStop")
                .and_then(serde_json::Value::as_bool)
                == Some(true);
            let mut actions = Vec::new();
            if state == Some("stopped") && root_device_type != Some("instance-store") {
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
                if root_device_type != Some("instance-store") && !api_stop_disabled {
                    actions.push(operation(
                        "Stop instance",
                        "ec2",
                        "stop-instances",
                        json!({"InstanceIds":[id]}),
                        id,
                        "Stops this EBS-backed instance. Data in memory and instance store is lost.",
                        None,
                    ));
                }
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
            let api_termination_disabled = resource
                .detail
                .get("DisableApiTermination")
                .and_then(serde_json::Value::as_bool)
                == Some(true);
            if !matches!(state, Some("terminated" | "shutting-down"))
                && !api_stop_disabled
                && !api_termination_disabled
            {
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
            actions.push(
                operation(
                    "View instance status",
                    "ec2",
                    "describe-instance-status",
                    json!({"InstanceIds":[id],"IncludeAllInstances":true}),
                    id,
                    "Reads system and instance status checks and scheduled events.",
                    None,
                )
                .paginated(),
            );
            actions.push(operation(
                "View latest console output",
                "ec2",
                "get-console-output",
                json!({"InstanceId":id,"Latest":true}),
                id,
                "Reads boot console output. It can contain sensitive startup data.",
                None,
            ));
            if !matches!(state, Some("terminated" | "shutting-down")) {
                actions.push(operation(
                    "Add or update instance tags",
                    "ec2",
                    "create-tags",
                    json!({"Resources":[id]}),
                    id,
                    "Enter a Tags list with Key and Value. Tag changes can affect access policies.",
                    Some(Parameter::JsonFields),
                ));
            }
            actions
        }
        1 => {
            let state = string(&resource.detail, &["State", "state"]);
            let mut actions = vec![operation(
                "Create volume snapshot",
                "ec2",
                "create-snapshot",
                json!({"VolumeId":id}),
                id,
                "Creates a snapshot. Snapshot storage can incur charges.",
                None,
            )];
            if state == Some("available") {
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
            if matches!(state, Some("in-use" | "available")) {
                actions.push(operation(
                    "Increase volume size",
                    "ec2",
                    "modify-volume",
                    json!({"VolumeId":id}),
                    id,
                    "Enter a larger Size in JSON fields. AWS does not allow shrinking a volume. Extend its file system after the resize; EBS charges can increase.",
                    Some(Parameter::JsonFields),
                ));
            }
            actions.push(
                operation(
                    "View volume modification status",
                    "ec2",
                    "describe-volumes-modifications",
                    json!({"VolumeIds":[id]}),
                    id,
                    "Reads the most recent modification request for this volume.",
                    None,
                )
                .paginated(),
            );
            if let Some((instance_id, device)) = selected_attachment(resource, primary) {
                let is_root_volume = string(&primary.detail, &["RootDeviceName"]) == Some(device);
                let primary_state = nested_string(&primary.detail, &["State", "Name"]);
                if !is_root_volume || primary_state == Some("stopped") {
                    actions.push(operation(
                        "Detach volume from instance",
                        "ec2",
                        "detach-volume",
                        json!({"VolumeId":id,"InstanceId":instance_id,"Device":device}),
                        id,
                        "Unmount this volume first. Detaching a mounted file system can corrupt data. A root volume needs a stopped instance.",
                        None,
                    ));
                }
            }
            actions
        }
        2 => vec![
            operation(
                "Delete security group",
                "ec2",
                "delete-security-group",
                json!({"GroupId":id}),
                id,
                "Deletes this security group. AWS rejects it while it is in use.",
                None,
            ),
            operation(
                "Add inbound rules",
                "ec2",
                "authorize-security-group-ingress",
                json!({"GroupId":id}),
                id,
                "Enter an IpPermissions array in JSON fields. Use least-privilege sources. Changes affect every resource using this group.",
                Some(Parameter::JsonFields),
            ),
            operation(
                "Remove inbound rules",
                "ec2",
                "revoke-security-group-ingress",
                json!({"GroupId":id}),
                id,
                "Enter exact rule data or rule IDs in JSON fields. Changes affect every resource using this group.",
                Some(Parameter::JsonFields),
            ),
            operation(
                "Add outbound rules",
                "ec2",
                "authorize-security-group-egress",
                json!({"GroupId":id}),
                id,
                "Enter an IpPermissions array in JSON fields. Use least-privilege destinations. Changes affect every resource using this group.",
                Some(Parameter::JsonFields),
            ),
            operation(
                "Remove outbound rules",
                "ec2",
                "revoke-security-group-egress",
                json!({"GroupId":id}),
                id,
                "Enter exact rule data or rule IDs in JSON fields. Changes affect every resource using this group.",
                Some(Parameter::JsonFields),
            ),
        ],
        3 => {
            let key = string(&resource.detail, &["Key", "key"]).unwrap_or(id);
            vec![
                operation(
                    "Remove instance tag",
                    "ec2",
                    "delete-tags",
                    json!({"Resources":[primary.id],"Tags":[{"Key":key}]}),
                    id,
                    "Removes this tag from the selected instance.",
                    None,
                ),
                operation(
                    "Update instance tag value",
                    "ec2",
                    "create-tags",
                    json!({"Resources":[primary.id],"Tags":[{"Key":key}]}),
                    id,
                    "Overwrites this tag's value. Tag changes can affect policies that use tags.",
                    Some(Parameter::TagValue),
                ),
                operation(
                    "Add or update instance tags",
                    "ec2",
                    "create-tags",
                    json!({"Resources":[primary.id]}),
                    id,
                    "Enter a Tags array in JSON fields. Tag changes can affect policies that use tags.",
                    Some(Parameter::JsonFields),
                ),
            ]
        }
        _ => Vec::new(),
    }
}

fn selected_attachment<'a>(volume: &'a Resource, primary: &Resource) -> Option<(&'a str, &'a str)> {
    volume
        .detail
        .get("Attachments")
        .and_then(serde_json::Value::as_array)?
        .iter()
        .find_map(|attachment| {
            let instance_id = attachment.get("InstanceId")?.as_str()?;
            let device = attachment.get("Device")?.as_str()?;
            (instance_id == primary.id.as_str()
                && attachment.get("State").and_then(serde_json::Value::as_str) == Some("attached"))
            .then_some((instance_id, device))
        })
}

#[cfg(test)]
#[path = "ec2_tests.rs"]
mod tests;
