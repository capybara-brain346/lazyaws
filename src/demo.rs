use serde_json::{Value, json};

use crate::model::{Resource, Service, Snapshot};

pub fn snapshot(service: Service, selected: Option<&str>) -> Snapshot {
    let items = match service {
        Service::Ec2 => ec2(selected),
        Service::Ecs => ecs(selected),
        Service::Ecr => ecr(selected),
        Service::S3 => s3(selected),
        Service::Lambda => lambda(selected),
    };
    let mut snapshot = Snapshot::empty(service);
    for (panel, items) in snapshot.panels.iter_mut().zip(items) {
        panel.items = items;
    }
    snapshot.logs = vec![format!("{} demo data loaded", service.name())];
    snapshot
}

fn resource(id: &str, label: &str, summary: &str, detail: Value) -> Resource {
    Resource {
        id: id.into(),
        label: label.into(),
        summary: summary.into(),
        detail,
    }
}

fn selected_id<'a>(parents: &'a [Resource], selected: Option<&str>) -> &'a str {
    parents
        .iter()
        .find(|parent| Some(parent.id.as_str()) == selected)
        .or_else(|| parents.first())
        .map_or("", |parent| parent.id.as_str())
}

fn ec2(selected: Option<&str>) -> [Vec<Resource>; 4] {
    let prod = "i-0a12bc34de56f7890";
    let worker = "i-0b98fe76dc54a3210";
    let parents = vec![
        resource(
            prod,
            "api-prod-1",
            "running · t3.medium · us-east-1a",
            json!({"instanceId":prod,"instanceType":"t3.medium","state":"running","availabilityZone":"us-east-1a","privateIpAddress":"10.0.1.24","publicIpAddress":"203.0.113.24","launchTime":"2025-04-12T09:18:00Z","tags":{"Name":"api-prod-1","Environment":"production"}}),
        ),
        resource(
            worker,
            "batch-worker-1",
            "running · m6i.large · us-east-1b",
            json!({"instanceId":worker,"instanceType":"m6i.large","state":"running","availabilityZone":"us-east-1b","privateIpAddress":"10.0.2.37","launchTime":"2025-06-03T14:02:00Z","tags":{"Name":"batch-worker-1","Environment":"production"}}),
        ),
    ];
    let id = selected_id(&parents, selected);
    let volumes = if id == worker {
        vec![resource(
            "vol-0c1234abcd5678ef0",
            "worker-root",
            "gp3 · 40 GiB · attached",
            json!({"volumeId":"vol-0c1234abcd5678ef0","size":40,"type":"gp3","state":"in-use","context":id}),
        )]
    } else {
        vec![
            resource(
                "vol-0a1234abcd5678ef0",
                "api-root",
                "gp3 · 24 GiB · attached",
                json!({"volumeId":"vol-0a1234abcd5678ef0","size":24,"type":"gp3","state":"in-use","context":id}),
            ),
            resource(
                "vol-0a2345bcde6789f01",
                "api-data",
                "gp3 · 80 GiB · attached",
                json!({"volumeId":"vol-0a2345bcde6789f01","size":80,"type":"gp3","state":"in-use","context":id}),
            ),
        ]
    };
    let groups = vec![resource(
        "sg-0123abcd4567ef890",
        "app-private",
        "vpc-0a1b2c3d · 2 inbound rules",
        json!({"groupId":"sg-0123abcd4567ef890","vpcId":"vpc-0a1b2c3d","context":id,"rules":[{"protocol":"tcp","fromPort":443,"toPort":443,"source":"10.0.0.0/16"},{"protocol":"tcp","fromPort":8080,"toPort":8080,"source":"10.0.0.0/16"}]}),
    )];
    let tags = vec![
        resource(
            "Environment",
            "Environment",
            "production",
            json!({"key":"Environment","value":"production","context":id}),
        ),
        resource(
            "Name",
            "Name",
            if id == worker {
                "batch-worker-1"
            } else {
                "api-prod-1"
            },
            json!({"key":"Name","value":if id == worker { "batch-worker-1" } else { "api-prod-1" },"context":id}),
        ),
    ];
    [parents, volumes, groups, tags]
}

