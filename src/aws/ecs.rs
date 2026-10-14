use serde_json::{Value, json};

use crate::{
    config::{ECS_CLUSTERS_BATCH, ECS_SERVICES_BATCH, ECS_TASKS_BATCH},
    model::{Resource, Snapshot},
};

use super::{
    FetchContext,
    common::{
        array, optional_array, optional_string, panel_error, resource, selected_id, sort_and_limit,
        string,
    },
};

type ParsedResources = (Vec<Resource>, Vec<String>, Vec<String>);

pub(super) fn fetch(cli: &FetchContext<'_>, selected: Option<&str>) -> Snapshot {
    let mut snapshot = Snapshot::empty(crate::model::Service::Ecs);
    let response = match cli.invoke(
        &mut snapshot.logs,
        &["ecs".into(), "list-clusters".into()],
        true,
    ) {
        Ok(response) => response,
        Err(error) => {
            panel_error(&mut snapshot, 0, error);
            return snapshot;
        }
    };

    match cluster_arns(&response) {
        Ok(arns) => {
            snapshot.panels[0].items = arns
                .into_iter()
                .map(|arn| {
                    let name = arn.rsplit('/').next().unwrap_or(&arn);
                    resource(
                        arn.clone(),
                        name,
                        arn.clone(),
                        json!({"clusterArn": arn, "clusterName": name}),
                    )
                })
                .collect();
        }
        Err(error) => panel_error(&mut snapshot, 0, error),
    }
    sort_and_limit(&mut snapshot.panels[0], &mut snapshot.logs, "ECS clusters");
    describe_clusters(cli, &mut snapshot);

    let Some(cluster_arn) = selected_id(&snapshot.panels[0].items, selected) else {
        return snapshot;
    };

    let service_arns = list_arns(
        cli,
        &mut snapshot,
        1,
        vec![
            "ecs".into(),
            "list-services".into(),
            "--cluster".into(),
            cluster_arn.clone(),
        ],
        "serviceArns",
        "ECS services",
    );
    let mut task_definition_arns = Vec::new();
    if let Some(arns) = service_arns {
        for batch in arns.chunks(ECS_SERVICES_BATCH) {
            if refresh_expired(cli, &mut snapshot, 1, "service descriptions") {
                break;
            }
            let mut command = vec![
                "ecs".into(),
                "describe-services".into(),
                "--cluster".into(),
                cluster_arn.clone(),
                "--services".into(),
            ];
            command.extend(batch.iter().cloned());
            match cli
                .invoke(&mut snapshot.logs, &command, false)
                .and_then(|response| services(&response))
            {
                Ok((items, definitions, failures)) => {
                    snapshot.panels[1].items.extend(items);
                    task_definition_arns.extend(definitions);
                    for failure in failures {
                        panel_error(&mut snapshot, 1, failure);
                    }
                }
                Err(error) => panel_error(&mut snapshot, 1, error),
            }
        }
    }
    sort_and_limit(&mut snapshot.panels[1], &mut snapshot.logs, "ECS services");

    let mut task_arns = Vec::new();
    for desired_status in ["RUNNING", "STOPPED"] {
        let task_list = list_arns(
            cli,
            &mut snapshot,
            2,
            vec![
                "ecs".into(),
                "list-tasks".into(),
                "--cluster".into(),
                cluster_arn.clone(),
                "--desired-status".into(),
                desired_status.into(),
            ],
            "taskArns",
            "ECS tasks",
        );
        if let Some(arns) = task_list {
            task_arns.extend(arns);
        }
    }
    task_arns.sort();
    task_arns.dedup();
    if task_arns.len() > crate::config::MAX_ITEMS {
        let total = task_arns.len();
        task_arns.truncate(crate::config::MAX_ITEMS);
        snapshot.logs.push(format!(
            "notice: ECS tasks were limited to {} of {total} items; results are incomplete",
            crate::config::MAX_ITEMS
        ));
    }
    for batch in task_arns.chunks(ECS_TASKS_BATCH) {
        if refresh_expired(cli, &mut snapshot, 2, "task descriptions") {
            break;
        }
        let mut command = vec![
            "ecs".into(),
            "describe-tasks".into(),
            "--cluster".into(),
            cluster_arn.clone(),
            "--tasks".into(),
        ];
        command.extend(batch.iter().cloned());
        match cli
            .invoke(&mut snapshot.logs, &command, false)
            .and_then(|response| tasks(&response))
        {
            Ok((items, definitions, failures)) => {
                snapshot.panels[2].items.extend(items);
                task_definition_arns.extend(definitions);
                for failure in failures {
                    panel_error(&mut snapshot, 2, failure);
                }
            }
            Err(error) => panel_error(&mut snapshot, 2, error),
        }
    }
    sort_and_limit(&mut snapshot.panels[2], &mut snapshot.logs, "ECS tasks");

    task_definition_arns.sort();
    task_definition_arns.dedup();
    if task_definition_arns.len() > crate::config::MAX_ITEMS {
        let total = task_definition_arns.len();
        task_definition_arns.truncate(crate::config::MAX_ITEMS);
        snapshot.logs.push(format!(
            "notice: ECS task definitions were limited to {} of {total} items; results are incomplete",
            crate::config::MAX_ITEMS
        ));
    }
    for arn in task_definition_arns {
        if refresh_expired(cli, &mut snapshot, 3, "task definitions") {
            break;
        }
        let command = vec![
            "ecs".into(),
            "describe-task-definition".into(),
            "--task-definition".into(),
            arn,
        ];
        match cli
            .invoke(&mut snapshot.logs, &command, false)
            .and_then(|response| task_definition(&response))
        {
            Ok(item) => snapshot.panels[3].items.push(item),
            Err(error) => panel_error(&mut snapshot, 3, error),
        }
    }
    sort_and_limit(
        &mut snapshot.panels[3],
        &mut snapshot.logs,
        "ECS task definitions",
    );
    snapshot
}

