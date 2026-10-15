use serde_json::json;

use super::*;

pub(super) fn available(panel: usize, resource: &Resource, primary: &Resource) -> Vec<Operation> {
    let id = resource.id.as_str();
    if id.is_empty() {
        return Vec::new();
    }

    match panel {
        0 => vec![operation(
            "Delete cluster",
            "ecs",
            "delete-cluster",
            json!({"cluster":id}),
            id,
            "Deletes this cluster. It must have no services or running tasks.",
            None,
        )],
        1 => {
            let name = string(&resource.detail, &["serviceName"]).unwrap_or(id);
            let cluster = primary.id.as_str();
            vec![
                operation(
                    "Start new deployment",
                    "ecs",
                    "update-service",
                    json!({"cluster":cluster,"service":name,"forceNewDeployment":true}),
                    id,
                    "Starts a new deployment. ECS replaces running tasks.",
                    None,
                ),
                operation(
                    "Set desired task count",
                    "ecs",
                    "update-service",
                    json!({"cluster":cluster,"service":name}),
                    id,
                    "Changes the service task count. ECS can start or stop tasks.",
                    Some("Desired task count"),
                ),
                operation(
                    "Delete service",
                    "ecs",
                    "delete-service",
                    json!({"cluster":cluster,"service":name,"force":false}),
                    id,
                    "Deletes this service without force. Its desired task count must be zero.",
                    None,
                ),
            ]
        }
        2 => {
            let status = string(&resource.detail, &["lastStatus", "LastStatus"]);
            if status.is_some_and(|status| !status.eq_ignore_ascii_case("stopped")) {
                vec![operation(
                    "Stop task",
                    "ecs",
                    "stop-task",
                    json!({"cluster":primary.id,"task":id}),
                    id,
                    "Stops this task. An ECS service may start a replacement task.",
                    None,
                )]
            } else {
                Vec::new()
            }
        }
        3 => {
            if string(&resource.detail, &["status", "Status"])
                .is_some_and(|status| status.eq_ignore_ascii_case("active"))
            {
                vec![operation(
                    "Deregister task definition",
                    "ecs",
                    "deregister-task-definition",
                    json!({"taskDefinition":id}),
                    id,
                    "Deregisters this revision. Running tasks keep using it.",
                    None,
                )]
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    }
}