fn ecs(selected: Option<&str>) -> [Vec<Resource>; 4] {
    let prod = "arn:aws:ecs:us-east-1:123456789012:cluster/production";
    let staging = "arn:aws:ecs:us-east-1:123456789012:cluster/staging";
    let parents = vec![
        resource(
            prod,
            "production",
            "ACTIVE · 3 services · Fargate",
            json!({"clusterArn":prod,"status":"ACTIVE","runningTasksCount":4,"pendingTasksCount":0,"capacityProviders":["FARGATE","FARGATE_SPOT"]}),
        ),
        resource(
            staging,
            "staging",
            "ACTIVE · 1 service · Fargate",
            json!({"clusterArn":staging,"status":"ACTIVE","runningTasksCount":1,"pendingTasksCount":0,"capacityProviders":["FARGATE"]}),
        ),
    ];
    let id = selected_id(&parents, selected);
    let (services, tasks, definitions) = if id == staging {
        (
            vec![resource(
                "arn:aws:ecs:us-east-1:123456789012:service/staging/web",
                "web",
                "1/1 running · Fargate",
                json!({"serviceName":"web","desiredCount":1,"runningCount":1,"launchType":"FARGATE","context":id}),
            )],
            vec![resource(
                "task-staging-7f1a",
                "web · RUNNING",
                "Fargate · started 18 minutes ago",
                json!({"taskArn":"arn:aws:ecs:us-east-1:123456789012:task/staging/7f1a","taskDefinitionArn":"arn:aws:ecs:us-east-1:123456789012:task-definition/web:12","launchType":"FARGATE","lastStatus":"RUNNING","cpu":"256","memory":"512","context":id}),
            )],
            vec![resource(
                "web:12",
                "web:12",
                "ACTIVE · linux/amd64",
                json!({"family":"web","revision":12,"status":"ACTIVE","context":id}),
            )],
        )
    } else {
        (
            vec![
                resource(
                    "arn:aws:ecs:us-east-1:123456789012:service/production/api",
                    "api",
                    "2/2 running · Fargate",
                    json!({"serviceName":"api","desiredCount":2,"runningCount":2,"launchType":"FARGATE","context":id}),
                ),
                resource(
                    "arn:aws:ecs:us-east-1:123456789012:service/production/worker",
                    "worker",
                    "2/2 running · Fargate Spot",
                    json!({"serviceName":"worker","desiredCount":2,"runningCount":2,"launchType":"FARGATE","capacityProvider":"FARGATE_SPOT","context":id}),
                ),
            ],
            vec![
                resource(
                    "task-api-45d2",
                    "api · RUNNING",
                    "Fargate · started 6 minutes ago",
                    json!({"taskArn":"arn:aws:ecs:us-east-1:123456789012:task/production/45d2","taskDefinitionArn":"arn:aws:ecs:us-east-1:123456789012:task-definition/api:28","launchType":"FARGATE","lastStatus":"RUNNING","cpu":"512","memory":"1024","context":id}),
                ),
                resource(
                    "task-worker-a913",
                    "worker · RUNNING",
                    "Fargate Spot · started 11 minutes ago",
                    json!({"taskArn":"arn:aws:ecs:us-east-1:123456789012:task/production/a913","taskDefinitionArn":"arn:aws:ecs:us-east-1:123456789012:task-definition/worker:9","launchType":"FARGATE","lastStatus":"RUNNING","cpu":"1024","memory":"2048","context":id}),
                ),
            ],
            vec![
                resource(
                    "api:28",
                    "api:28",
                    "ACTIVE · linux/amd64",
                    json!({"family":"api","revision":28,"status":"ACTIVE","context":id}),
                ),
                resource(
                    "worker:9",
                    "worker:9",
                    "ACTIVE · linux/amd64",
                    json!({"family":"worker","revision":9,"status":"ACTIVE","context":id}),
                ),
            ],
        )
    };
    [parents, services, tasks, definitions]
}