fn cluster_arns(response: &Value) -> Result<Vec<String>, String> {
    array(response, "clusterArns")?
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .ok_or("AWS response cluster ARN must be a string".into())
        })
        .collect()
}

fn describe_clusters(cli: &FetchContext<'_>, snapshot: &mut Snapshot) {
    let arns = snapshot.panels[0]
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    for batch in arns.chunks(ECS_CLUSTERS_BATCH) {
        if refresh_expired(cli, snapshot, 0, "cluster details") {
            break;
        }
        let mut command = vec![
            "ecs".into(),
            "describe-clusters".into(),
            "--clusters".into(),
        ];
        command.extend(batch.iter().cloned());
        match cli
            .invoke(&mut snapshot.logs, &command, false)
            .and_then(|response| clusters(&response))
        {
            Ok((items, failures)) => {
                for described in items {
                    if let Some(item) = snapshot.panels[0]
                        .items
                        .iter_mut()
                        .find(|item| item.id == described.id)
                    {
                        *item = described;
                    }
                }
                for failure in failures {
                    panel_error(snapshot, 0, failure);
                }
            }
            Err(error) => panel_error(snapshot, 0, error),
        }
    }
    sort_and_limit(&mut snapshot.panels[0], &mut snapshot.logs, "ECS clusters");
}

