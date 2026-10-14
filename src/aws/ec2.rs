use serde_json::Value;

use crate::model::{Resource, Snapshot};

use super::{
    FetchContext,
    common::{
        array, optional_array, optional_string, panel_error, resource, selected_id, sort_and_limit,
        string, tag_resources,
    },
};

pub(super) fn fetch(cli: &FetchContext<'_>, selected: Option<&str>) -> Snapshot {
    let mut snapshot = Snapshot::empty(crate::model::Service::Ec2);
    let response = match cli.invoke(
        &mut snapshot.logs,
        &["ec2".into(), "describe-instances".into()],
        true,
    ) {
        Ok(response) => response,
        Err(error) => {
            panel_error(&mut snapshot, 0, error);
            return snapshot;
        }
    };

    match instances(&response) {
        Ok(items) => snapshot.panels[0].items = items,
        Err(error) => panel_error(&mut snapshot, 0, error),
    }
    sort_and_limit(&mut snapshot.panels[0], &mut snapshot.logs, "EC2 instances");
    let Some(instance_id) = selected_id(&snapshot.panels[0].items, selected) else {
        return snapshot;
    };
    let selected_instance = snapshot.panels[0]
        .items
        .iter()
        .find(|instance| instance.id == instance_id)
        .map(|instance| instance.detail.clone())
        .unwrap_or(Value::Null);

    let volume_command = vec![
        "ec2".into(),
        "describe-volumes".into(),
        "--filters".into(),
        format!("Name=attachment.instance-id,Values={instance_id}"),
    ];
    match cli
        .invoke(&mut snapshot.logs, &volume_command, true)
        .and_then(|response| volumes(&response))
    {
        Ok(items) => snapshot.panels[1].items = items,
        Err(error) => panel_error(&mut snapshot, 1, error),
    }
    sort_and_limit(&mut snapshot.panels[1], &mut snapshot.logs, "EC2 volumes");

    match security_group_ids(&selected_instance) {
        Ok(group_ids) if !group_ids.is_empty() => {
            let mut command = vec![
                "ec2".into(),
                "describe-security-groups".into(),
                "--group-ids".into(),
            ];
            command.extend(group_ids);
            match cli
                .invoke(&mut snapshot.logs, &command, true)
                .and_then(|response| security_groups(&response))
            {
                Ok(items) => snapshot.panels[2].items = items,
                Err(error) => panel_error(&mut snapshot, 2, error),
            }
        }
        Ok(_) => {}
        Err(error) => panel_error(&mut snapshot, 2, error),
    }
    sort_and_limit(
        &mut snapshot.panels[2],
        &mut snapshot.logs,
        "EC2 security groups",
    );

    match optional_array(&selected_instance, "Tags")
        .and_then(|tags| tag_resources(tags, "Key", "Value", ""))
    {
        Ok(items) => snapshot.panels[3].items = items,
        Err(error) => panel_error(&mut snapshot, 3, error),
    }
    sort_and_limit(&mut snapshot.panels[3], &mut snapshot.logs, "EC2 tags");
    snapshot
}

fn instances(response: &Value) -> Result<Vec<Resource>, String> {
    let mut items = Vec::new();
    for reservation in array(response, "Reservations")? {
        for instance in array(reservation, "Instances")? {
            let id = string(instance, "InstanceId")?;
            let state = instance
                .get("State")
                .ok_or("AWS response is missing `State`")?;
            let state = string(state, "Name")?;
            let instance_type = optional_string(instance, "InstanceType")?.unwrap_or("unknown");
            let name = optional_array(instance, "Tags")?
                .iter()
                .find(|tag| tag.get("Key").and_then(Value::as_str) == Some("Name"))
                .and_then(|tag| tag.get("Value"))
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .unwrap_or(id);
            items.push(resource(
                id,
                name,
                format!("{state} · {instance_type}"),
                instance.clone(),
            ));
        }
    }
    Ok(items)
}

fn volumes(response: &Value) -> Result<Vec<Resource>, String> {
    array(response, "Volumes")?
        .iter()
        .map(|volume| {
            let id = string(volume, "VolumeId")?;
            let state = optional_string(volume, "State")?.unwrap_or("unknown");
            let size = volume
                .get("Size")
                .and_then(Value::as_u64)
                .map(|value| format!("{value} GiB"))
                .unwrap_or_else(|| "size unknown".into());
            Ok(resource(
                id,
                id,
                format!("{state} · {size}"),
                volume.clone(),
            ))
        })
        .collect()
}

fn security_group_ids(instance: &Value) -> Result<Vec<String>, String> {
    let mut ids = optional_array(instance, "SecurityGroups")?
        .iter()
        .map(|group| string(group, "GroupId").map(str::to_string))
        .collect::<Result<Vec<_>, _>>()?;
    ids.sort();
    ids.dedup();
    Ok(ids)
}

fn security_groups(response: &Value) -> Result<Vec<Resource>, String> {
    array(response, "SecurityGroups")?
        .iter()
        .map(|group| {
            let id = string(group, "GroupId")?;
            let name = optional_string(group, "GroupName")?.unwrap_or(id);
            let description = optional_string(group, "Description")?.unwrap_or("");
            let vpc = optional_string(group, "VpcId")?.unwrap_or("");
            Ok(resource(
                id,
                name,
                [description, vpc]
                    .into_iter()
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
                    .join(" · "),
                group.clone(),
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_instances_and_limits_security_groups_to_the_selected_instance() {
        let response: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/aws_ec2.json")).unwrap();
        let items = instances(&response).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[1].id, "i-0001");
        assert_eq!(
            security_group_ids(&items[1].detail).unwrap(),
            vec!["sg-0001"]
        );
    }

    #[test]
    fn rejects_missing_instance_collection() {
        assert!(instances(&Value::Object(Default::default())).is_err());
    }
}