fn ecr(selected: Option<&str>) -> [Vec<Resource>; 4] {
    let api = "arn:aws:ecr:us-east-1:123456789012:repository/api";
    let worker = "arn:aws:ecr:us-east-1:123456789012:repository/worker";
    let parents = vec![
        resource(
            api,
            "api",
            "private · scan on push · AES-256",
            json!({"repositoryName":"api","repositoryArn":api,"repositoryUri":"123456789012.dkr.ecr.us-east-1.amazonaws.com/api","imageTagMutability":"IMMUTABLE","imageScanningConfiguration":{"scanOnPush":true}}),
        ),
        resource(
            worker,
            "worker",
            "private · scan on push · AES-256",
            json!({"repositoryName":"worker","repositoryArn":worker,"repositoryUri":"123456789012.dkr.ecr.us-east-1.amazonaws.com/worker","imageTagMutability":"MUTABLE","imageScanningConfiguration":{"scanOnPush":true}}),
        ),
    ];
    let id = selected_id(&parents, selected);
    let (images, tags, repository_tags) = if id == worker {
        (
            vec![resource(
                "sha256:4c23...a781",
                "sha256:4c23...a781",
                "pushed 2 days ago · 184 MB",
                json!({"imageDigest":"sha256:4c23d8a81f09","imageSizeInBytes":192937984,"imagePushedAt":"2025-10-12T12:40:00Z","context":id}),
            )],
            vec![
                resource(
                    "latest",
                    "latest",
                    "sha256:4c23...a781",
                    json!({"imageTag":"latest","imageDigest":"sha256:4c23d8a81f09","context":id}),
                ),
                resource(
                    "v2.4.1",
                    "v2.4.1",
                    "sha256:4c23...a781",
                    json!({"imageTag":"v2.4.1","imageDigest":"sha256:4c23d8a81f09","context":id}),
                ),
            ],
            vec![resource(
                "Team",
                "Team",
                "platform",
                json!({"key":"Team","value":"platform","context":id}),
            )],
        )
    } else {
        (
            vec![
                resource(
                    "sha256:9a10...be42",
                    "sha256:9a10...be42",
                    "pushed 4 hours ago · 96 MB",
                    json!({"imageDigest":"sha256:9a10f30e6642","imageSizeInBytes":100663296,"imagePushedAt":"2025-10-14T07:12:00Z","context":id}),
                ),
                resource(
                    "sha256:103e...c512",
                    "sha256:103e...c512",
                    "pushed 2 days ago · 95 MB",
                    json!({"imageDigest":"sha256:103e98d8ad12","imageSizeInBytes":99614720,"imagePushedAt":"2025-10-12T16:25:00Z","context":id}),
                ),
            ],
            vec![
                resource(
                    "v3.8.0",
                    "v3.8.0",
                    "sha256:9a10...be42",
                    json!({"imageTag":"v3.8.0","imageDigest":"sha256:9a10f30e6642","context":id}),
                ),
                resource(
                    "stable",
                    "stable",
                    "sha256:9a10...be42",
                    json!({"imageTag":"stable","imageDigest":"sha256:9a10f30e6642","context":id}),
                ),
            ],
            vec![
                resource(
                    "Team",
                    "Team",
                    "commerce",
                    json!({"key":"Team","value":"commerce","context":id}),
                ),
                resource(
                    "Environment",
                    "Environment",
                    "production",
                    json!({"key":"Environment","value":"production","context":id}),
                ),
            ],
        )
    };
    [parents, images, tags, repository_tags]
}