fn clusters(response: &Value) -> Result<(Vec<Resource>, Vec<String>), String> {
    let items = array(response, "clusters")?
        .iter()
        .map(|cluster| {
            let arn = string(cluster, "clusterArn")?;
            let name = optional_string(cluster, "clusterName")?.unwrap_or(arn);
            let status = optional_string(cluster, "status")?.unwrap_or("unknown");
            Ok(resource(arn, name, status, cluster.clone()))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok((items, failures(response)?))
}

fn list_arns(
    cli: &FetchContext<'_>,
    snapshot: &mut Snapshot,
    panel: usize,
    command: Vec<String>,
    field: &str,
    name: &str,
) -> Option<Vec<String>> {
    let parsed = cli
        .invoke(&mut snapshot.logs, &command, true)
        .and_then(|response| {
            array(&response, field)?
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .filter(|value| !value.is_empty())
                        .map(str::to_string)
                        .ok_or_else(|| format!("AWS response `{field}` entries must be strings"))
                })
                .collect()
        });
    match parsed {
        Ok(arns) => Some(arns),
        Err(error) => {
            panel_error(snapshot, panel, format!("{name}: {error}"));
            None
        }
    }
}

fn services(response: &Value) -> Result<ParsedResources, String> {
    let mut items = Vec::new();
    let mut definitions = Vec::new();
    for service in array(response, "services")? {
        let arn = string(service, "serviceArn")?;
        let name = optional_string(service, "serviceName")?.unwrap_or(arn);
        let status = optional_string(service, "status")?.unwrap_or("unknown");
        definitions.push(string(service, "taskDefinition")?.into());
        items.push(resource(arn, name, status, service.clone()));
    }
    Ok((items, definitions, failures(response)?))
}

fn tasks(response: &Value) -> Result<ParsedResources, String> {
    let mut items = Vec::new();
    let mut definitions = Vec::new();
    for task in array(response, "tasks")? {
        let arn = string(task, "taskArn")?;
        let status = optional_string(task, "lastStatus")?.unwrap_or("unknown");
        let definition = string(task, "taskDefinitionArn")?;
        definitions.push(definition.into());
        items.push(resource(
            arn,
            arn.rsplit('/').next().unwrap_or(arn),
            format!("{status} · {definition}"),
            task.clone(),
        ));
    }
    Ok((items, definitions, failures(response)?))
}

fn task_definition(response: &Value) -> Result<Resource, String> {
    let definition = response
        .get("taskDefinition")
        .ok_or("AWS response is missing `taskDefinition`")?;
    let arn = string(definition, "taskDefinitionArn")?;
    let family = optional_string(definition, "family")?.unwrap_or(arn);
    let revision = definition
        .get("revision")
        .and_then(Value::as_u64)
        .map(|revision| revision.to_string())
        .unwrap_or_default();
    Ok(resource(
        arn,
        family,
        if revision.is_empty() {
            String::new()
        } else {
            format!("revision {revision}")
        },
        definition.clone(),
    ))
}

fn failures(response: &Value) -> Result<Vec<String>, String> {
    optional_array(response, "failures")?
        .iter()
        .map(|failure| {
            let arn = optional_string(failure, "arn")?.unwrap_or("resource");
            let reason = optional_string(failure, "reason")?.unwrap_or("unknown reason");
            Ok(format!("{arn}: {reason}"))
        })
        .collect()
}

fn refresh_expired(
    cli: &FetchContext<'_>,
    snapshot: &mut Snapshot,
    panel: usize,
    pending: &str,
) -> bool {
    if !cli.expired() {
        return false;
    }
    cli.log_timeout_once(&mut snapshot.logs);
    panel_error(
        snapshot,
        panel,
        format!("AWS refresh timed out; remaining {pending} were skipped"),
    );
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_services_tasks_and_related_task_definitions() {
        let response: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/aws_ecs.json")).unwrap();
        let (service_items, service_defs, _) = services(&response["serviceResponse"]).unwrap();
        let (task_items, task_defs, _) = tasks(&response["taskResponse"]).unwrap();
        assert_eq!(service_items[0].id, "service-a");
        assert_eq!(task_items[0].id, "task-a");
        assert_eq!(service_defs[0], task_defs[0]);
    }

    #[test]
    fn rejects_a_missing_tasks_array() {
        assert!(tasks(&json!({})).is_err());
    }
}
