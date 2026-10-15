use serde_json::json;

use super::*;

pub(super) fn available(panel: usize, resource: &Resource, primary: &Resource) -> Vec<Operation> {
    let id = resource.id.as_str();
    if id.is_empty() {
        return Vec::new();
    }

    match panel {
        0 => {
            let mut actions = vec![operation(
                "Delete cluster",
                "ecs",
                "delete-cluster",
                json!({"cluster":id}),
                id,
                "Deletes this cluster. It must have no services or running tasks.",
                None,
            )];

            if string(&resource.detail, &["status"])
                .is_some_and(|status| status.eq_ignore_ascii_case("active"))
            {
                actions.push(operation(
                    "Update cluster settings",
                    "ecs",
                    "update-cluster-settings",
                    json!({"cluster":id}),
                    id,
                    "Enter a settings list. Container Insights accepts enabled, enhanced, or disabled; charges may apply.",
                    Some(Parameter::JsonFields),
                ));
            }

            actions.extend([
                operation(
                    "List services",
                    "ecs",
                    "list-services",
                    json!({"cluster":id}),
                    id,
                    "Lists service ARNs in this cluster.",
                    None,
                )
                .paginated(),
                operation(
                    "List tasks",
                    "ecs",
                    "list-tasks",
                    json!({"cluster":id}),
                    id,
                    "Lists RUNNING tasks by default; recently stopped tasks may also appear.",
                    None,
                )
                .paginated(),
            ]);

            actions
        }
        1 => {
            let name = string(&resource.detail, &["serviceName"]).unwrap_or(id);
            let cluster = primary.id.as_str();
            let mut count_input = json!({"cluster":cluster,"service":name});
            if let Some(count) = resource.detail.get("desiredCount").and_then(Value::as_u64) {
                count_input["desiredCount"] = json!(count);
            }

            let mut actions = vec![
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
                    count_input,
                    id,
                    "Changes the service task count. ECS can start or stop tasks.",
                    Some(Parameter::DesiredTaskCount),
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
            ];

            if nested_string(&resource.detail, &["deploymentController", "type"])
                .is_none_or(|controller| controller.eq_ignore_ascii_case("ecs"))
            {
                actions.extend([
                    operation(
                        "Deploy task definition revision",
                        "ecs",
                        "update-service",
                        json!({"cluster":cluster,"service":name}),
                        id,
                        "Starts a deployment with the supplied task definition revision. This does not roll back an ongoing deployment.",
                        Some(Parameter::TaskDefinitionRevision),
                    ),
                    operation(
                        "List service deployments",
                        "ecs",
                        "list-service-deployments",
                        json!({"cluster":cluster,"service":id}),
                        id,
                        "Lists this service's deployment history. Use a returned deployment ARN to roll back an ongoing deployment.",
                        None,
                    ),
                    operation(
                        "Rollback service deployment",
                        "ecs",
                        "stop-service-deployment",
                        json!({"stopType":"ROLLBACK"}),
                        id,
                        "Rolls an ongoing deployment back to its previous service revision. Get the deployment ARN from List service deployments.",
                        Some(Parameter::ServiceDeploymentArn),
                    ),
                ]);
            }

            actions
        }
        2 => {
            let status = string(&resource.detail, &["lastStatus", "LastStatus"]);
            let mut actions = Vec::new();

            if status.is_some_and(|status| !status.eq_ignore_ascii_case("stopped")) {
                actions.push(operation(
                    "Stop task",
                    "ecs",
                    "stop-task",
                    json!({"cluster":primary.id,"task":id}),
                    id,
                    "Stops this task. An ECS service may start a replacement task.",
                    None,
                ));
            }

            if let Some(definition) = string(&resource.detail, &["taskDefinitionArn"]) {
                let mut input = json!({"cluster":primary.id,"taskDefinition":definition});
                if let Some(launch_type) = string(&resource.detail, &["launchType"]) {
                    input["launchType"] = json!(launch_type);
                }
                actions.push(operation(
                    "Run task again",
                    "ecs",
                    "run-task",
                    input,
                    id,
                    "Starts a standalone task and may add compute charges. For awsvpc tasks, enter networkConfiguration with subnets; security groups are optional.",
                    Some(Parameter::JsonFields),
                ));
            }

            if let Some(definition) = string(&resource.detail, &["taskDefinitionArn"]) {
                actions.push(operation(
                    "View task definition",
                    "ecs",
                    "describe-task-definition",
                    json!({"taskDefinition":definition,"include":["TAGS"]}),
                    id,
                    "Shows this task's definition and tags. An inactive definition can be described only while an active task or service references it.",
                    None,
                ));
            }

            actions
        }
        3 => {
            let active = string(&resource.detail, &["status", "Status"])
                .is_some_and(|status| status.eq_ignore_ascii_case("active"));
            let mut actions = Vec::new();

            if active {
                actions.extend([
                    operation(
                        "Deregister task definition",
                        "ecs",
                        "deregister-task-definition",
                        json!({"taskDefinition":id}),
                        id,
                        "Deregisters this revision. Running tasks keep using it.",
                        None,
                    ),
                    operation(
                        "Run task",
                        "ecs",
                        "run-task",
                        json!({"cluster":primary.id,"taskDefinition":id}),
                        id,
                        "Starts a standalone task and may add compute charges. Set launchType or capacityProviderStrategy if needed; awsvpc tasks need networkConfiguration with subnets. Security groups are optional.",
                        Some(Parameter::JsonFields),
                    ),
                ]);
            }

            if let Some(family) = string(&resource.detail, &["family", "Family"]) {
                actions.push(operation(
                    "List task definition revisions",
                    "ecs",
                    "list-task-definitions",
                    json!({"familyPrefix":family}),
                    id,
                    "Lists active revisions by family prefix. The prefix can also match similarly named families.",
                    None,
                ).paginated());
            }

            actions.push(operation(
                "View task definition (include TAGS)",
                "ecs",
                "describe-task-definition",
                json!({"taskDefinition":id,"include":["TAGS"]}),
                id,
                "Shows this revision and its tags. An inactive definition can be described only while an active task or service references it.",
                None,
            ));

            actions
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
#[path = "ecs_tests.rs"]
mod tests;