fn s3(selected: Option<&str>) -> [Vec<Resource>; 4] {
    let assets = "acme-prod-assets";
    let logs = "acme-prod-logs";
    let parents = vec![
        resource(
            assets,
            "acme-prod-assets",
            "us-east-1 · versioned · SSE-S3",
            json!({"name":assets,"region":"us-east-1","creationDate":"2023-08-17T10:15:00Z","versioning":"Enabled","encryption":"AES256"}),
        ),
        resource(
            logs,
            "acme-prod-logs",
            "us-east-1 · versioned · SSE-KMS",
            json!({"name":logs,"region":"us-east-1","creationDate":"2024-01-22T08:40:00Z","versioning":"Enabled","encryption":"aws:kms"}),
        ),
    ];
    let id = selected_id(&parents, selected);
    let (objects, prefixes, settings) = if id == logs {
        (
            vec![resource(
                "2025/10/14/api-12.log.gz",
                "2025/10/14/api-12.log.gz",
                "Standard · 18.4 MB · modified 09:32",
                json!({"key":"2025/10/14/api-12.log.gz","size":19293798,"storageClass":"STANDARD","lastModified":"2025-10-14T09:32:00Z","context":id}),
            )],
            vec![resource(
                "2025/",
                "2025/",
                "3 child prefixes",
                json!({"prefix":"2025/","context":id}),
            )],
            vec![
                resource(
                    "encryption",
                    "Encryption",
                    "aws:kms · alias/log-archive",
                    json!({"algorithm":"aws:kms","keyId":"alias/log-archive","context":id}),
                ),
                resource(
                    "versioning",
                    "Versioning",
                    "Enabled",
                    json!({"status":"Enabled","context":id}),
                ),
            ],
        )
    } else {
        (
            vec![
                resource(
                    "images/logo.svg",
                    "images/logo.svg",
                    "Standard · 14 KB · modified 2 days ago",
                    json!({"key":"images/logo.svg","size":14336,"storageClass":"STANDARD","contentType":"image/svg+xml","context":id}),
                ),
                resource(
                    "docs/guide.pdf",
                    "docs/guide.pdf",
                    "Standard · 2.1 MB · modified 8 days ago",
                    json!({"key":"docs/guide.pdf","size":2202009,"storageClass":"STANDARD","contentType":"application/pdf","context":id}),
                ),
            ],
            vec![
                resource(
                    "images/",
                    "images/",
                    "18 objects",
                    json!({"prefix":"images/","context":id}),
                ),
                resource(
                    "docs/",
                    "docs/",
                    "6 objects",
                    json!({"prefix":"docs/","context":id}),
                ),
            ],
            vec![
                resource(
                    "encryption",
                    "Encryption",
                    "SSE-S3 · AES-256",
                    json!({"algorithm":"AES256","context":id}),
                ),
                resource(
                    "versioning",
                    "Versioning",
                    "Enabled",
                    json!({"status":"Enabled","context":id}),
                ),
            ],
        )
    };
    [parents, objects, prefixes, settings]
}

