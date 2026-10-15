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
fn cluster_settings_action_requires_an_active_cluster_and_keeps_cluster_fixed() {
    let cluster = resource(
        "arn:aws:ecs:ap-south-1:123456789012:cluster/prod",
        json!({"status":"ACTIVE"}),
    );
    let actions = available(0, &cluster, &cluster);

    assert_eq!(actions.len(), 4);
    assert_eq!(actions[0].name, "delete-cluster");
    assert_eq!(actions[1].name, "update-cluster-settings");
    assert_eq!(actions[1].parameter, Some(Parameter::JsonFields));
    assert_eq!(actions[1].input["cluster"], cluster.id);
    assert_eq!(actions[2].name, "list-services");
    assert_eq!(actions[2].input["cluster"], cluster.id);
    assert_eq!(actions[3].name, "list-tasks");
    assert_eq!(actions[3].input["cluster"], cluster.id);

    let inactive = resource(&cluster.id, json!({"status":"INACTIVE"}));
    let inactive_actions = available(0, &inactive, &inactive);
    assert_eq!(
        inactive_actions
            .iter()
            .map(|action| action.name)
            .collect::<Vec<_>>(),
        ["delete-cluster", "list-services", "list-tasks"]
    );
}

#[test]
fn service_actions_capture_the_cluster_and_service_for_deployments_and_rollback() {
    let cluster = resource(
        "arn:aws:ecs:ap-south-1:123456789012:cluster/prod",
        json!({}),
    );
    let service = resource(
        "arn:aws:ecs:ap-south-1:123456789012:service/prod/api",
        json!({"serviceName":"api"}),
    );
    let actions = available(1, &service, &cluster);

    assert_eq!(actions[0].name, "update-service");
    assert_eq!(actions[2].input["force"], false);

    let deploy = &actions[3];
    assert_eq!(deploy.parameter, Some(Parameter::TaskDefinitionRevision));
    assert_eq!(deploy.input["cluster"], cluster.id);
    assert_eq!(deploy.input["service"], "api");

    let list = &actions[4];
    assert_eq!(list.name, "list-service-deployments");
    assert_eq!(list.input["cluster"], cluster.id);
    assert_eq!(list.input["service"], service.id);

    let rollback = &actions[5];
    assert_eq!(rollback.parameter, Some(Parameter::ServiceDeploymentArn));
    assert_eq!(rollback.target, service.id);
    assert_eq!(rollback.input["stopType"], "ROLLBACK");

    let code_deploy = resource(
        &service.id,
        json!({"serviceName":"api","deploymentController":{"type":"CODE_DEPLOY"}}),
    );
    assert_eq!(available(1, &code_deploy, &cluster).len(), 3);
}

#[test]
fn task_actions_keep_the_stop_gate_and_capture_run_task_context() {
    let cluster = resource("cluster-id", json!({}));
    let task = resource(
        "task-id",
        json!({
            "lastStatus":"RUNNING",
            "taskDefinitionArn":"arn:aws:ecs:ap-south-1:123456789012:task-definition/api:4",
            "launchType":"FARGATE"
        }),
    );
    let actions = available(2, &task, &cluster);

    assert_eq!(
        actions.iter().map(|action| action.name).collect::<Vec<_>>(),
        ["stop-task", "run-task", "describe-task-definition"]
    );
    let run = &actions[1];
    assert_eq!(run.parameter, Some(Parameter::JsonFields));
    assert_eq!(run.input["cluster"], cluster.id);
    assert_eq!(
        run.input["taskDefinition"],
        "arn:aws:ecs:ap-south-1:123456789012:task-definition/api:4"
    );
    assert_eq!(run.input["launchType"], "FARGATE");
    assert_eq!(
        actions[2].input["taskDefinition"],
        run.input["taskDefinition"]
    );
    assert_eq!(actions[2].input["include"], json!(["TAGS"]));

    let stopped = resource(
        &task.id,
        json!({
            "lastStatus":"STOPPED",
            "taskDefinitionArn":"arn:aws:ecs:ap-south-1:123456789012:task-definition/api:4"
        }),
    );
    let stopped_actions = available(2, &stopped, &cluster);
    assert_eq!(stopped_actions.len(), 2);
    assert_eq!(stopped_actions[0].name, "run-task");
    assert_eq!(stopped_actions[1].name, "describe-task-definition");
}

#[test]
fn task_definition_actions_keep_status_gates_and_offer_reads() {
    let cluster = resource("cluster-id", json!({}));
    let definition = resource(
        "arn:aws:ecs:ap-south-1:123456789012:task-definition/api:4",
        json!({"status":"ACTIVE","family":"api"}),
    );
    let actions = available(3, &definition, &cluster);

    assert_eq!(actions.len(), 4);
    assert_eq!(actions[0].name, "deregister-task-definition");
    assert_eq!(actions[1].name, "run-task");
    assert_eq!(actions[1].parameter, Some(Parameter::JsonFields));
    assert_eq!(actions[1].input["cluster"], cluster.id);
    assert_eq!(actions[1].input["taskDefinition"], definition.id);
    assert_eq!(actions[2].name, "list-task-definitions");
    assert_eq!(actions[2].input["familyPrefix"], "api");
    assert_eq!(actions[3].name, "describe-task-definition");
    assert_eq!(actions[3].input["taskDefinition"], definition.id);
    assert_eq!(actions[3].input["include"], json!(["TAGS"]));

    let inactive = resource(&definition.id, json!({"status":"INACTIVE","family":"api"}));
    let inactive_actions = available(3, &inactive, &cluster);
    assert_eq!(
        inactive_actions
            .iter()
            .map(|action| action.name)
            .collect::<Vec<_>>(),
        ["list-task-definitions", "describe-task-definition"]
    );
}