fn lambda(selected: Option<&str>) -> [Vec<Resource>; 4] {
    let api = "arn:aws:lambda:us-east-1:123456789012:function:api-handler";
    let images = "arn:aws:lambda:us-east-1:123456789012:function:image-processor";
    let parents = vec![
        resource(
            api,
            "api-handler",
            "python3.12 · 512 MB · 30s",
            json!({"FunctionName":"api-handler","FunctionArn":api,"Runtime":"python3.12","Handler":"app.handler","MemorySize":512,"Timeout":30,"LastModified":"2025-10-13T11:20:00Z","State":"Active"}),
        ),
        resource(
            images,
            "image-processor",
            "nodejs22.x · 1024 MB · 60s",
            json!({"FunctionName":"image-processor","FunctionArn":images,"Runtime":"nodejs22.x","Handler":"index.handler","MemorySize":1024,"Timeout":60,"LastModified":"2025-10-14T08:05:00Z","State":"Active"}),
        ),
    ];
    let id = selected_id(&parents, selected);
    let (versions, aliases, tags) = if id == images {
        (
            vec![
                resource(
                    &format!("{id}:5"),
                    "v5",
                    "nodejs22.x · 2025-10-14T08:05:00Z",
                    json!({"FunctionArn":format!("{id}:5"),"Version":"5","Runtime":"nodejs22.x","LastModified":"2025-10-14T08:05:00Z","context":id}),
                ),
                resource(
                    &format!("{id}:$LATEST"),
                    "$LATEST",
                    "nodejs22.x · 2025-10-14T08:12:00Z",
                    json!({"FunctionArn":format!("{id}:$LATEST"),"Version":"$LATEST","Runtime":"nodejs22.x","LastModified":"2025-10-14T08:12:00Z","context":id}),
                ),
            ],
            vec![resource(
                &format!("{id}:prod"),
                "prod",
                "-> 5 · live traffic",
                json!({"AliasArn":format!("{id}:prod"),"Name":"prod","FunctionVersion":"5","Description":"live traffic","context":id}),
            )],
            vec![
                resource(
                    "Environment",
                    "Environment",
                    "production",
                    json!({"Key":"Environment","Value":"production","context":id}),
                ),
                resource(
                    "Team",
                    "Team",
                    "platform",
                    json!({"Key":"Team","Value":"platform","context":id}),
                ),
            ],
        )
    } else {
        (
            vec![
                resource(
                    &format!("{id}:3"),
                    "v3",
                    "python3.12 · 2025-10-13T11:20:00Z",
                    json!({"FunctionArn":format!("{id}:3"),"Version":"3","Runtime":"python3.12","LastModified":"2025-10-13T11:20:00Z","context":id}),
                ),
                resource(
                    &format!("{id}:2"),
                    "v2",
                    "python3.12 · 2025-10-11T16:44:00Z",
                    json!({"FunctionArn":format!("{id}:2"),"Version":"2","Runtime":"python3.12","LastModified":"2025-10-11T16:44:00Z","context":id}),
                ),
            ],
            vec![
                resource(
                    &format!("{id}:prod"),
                    "prod",
                    "-> 3 · live traffic",
                    json!({"AliasArn":format!("{id}:prod"),"Name":"prod","FunctionVersion":"3","Description":"live traffic","context":id}),
                ),
                resource(
                    &format!("{id}:staging"),
                    "staging",
                    "-> 3 · canary 10%",
                    json!({"AliasArn":format!("{id}:staging"),"Name":"staging","FunctionVersion":"3","Description":"canary 10%","context":id}),
                ),
            ],
            vec![
                resource(
                    "Environment",
                    "Environment",
                    "production",
                    json!({"Key":"Environment","Value":"production","context":id}),
                ),
                resource(
                    "Team",
                    "Team",
                    "commerce",
                    json!({"Key":"Team","Value":"commerce","context":id}),
                ),
            ],
        )
    };
    [parents, versions, aliases, tags]
}

#[cfg(test)]
mod tests {
    use super::snapshot;
    use crate::model::Service;

    #[test]
    fn every_demo_service_has_two_parents_and_contextual_related_data() {
        for service in Service::ALL {
            let initial = snapshot(service, None);
            assert_eq!(
                initial.panels[0].items.len(),
                2,
                "{} parents",
                service.name()
            );
            let second_id = initial.panels[0].items[1].id.clone();
            let contextual = snapshot(service, Some(&second_id));
            for panel in &contextual.panels[1..] {
                assert!(
                    !panel.items.is_empty(),
                    "{} {} has no demo data",
                    service.name(),
                    panel.title
                );
                assert_eq!(panel.items[0].detail["context"], second_id);
            }
        }
    }
}
