# AWS CLI action inventory

Research date: 2026-10-15. Local command index: AWS CLI 2.38.0.

The five services are EC2, ECS, ECR, S3, and Lambda. The sections below map commands
to the 20 resource tabs. Status has these meanings:

- **Action**: available from the selected row, subject to resource state.
- **Read**: used to load a pane or available as a read action.
- **CLI-only**: available in AWS CLI; not available in this action dialog.

The final section lists every command group in the installed CLI for these
services, including commands for resources outside the current tabs. `help`
is omitted. `wait` is a command group with service-specific subcommands.
Fargate uses ECS commands; it is not a separate CLI service.

## Action input and execution

Press `a` on a row. Choose an action, enter input if required, and press Enter.
Check the target, account, region, warning, and command. Press `y` to run it.
Long action lists scroll to show the selected row.

JSON fields must form a non-empty object. They add request fields. They cannot
replace captured identifiers or options. The input cannot set `registryId`.
AWS CLI checks field names and types. AWS checks permissions and resource state.
The preview shows full input. The command log hides JSON request values and
Lambda `--payload` values because they can contain sensitive data.
A failed API item is an error even when the CLI process exits successfully.
Partial `run-task` success can leave tasks running; check the cluster before retry.
A Lambda `FunctionError` is an error with a payload excerpt even when the CLI
exits successfully.
Read output appears in Details. The catalog marks each CLI-paginated action;
those actions use `MAX_ITEMS` from `src/config.rs` (500). Service deployment lists use one API page and can return
`nextToken`; this command does not support CLI `--max-items`. Read the command log for incomplete results.

```text
Selected row + parent --> catalog --> fixed identifiers + user input
Fixed identifiers + user input --> input check --> confirmation --> worker
Worker --> AWS CLI --> AWS API --> JSON result or error --> Details + log
```

```text
User             Lazy AWS             AWS CLI             AWS API
 |-- choose -------->|                    |                   |
 |-- input --------->| check fields        |                   |
 |-- confirm ------->|-- command --------->|-- request -------->|
 |                   |<-- result ---------|<-- response -------|
```

## EC2

### Instances tab

- Existing read — `describe-instances` supplies instance details and the selected instance's tags and security groups.
- Existing actions — `start-instances` for stopped; `stop-instances` and `reboot-instances` for running; `terminate-instances` except while shutting down or terminated.
- Added actions — `describe-instance-status` with `IncludeAllInstances=true`; `get-console-output` with `Latest=true`; `create-tags` can add the first instance tag.
- CLI-only — `run-instances` creates instances; `create-image` creates an AMI; `modify-instance-attribute` changes instance type or API stop/termination protection; `modify-instance-metadata-options` changes metadata access; `modify-instance-credit-specification` changes CPU credit mode.

Stop is supported only for EBS-root instances. The catalog hides Stop for `RootDeviceType=instance-store` and when returned data shows API stop protection. It hides Start for an instance-store root. It hides Terminate when returned data shows API stop or termination protection, or the instance is shutting down or terminated. AWS checks the actual protection settings when the command runs. Instance-store-root instances can be terminated but cannot be stopped. Stop releases instance usage charges, while attached EBS storage remains billed. Stopping loses RAM and instance-store data. Starting resumes instance usage billing. Termination is permanent; EBS volumes with `DeleteOnTermination=true` and all instance-store data are lost. Console output can contain sensitive startup data.

### Volumes tab

- Existing read — `describe-volumes` returns volumes attached to the selected instance.
- Existing actions — `create-snapshot`; `delete-volume` only while the volume is `available`.
- Added actions — `describe-volumes-modifications`; `modify-volume` for size increase; `detach-volume` only for an attached volume whose attachment names the selected primary instance.
- CLI-only — `create-volume`; `attach-volume`; volume attachment to a different instance. The JSON action also accepts other valid `modify-volume` fields.

The resize input adds fields to a fixed `VolumeId`, for example `{"Size":128}`. AWS does not support decreasing a volume size. The volume must be `in-use` or `available`; an earlier modification must be `completed`; AWS allows at most four modifications in a rolling 24-hour period. After a size increase, the guest file system still needs to be extended. A larger provisioned volume can raise EBS charges. The modification status command reports the most recent request.

Detach includes the selected instance ID and device from the matching `Attachments` entry. It is hidden if no attached entry matches the selected primary instance. A root volume can be detached only after its instance is stopped. The action does not force detach. Unmount in the guest OS first; a mounted file system can become busy or lose data. Detached EBS volumes continue to incur storage charges. A volume can attach only to an instance in the same Availability Zone. The current pane only lists attached volumes, so attach and create-volume stay in the CLI.

Snapshot storage can incur charges. Delete removes an available volume and its data. AWS says to delete an unused volume to stop its storage cost.

### Security groups tab

- Existing read — `describe-security-groups` for groups attached to the selected instance.
- Existing action — `delete-security-group`; AWS rejects deletion while a group is in use.
- Added actions — `authorize-security-group-ingress`, `revoke-security-group-ingress`, `authorize-security-group-egress`, and `revoke-security-group-egress`. Each rule change keeps the selected `GroupId` fixed.
- CLI-only — `create-security-group`; `describe-security-group-rules`; `modify-security-group-rules`; `update-security-group-rule-descriptions-ingress` and `update-security-group-rule-descriptions-egress`.

Rule input is a JSON object with `IpPermissions`, for example `{"IpPermissions":[{"IpProtocol":"tcp","FromPort":443,"ToPort":443,"IpRanges":[{"CidrIp":"203.0.113.0/24"}]}]}`. A rule needs exactly one source or destination: IPv4/IPv6 CIDR, prefix list, or security group. TCP and UDP need a port or port range; ICMP needs type and code. Rule changes affect all resources using the group and propagate quickly, with a short possible delay. Broad ingress can expose services; removed rules can interrupt traffic. AWS VPC security group quotas apply. The group delete operation can fail while it is attached or referenced.

### Tags tab

- Existing read — `describe-instances` supplies tags for the selected instance.
- Existing action — `delete-tags` removes the selected tag by key.
- Added actions — `create-tags` updates the selected tag value; another `create-tags` action adds or updates tags on the selected instance.
- CLI-only — `describe-tags` reads tags outside this selected-instance response; `create-tags` and `delete-tags` can target other EC2 resource IDs.

The selected-key update keeps `Resources` and `Tags[0].Key` fixed; the text input supplies only the value. The add/update action takes a JSON object such as `{"Tags":[{"Key":"Environment","Value":"prod"}]}` and keeps `Resources` fixed. EC2 allows up to 50 tags per resource. Keys must be unique; AWS validates key and value lengths. The `aws:` prefix is reserved and cannot be changed or deleted. Tag changes can affect IAM policies that use tags. Tag values are case-sensitive. Tag actions do not create a billable resource.

### IAM actions

New action permissions are `ec2:DescribeInstanceStatus`, `ec2:GetConsoleOutput`, `ec2:DescribeVolumesModifications`, `ec2:ModifyVolume`, `ec2:DetachVolume`, `ec2:AuthorizeSecurityGroupIngress`, `ec2:RevokeSecurityGroupIngress`, `ec2:AuthorizeSecurityGroupEgress`, `ec2:RevokeSecurityGroupEgress`, and `ec2:CreateTags`. Existing permissions include `ec2:DescribeInstances`, `ec2:DescribeVolumes`, `ec2:DescribeSecurityGroups`, `ec2:StartInstances`, `ec2:StopInstances`, `ec2:RebootInstances`, `ec2:TerminateInstances`, `ec2:CreateSnapshot`, `ec2:DeleteVolume`, `ec2:DeleteSecurityGroup`, and `ec2:DeleteTags`.

### Official sources

- [EC2 AWS CLI command index](https://docs.aws.amazon.com/cli/latest/reference/ec2/)
- Instances: [describe-instances](https://docs.aws.amazon.com/cli/latest/reference/ec2/describe-instances.html), [describe-instance-status](https://docs.aws.amazon.com/cli/latest/reference/ec2/describe-instance-status.html), [get-console-output](https://docs.aws.amazon.com/cli/latest/reference/ec2/get-console-output.html), [start-instances](https://docs.aws.amazon.com/cli/latest/reference/ec2/start-instances.html), [stop-instances](https://docs.aws.amazon.com/cli/latest/reference/ec2/stop-instances.html), [reboot-instances](https://docs.aws.amazon.com/cli/latest/reference/ec2/reboot-instances.html), [terminate-instances](https://docs.aws.amazon.com/cli/latest/reference/ec2/terminate-instances.html), [modify-instance-attribute](https://docs.aws.amazon.com/cli/latest/reference/ec2/modify-instance-attribute.html), [modify-instance-metadata-options](https://docs.aws.amazon.com/cli/latest/reference/ec2/modify-instance-metadata-options.html), [modify-instance-credit-specification](https://docs.aws.amazon.com/cli/latest/reference/ec2/modify-instance-credit-specification.html), [create-image](https://docs.aws.amazon.com/cli/latest/reference/ec2/create-image.html), [run-instances](https://docs.aws.amazon.com/cli/latest/reference/ec2/run-instances.html), [EC2 root device storage](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/RootDeviceStorage.html), [stop and start pricing behavior](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/how-ec2-instance-stop-start-works.html)
- Volumes: [describe-volumes](https://docs.aws.amazon.com/cli/latest/reference/ec2/describe-volumes.html), [describe-volumes-modifications](https://docs.aws.amazon.com/cli/latest/reference/ec2/describe-volumes-modifications.html), [modify-volume](https://docs.aws.amazon.com/cli/latest/reference/ec2/modify-volume.html), [create-snapshot](https://docs.aws.amazon.com/cli/latest/reference/ec2/create-snapshot.html), [delete-volume](https://docs.aws.amazon.com/cli/latest/reference/ec2/delete-volume.html), [create-volume](https://docs.aws.amazon.com/cli/latest/reference/ec2/create-volume.html), [attach-volume](https://docs.aws.amazon.com/cli/latest/reference/ec2/attach-volume.html), [detach-volume](https://docs.aws.amazon.com/cli/latest/reference/ec2/detach-volume.html), [EBS attach guide](https://docs.aws.amazon.com/ebs/latest/userguide/ebs-attaching-volume.html), [EBS detach guide](https://docs.aws.amazon.com/ebs/latest/userguide/ebs-detaching-volume.html), [EBS pricing](https://aws.amazon.com/ebs/pricing/)
- Security groups: [describe-security-groups](https://docs.aws.amazon.com/cli/latest/reference/ec2/describe-security-groups.html), [delete-security-group](https://docs.aws.amazon.com/cli/latest/reference/ec2/delete-security-group.html), [authorize-security-group-ingress](https://docs.aws.amazon.com/cli/latest/reference/ec2/authorize-security-group-ingress.html), [revoke-security-group-ingress](https://docs.aws.amazon.com/cli/latest/reference/ec2/revoke-security-group-ingress.html), [authorize-security-group-egress](https://docs.aws.amazon.com/cli/latest/reference/ec2/authorize-security-group-egress.html), [revoke-security-group-egress](https://docs.aws.amazon.com/cli/latest/reference/ec2/revoke-security-group-egress.html), [create-security-group](https://docs.aws.amazon.com/cli/latest/reference/ec2/create-security-group.html), [describe-security-group-rules](https://docs.aws.amazon.com/cli/latest/reference/ec2/describe-security-group-rules.html), [modify-security-group-rules](https://docs.aws.amazon.com/cli/latest/reference/ec2/modify-security-group-rules.html), [update-security-group-rule-descriptions-ingress](https://docs.aws.amazon.com/cli/latest/reference/ec2/update-security-group-rule-descriptions-ingress.html), [update-security-group-rule-descriptions-egress](https://docs.aws.amazon.com/cli/latest/reference/ec2/update-security-group-rule-descriptions-egress.html), [VPC security group rule guide](https://docs.aws.amazon.com/vpc/latest/userguide/working-with-security-group-rules.html), [AWS CLI security group guide](https://docs.aws.amazon.com/cli/latest/userguide/cli-services-ec2-sg.html)
- Tags: [create-tags](https://docs.aws.amazon.com/cli/latest/reference/ec2/create-tags.html), [delete-tags](https://docs.aws.amazon.com/cli/latest/reference/ec2/delete-tags.html), [describe-tags](https://docs.aws.amazon.com/cli/latest/reference/ec2/describe-tags.html), [EC2 tag rules](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/Using_Tags.html)

## ECS

### Clusters

- Delete cluster — Existing — `ecs delete-cluster`. ECS requires the cluster to have no services or running tasks.
- Update cluster settings — Added — `ecs update-cluster-settings --cluster ... --settings ...`. The cluster ARN is fixed; JSON fields add the required `settings` list, such as `{"settings":[{"name":"containerInsights","value":"enhanced"}]}`. Enhanced Container Insights supports EC2 and Fargate clusters and can add CloudWatch charges.
- List services — Added — `ecs list-services --cluster ...`. The selected cluster is fixed. AWS CLI paginates results and returns service ARNs.
- List tasks — Added — `ecs list-tasks --cluster ...`. The selected cluster is fixed. The default desired status is `RUNNING`; recently stopped tasks can also appear briefly. AWS CLI paginates results. The command supports `desiredStatus=STOPPED`, but the action has no status input.
- Update cluster configuration — CLI-only — `ecs update-cluster` takes the cluster plus optional `settings`, `configuration`, and `serviceConnectDefaults`. It also updates execute-command and managed-storage configuration. The selected action catalog exposes the narrower settings operation only.
- Create cluster — CLI-only — `ecs create-cluster` is not tied to a selected cluster row. It needs a new name or the default name; capacity providers, settings, tags, and service-connect defaults are optional.

Sources: [delete-cluster](https://docs.aws.amazon.com/cli/latest/reference/ecs/delete-cluster.html), [update-cluster-settings](https://docs.aws.amazon.com/cli/latest/reference/ecs/update-cluster-settings.html), [list-services](https://docs.aws.amazon.com/cli/latest/reference/ecs/list-services.html), [list-tasks](https://docs.aws.amazon.com/cli/latest/reference/ecs/list-tasks.html), [update-cluster](https://docs.aws.amazon.com/cli/latest/reference/ecs/update-cluster.html), [create-cluster](https://docs.aws.amazon.com/cli/latest/reference/ecs/create-cluster.html), [Container Insights settings](https://docs.aws.amazon.com/AmazonECS/latest/developerguide/ecs-account-settings.html).

### Services

CLI-only update fields include `platformVersion`, `networkConfiguration`,
`deploymentConfiguration`, `capacityProviderStrategy`, load balancers, health
check grace period, and ECS Exec settings. Support depends on the deployment
controller. The app exposes task count, task definition, and forced deployment.
See [update-service](https://docs.aws.amazon.com/cli/latest/reference/ecs/update-service.html)
for the full field list.

- Start new deployment — Existing — `ecs update-service --force-new-deployment`. ECS replaces tasks. This is useful when a reused image tag now points to a new image.
- Set desired task count — Existing — `ecs update-service --desired-count`. The dialog starts with the current desired count. The count can start or stop tasks; it does not itself start a service deployment.
- Delete service — Existing — `ecs delete-service` with JSON `force:false`. The service desired count must be zero.
- Deploy task definition revision — Added for ECS controller — `ecs update-service --task-definition`. The action takes a family/revision or ARN. This starts a new deployment. It sets the task definition; it does not restore every prior service setting. This added action is shown only when the controller is ECS or absent (the API default).
- List service deployments — Added for ECS controller — `ecs list-service-deployments`. The selected cluster and service ARN are fixed. Output includes each deployment ARN, status, and target service revision. AWS documents recent deployment history for the last 90 days.
- Rollback service deployment — Added for ECS controller — `ecs stop-service-deployment --stop-type ROLLBACK`. The user supplies the deployment ARN returned by list. This rolls an ongoing deployment back to the previous service revision, even if rollback was not configured. Ongoing deployments can be stopped; a completed deployment cannot. The command uses the ECS service-deployment ARN.
- Create service — CLI-only — `ecs create-service` needs a task definition and service-specific deployment, scheduling, and networking choices. Fargate requires `awsvpc`; this is not a safe one-field selected-row action.
- Describe deployment — CLI-only — `ecs describe-service-deployments` needs a deployment ARN. The list action returns it, but the UI has no deployment row to select.

Rollback distinction: `stop-service-deployment ROLLBACK` stops the selected ongoing deployment and returns to the previous service revision. `update-service --task-definition old-family:revision` starts a new deployment with that chosen task definition. It is a practical fallback when the desired task definition revision is known, but it is not the same operation and does not restore other prior service fields. For Fargate, use the ECS service-deployment ARN for an ongoing ECS-managed deployment; task definition ARN alone does not identify that deployment.

Service deployment sequence:

```text
User          Lazy AWS              AWS CLI                 ECS
 |                |                     |                    |
 |-- list ------->|                     |                    |
 |                |-- CLI command ----->|                    |
 |                |                     |-- ListDeployments ->|
 |                |                     |<-- ARNs + status ---|
 |                |<-- JSON ------------|                    |
 |<-- output -----|                     |                    |
 |-- confirm ---->|                     |                    |
 |                |-- CLI command ----->|                    |
 |                |                     |-- StopDeployment -->|
 |                |                     |<-- result ----------|
 |                |<-- JSON ------------|                    |
 |<-- output -----|                     |                    |
```

Sources: [update-service](https://docs.aws.amazon.com/cli/latest/reference/ecs/update-service.html), [delete-service](https://docs.aws.amazon.com/cli/latest/reference/ecs/delete-service.html), [list-service-deployments](https://docs.aws.amazon.com/cli/latest/reference/ecs/list-service-deployments.html), [describe-service-deployments](https://docs.aws.amazon.com/cli/latest/reference/ecs/describe-service-deployments.html), [stop-service-deployment](https://docs.aws.amazon.com/cli/latest/reference/ecs/stop-service-deployment.html), [stop and roll back a deployment](https://docs.aws.amazon.com/AmazonECS/latest/developerguide/stop-service-deployment.html), [service controllers](https://docs.aws.amazon.com/AmazonECS/latest/developerguide/service_definition_parameters.html), [create-service](https://docs.aws.amazon.com/cli/latest/reference/ecs/create-service.html).

### Tasks

- Stop task — Existing — `ecs stop-task`. A service may start a replacement. The task can receive `SIGTERM`, then `SIGKILL` after its stop timeout.
- Run task again — Added — `ecs run-task` with captured cluster and task-definition ARN, plus the original launch type when present. It creates a standalone task and may add compute charges. JSON fields can add capacity-provider or network settings. `awsvpc` task definitions require `networkConfiguration`; Fargate uses `awsvpc`, so enter subnet IDs. Security groups are optional; AWS uses the VPC default if omitted. `--count` is omitted; the API permits at most 10 tasks per call.
- View task definition — Added when task definition ARN exists — `ecs describe-task-definition --include TAGS` with the task's captured task-definition ARN. AWS only describes an inactive definition while an active task or service references it.
- Start task — CLI-only — `ecs start-task` needs container-instance ARNs and is for manually placing tasks. It is not the Fargate path.
- Execute command — CLI-only — `ecs execute-command` needs an interactive session and the Session Manager plugin, plus ECS Exec and IAM setup. The action runner captures command output and does not provide an interactive terminal.

Sources: [stop-task](https://docs.aws.amazon.com/cli/latest/reference/ecs/stop-task.html), [run-task](https://docs.aws.amazon.com/cli/latest/reference/ecs/run-task.html), [describe-task-definition](https://docs.aws.amazon.com/cli/latest/reference/ecs/describe-task-definition.html), [list-tasks](https://docs.aws.amazon.com/cli/latest/reference/ecs/list-tasks.html), [start-task](https://docs.aws.amazon.com/cli/latest/reference/ecs/start-task.html), [execute-command](https://docs.aws.amazon.com/cli/latest/reference/ecs/execute-command.html).

### Task definitions

- CLI-only: `delete-task-definitions` requests removal of inactive revisions.
  Deregister an active revision first. Running tasks and services can delay
  removal. See [delete-task-definitions](https://docs.aws.amazon.com/cli/latest/reference/ecs/delete-task-definitions.html).

- Deregister task definition — Existing — `ecs deregister-task-definition`. Active revisions can be deregistered; running tasks keep using their revision.
- Run task — Added for active definitions — `ecs run-task` with selected cluster and task-definition ARN. It can add compute charges. User JSON fields must supply any launch type or capacity-provider strategy required by the cluster and `networkConfiguration` for `awsvpc` (including Fargate).
- List task definition revisions — Added when family exists — `ecs list-task-definitions --family-prefix ...`. Uses the selected family as a prefix, which can match similarly named families. AWS returns `ACTIVE` revisions by default; listing inactive revisions needs `--status INACTIVE`.
- View task definition (include TAGS) — Added for active and inactive rows — `ecs describe-task-definition --include TAGS` with the selected revision ARN. AWS only describes an inactive definition while an active task or service references it.
- Register task definition — CLI-only — `ecs register-task-definition` registers a new revision and needs a full family/container definition, roles, and any volumes, networking, logging, or secrets. A create wizard or file input is outside this action dialog.
- Describe task definition — Background read — The ECS refresh path also calls `ecs describe-task-definition` for definitions found through selected-cluster services and tasks.

Sources: [deregister-task-definition](https://docs.aws.amazon.com/cli/latest/reference/ecs/deregister-task-definition.html), [run-task](https://docs.aws.amazon.com/cli/latest/reference/ecs/run-task.html), [list-task-definitions](https://docs.aws.amazon.com/cli/latest/reference/ecs/list-task-definitions.html), [register-task-definition](https://docs.aws.amazon.com/cli/latest/reference/ecs/register-task-definition.html), [describe-task-definition](https://docs.aws.amazon.com/cli/latest/reference/ecs/describe-task-definition.html).

### Fargate and `run-task` request fields

```text
Task or task-definition row
  -> fixed cluster + taskDefinition (+ captured launchType for an existing task)
  -> JSON fields: networkConfiguration.awsvpcConfiguration.subnets
  -> AWS CLI ecs run-task
  -> ECS starts a standalone task
```

AWS requires `networkConfiguration` when the task definition uses `awsvpc`, and the subnet list is required. Fargate task definitions use `awsvpc`, so the action cannot infer usable subnet IDs from the task definition. Security groups are optional; AWS uses the VPC default when none are supplied. The selected cluster does not provide a single safe network choice. A standalone task can incur compute charges. The dialog asks for JSON fields and leaves request-schema validation to the AWS CLI/API.


### ECS tags and capacity

- CLI-only on clusters: `put-cluster-capacity-providers` changes the available
  providers and default placement strategy. Supply the full provider list;
  an omitted provider is removed from the association. Fargate uses `FARGATE`
  or `FARGATE_SPOT`. See [cluster capacity providers](https://docs.aws.amazon.com/cli/latest/reference/ecs/put-cluster-capacity-providers.html).
- CLI-only on supported ECS resource ARNs: `list-tags-for-resource`,
  `tag-resource`, and `untag-resource` read, add/update, and remove tags.
  These do not have separate ECS tag tabs. See [resource tags](https://docs.aws.amazon.com/cli/latest/reference/ecs/list-tags-for-resource.html).

## ECR

### Repositories

- `describe-repositories` — Implemented read — Populates repository rows and reads scan, mutability, ARN, and repository settings.
- `delete-repository` — Implemented action — Uses `force=false`; AWS only deletes an empty repository.
- `put-image-tag-mutability` — Implemented action — Toggles mutable/immutable settings. For `*_WITH_EXCLUSION`, retains the captured filters and hides the toggle if filters are absent.
- `get-repository-policy` — Implemented action — Reads the selected repository policy.
- `set-repository-policy` — Implemented action — Requires `policyText` in JSON fields. The action captures `force:false` to keep the AWS policy access check.
- `delete-repository-policy` — Implemented action — Removes the selected repository policy. It can fail when no policy exists.
- `get-lifecycle-policy` — Implemented action — Reads the selected lifecycle policy.
- `put-lifecycle-policy` — Implemented action — Requires `lifecyclePolicyText` in JSON fields. ECR evaluates matching rules within 24 hours.
- `delete-lifecycle-policy` — Implemented action — Stops the selected repository's automatic lifecycle rules. It can fail when no policy exists.
- `put-image-scanning-configuration` — Implemented action — Toggles repository `scanOnPush` only when the current value is present. AWS marks this API deprecated in favor of registry-level scanning; both toggle warnings state this.
- `list-tags-for-resource` — Implemented read — Populates repository tag rows from the repository ARN.
- `tag-resource` — Implemented action — Adds a tag or changes the selected tag value. AWS replaces the value when a tag key already exists.
- `untag-resource` — Implemented action — Removes the selected repository tag key.
- `put-registry-scanning-configuration` — CLI-only — Preferred way to set scanning rules. It affects registry-level configuration, so it is not scoped to the selected repository.
- `create-repository` — CLI-only — Creates a new resource and is outside selected-row actions.

### Images

- `describe-images` — Implemented read and action — Existing read lists images. The action addresses the selected digest exactly.
- `start-image-scan` — Implemented action — Starts a basic scan for the selected digest. ECR limits basic scan requests to one per image per day.
- `describe-image-scan-findings` — Implemented action — Reads findings for the selected digest.
- `batch-get-image` — Implemented action — Retrieves the selected digest's manifest; the response can be large.
- `batch-delete-image` — Implemented action — Deletes the selected digest and its tags.

### Image tags

- `describe-images` — Implemented action — Reads metadata for the selected image tag.
- `batch-get-image` — Implemented action — Retrieves the selected tag's image manifest; the response can be large.
- `batch-delete-image` — Implemented action — Removes the selected tag. ECR removes the image when no tags remain.
- `put-image` — CLI-only — Can add or move a tag without Docker, but requires the full image manifest. The current one-line input dialog makes this a poor in-app action.

### Repository tags

- `list-tags-for-resource` — Implemented read — Populates selected repository tags.
- `tag-resource` — Implemented action — Adds a user-supplied key/value pair or changes the selected tag value. A matching key is replaced.
- `untag-resource` — Implemented action — Removes the selected tag key.

### Other commands for these tabs

- Repositories, CLI-only: `batch-get-repository-scanning-configuration` reads current
  scan settings for named repositories. `start-lifecycle-policy-preview` and
  `get-lifecycle-policy-preview` show which images a policy can expire.
- Images and image tags, CLI-only: `describe-image-replication-status` and
  `describe-image-signing-status` read replication and signing state.
- Images, CLI-only: `list-image-referrers` reads OCI artifacts, such as signatures
  and SBOMs, attached to the selected image digest.
- Images and image tags, CLI-only: `update-image-storage-class` archives or restores
  an image. Archived images cannot be pulled. The archive has a 90-day minimum
  storage charge; restore to STANDARD can take up to 20 minutes.
- Repositories, CLI-only: registry, account, pull-through cache, replication,
  signing, and repository creation template commands affect more than the
  selected repository. They are listed in the full command index.

Sources: [scanning configuration](https://docs.aws.amazon.com/cli/latest/reference/ecr/batch-get-repository-scanning-configuration.html),
[lifecycle preview](https://docs.aws.amazon.com/cli/latest/reference/ecr/start-lifecycle-policy-preview.html),
[preview results](https://docs.aws.amazon.com/cli/latest/reference/ecr/get-lifecycle-policy-preview.html),
[replication status](https://docs.aws.amazon.com/cli/latest/reference/ecr/describe-image-replication-status.html),
[signing status](https://docs.aws.amazon.com/cli/latest/reference/ecr/describe-image-signing-status.html),
[image referrers](https://docs.aws.amazon.com/cli/latest/reference/ecr/list-image-referrers.html),
[image storage class](https://docs.aws.amazon.com/cli/latest/reference/ecr/update-image-storage-class.html).

### Other reviewed commands

- `get-login-password` is excluded. It writes an ECR authentication token, and the app must not log that token.
- Docker build, login, pull, and push flows are excluded. They need local files or interactive Docker behavior.
- Registry-wide scanning updates and repository creation are CLI-only because they do not target the selected row.

### Official sources

- [AWS CLI: describe-repositories](https://docs.aws.amazon.com/cli/latest/reference/ecr/describe-repositories.html)
- [AWS CLI: delete-repository](https://docs.aws.amazon.com/cli/latest/reference/ecr/delete-repository.html)
- [AWS CLI: put-image-tag-mutability](https://docs.aws.amazon.com/cli/latest/reference/ecr/put-image-tag-mutability.html)
- [AWS CLI: get-repository-policy](https://docs.aws.amazon.com/cli/latest/reference/ecr/get-repository-policy.html)
- [AWS CLI: set-repository-policy](https://docs.aws.amazon.com/cli/latest/reference/ecr/set-repository-policy.html)
- [AWS CLI: delete-repository-policy](https://docs.aws.amazon.com/cli/latest/reference/ecr/delete-repository-policy.html)
- [AWS CLI: get-lifecycle-policy](https://docs.aws.amazon.com/cli/latest/reference/ecr/get-lifecycle-policy.html)
- [AWS CLI: put-lifecycle-policy](https://docs.aws.amazon.com/cli/latest/reference/ecr/put-lifecycle-policy.html)
- [AWS CLI: delete-lifecycle-policy](https://docs.aws.amazon.com/cli/latest/reference/ecr/delete-lifecycle-policy.html)
- [AWS CLI: put-image-scanning-configuration](https://docs.aws.amazon.com/cli/latest/reference/ecr/put-image-scanning-configuration.html)
- [AWS CLI: put-registry-scanning-configuration](https://docs.aws.amazon.com/cli/latest/reference/ecr/put-registry-scanning-configuration.html)
- [AWS CLI: list-tags-for-resource](https://docs.aws.amazon.com/cli/latest/reference/ecr/list-tags-for-resource.html)
- [AWS CLI: tag-resource](https://docs.aws.amazon.com/cli/latest/reference/ecr/tag-resource.html)
- [AWS CLI: untag-resource](https://docs.aws.amazon.com/cli/latest/reference/ecr/untag-resource.html)
- [AWS CLI: describe-images](https://docs.aws.amazon.com/cli/latest/reference/ecr/describe-images.html)
- [AWS CLI: start-image-scan](https://docs.aws.amazon.com/cli/latest/reference/ecr/start-image-scan.html)
- [AWS CLI: describe-image-scan-findings](https://docs.aws.amazon.com/cli/latest/reference/ecr/describe-image-scan-findings.html)
- [AWS CLI: batch-get-image](https://docs.aws.amazon.com/cli/latest/reference/ecr/batch-get-image.html)
- [AWS CLI: batch-delete-image](https://docs.aws.amazon.com/cli/latest/reference/ecr/batch-delete-image.html)
- [AWS CLI: put-image](https://docs.aws.amazon.com/cli/latest/reference/ecr/put-image.html)
- [AWS CLI: create-repository](https://docs.aws.amazon.com/cli/latest/reference/ecr/create-repository.html)
- [AWS CLI: get-login-password](https://docs.aws.amazon.com/cli/latest/reference/ecr/get-login-password.html)
- [ECR image tag mutability and exclusions](https://docs.aws.amazon.com/AmazonECR/latest/userguide/image-tag-mutability.html)
- [ECR image scanning](https://docs.aws.amazon.com/AmazonECR/latest/userguide/image-scanning.html)
- [ECR lifecycle policies](https://docs.aws.amazon.com/AmazonECR/latest/userguide/LifecyclePolicies.html)
- [ECR repository tags](https://docs.aws.amazon.com/AmazonECR/latest/userguide/ecr-using-tags.html)

## S3

These actions use the selected bucket's Region. The current tabs list general
purpose buckets. Some commands in the full index also support directory buckets;
they need different endpoints and permissions.

### Buckets

- Read: `s3api list-buckets`, `get-bucket-location`, `get-bucket-versioning`.
- Action: `put-bucket-versioning` enables or suspends versioning. Suspension
  does not remove stored versions.
- Action: `delete-bucket` deletes an empty bucket. Objects, versions, and delete
  markers must be removed first. The action does not empty the bucket.
- Action: `put-object` uploads one local file. The file name becomes the key.
  It can replace an object with that key.
- Action: `get-bucket-tagging` reads tags; `put-bucket-tagging` replaces all tags.
  Input: `{"Tagging":{"TagSet":[{"Key":"owner","Value":"team"}]}}`.
- CLI-only: `create-bucket`, `head-bucket`, and `delete-bucket-tagging`.
  Creation needs a new name and Region settings; the other commands are not in
  the current menu.

### Objects

- Action: `get-object` downloads to a new local file; an existing file is not replaced.
  This streaming command uses named bucket/key arguments and an output file;
  it does not accept `--cli-input-json`.
- Action: `put-object` replaces the selected key with one local file.
- Action: `delete-object` removes the current object. With versioning enabled,
  S3 adds a delete marker. This is not a permanent removal of all versions.
- Action: `head-object` reads object metadata.
- Action: `get-object-tagging`, `put-object-tagging`, `delete-object-tagging`
  read, replace, or remove tags on the current version. Replace uses the same
  `Tagging` input as bucket tags.
- Action: `list-object-versions` reads versions and delete markers with the
  selected key as `Prefix`. Other keys with the same prefix can appear.
- CLI-only: `copy-object` copies an object or restores a chosen version as
  current; `restore-object` requests archive retrieval; `get-object-attributes`
  reads checksums and size; `get-object-acl`/`put-object-acl` manage ACLs where
  enabled; `get-object-retention`/`put-object-retention` and
  `get-object-legal-hold`/`put-object-legal-hold` manage Object Lock controls.
- CLI-only: `delete-object --version-id` permanently removes a specified
  version or delete marker. `delete-objects` removes multiple selected keys
  or versions. The tabs have no version selection or bulk selection.
- CLI-only: `create-multipart-upload`, `upload-part`, `upload-part-copy`,
  `list-parts`, `complete-multipart-upload`, `abort-multipart-upload` need an
  upload ID and part state. Lazy AWS uses one PutObject request, up to 5 GB.
- CLI-only: `s3 presign` produces a URL that grants temporary object access.
  Such URLs should not be stored in the command log.

### Prefixes

- Action: `list-objects-v2` reads keys with the selected prefix.
- Action: `list-object-versions` reads matching versions and delete markers.
- Action: `list-multipart-uploads` reads incomplete matching uploads.
- CLI-only: `s3 cp --recursive`, `mv --recursive`, `rm --recursive`, and `sync`
  act on many objects or local files. A prefix is a key string, not a directory
  resource. Recursive changes need a separate review of the affected keys.

### Bucket settings

- Action: `get-bucket-versioning`, `get-bucket-encryption`,
  `get-public-access-block`, `get-bucket-policy`,
  `get-bucket-lifecycle-configuration`, `get-bucket-cors`, `get-bucket-logging`,
  `get-bucket-ownership-controls`, `get-bucket-replication`,
  `get-bucket-notification-configuration`, and `get-bucket-tagging` read settings.
  AWS can return an error when a setting is absent.
- Action: `put-bucket-versioning` is also available on the Versioning row.
- Action: `put-bucket-encryption` sets encryption for new objects. Input:
  `{"ServerSideEncryptionConfiguration":{"Rules":[{"ApplyServerSideEncryptionByDefault":{"SSEAlgorithm":"AES256"}}]}}`.
- Action: `put-bucket-lifecycle-configuration` replaces all lifecycle rules.
  Input needs `LifecycleConfiguration` with `Rules`. Expiration rules can remove data.
- Action: `put-bucket-cors` replaces all CORS rules. Input needs
  `CORSConfiguration` with `CORSRules`.
- Action: `put-bucket-tagging` replaces all bucket tags.
- CLI-only: put/delete commands for policy, public access block, replication,
  logging, ownership controls, notifications, website, ACLs, acceleration,
  Object Lock, inventory, metrics, analytics, intelligent tiering, and metadata.
  The full command index below includes every command name.

Use encryption at rest and TLS in transit. Scope permissions to the required
resources. Keep public access blocked unless the workload needs it. For an
operational account, use S3 access logs, CloudTrail data events, and CloudWatch
metrics to record access and find failed requests. These are AWS settings;
this change does not create them. Requests, transfers, versions, archive
retrieval, KMS encryption, and lifecycle transitions can add charges.

Sources: [S3 API command index](https://docs.aws.amazon.com/cli/latest/reference/s3api/),
[high-level S3 commands](https://docs.aws.amazon.com/cli/latest/reference/s3/),
[object versions](https://docs.aws.amazon.com/cli/latest/reference/s3api/list-object-versions.html),
[object tags](https://docs.aws.amazon.com/cli/latest/reference/s3api/put-object-tagging.html),
[bucket tags](https://docs.aws.amazon.com/cli/latest/reference/s3api/put-bucket-tagging.html),
[bucket encryption](https://docs.aws.amazon.com/cli/latest/reference/s3api/put-bucket-encryption.html),
[bucket lifecycle](https://docs.aws.amazon.com/cli/latest/reference/s3api/put-bucket-lifecycle-configuration.html),
[bucket CORS](https://docs.aws.amazon.com/cli/latest/reference/s3api/put-bucket-cors.html),
[multipart upload list](https://docs.aws.amazon.com/cli/latest/reference/s3api/list-multipart-uploads.html),
[S3 security practices](https://docs.aws.amazon.com/AmazonS3/latest/userguide/security-best-practices.html).

## Lambda

### Functions tab

- `list-functions` — Implemented read — Populates function rows with runtime, memory, and timeout. AWS returns up to 50 functions per call; the app paginates with `MAX_ITEMS`.
- `invoke` — Implemented action (three variants) — Sync, sync with `--log-type Tail`, and async `--invocation-type Event`. The dialog takes an optional JSON payload or empty input. The payload is passed with `--payload` and `--cli-binary-format raw-in-base64-out`; the function response is written to a private temporary file and shown in Details as `Payload`. A `FunctionError` is an action error with a payload excerpt. Log tail adds decoded `LogResultDecoded`. The command log hides the payload value.
- `get-function` — Implemented action — Reads configuration, code location, and tags.
- `get-function-configuration` — Implemented action — Reads runtime, memory, timeout, environment, and VPC settings.
- `get-policy` — Implemented action — Reads the resource policy; fails when no policy exists.
- `get-function-concurrency` — Implemented action — Reads reserved concurrency; empty means shared account capacity.
- `get-runtime-management-config` — Implemented action — Reads runtime update mode.
- `get-function-recursion-config` — Implemented action — Reads recursive-loop detection.
- `list-versions-by-function` — Implemented read and action — Pane read lists versions; the action refreshes the same list.
- `list-aliases` — Implemented read and action — Pane read lists aliases; the action refreshes the same list.
- `list-event-source-mappings` — Implemented action — Lists stream/queue mappings filtered by function name.
- `list-function-event-invoke-configs` — Implemented action — Lists async retry and destination settings.
- `list-function-url-configs` — Implemented action — Lists function URL endpoints.
- `publish-version` — Implemented action — Publishes a new version from `$LATEST`; takes optional JSON fields such as `{"Description":"release"}`.
- `update-function-configuration` — Implemented action — Takes JSON fields such as `{"MemorySize":512,"Timeout":30}`; the function restarts.
- `put-function-concurrency` — Implemented action — Takes a non-negative integer; zero blocks invocations.
- `delete-function-concurrency` — Implemented action — Removes the reserved limit.
- `create-alias` — Implemented action — Takes JSON fields such as `{"Name":"prod","FunctionVersion":"3"}`.
- `tag-resource` — Implemented action — Takes JSON fields such as `{"Tags":{"Team":"platform"}}`.
- `delete-function` — Implemented action — Deletes the function and its versions; remove event mappings first.
- CLI-only — `create-function` needs a new name plus code, role, and runtime choices; `update-function-code` needs a new deployment package; `create/update/delete-function-url-config`, `create/update/delete-event-source-mapping`, `put/list/delete` for code-signing, layers, capacity providers, durable executions, and `add/remove-permission` need ARNs or documents outside the selected row.

### Versions tab

- `invoke` — Implemented action — Invokes the version ARN synchronously with an optional JSON payload.
- `get-function` — Implemented action — Reads this version's configuration and code.
- `get-function-configuration` — Implemented action — Reads this version's settings.
- `get-provisioned-concurrency-config` — Implemented action except on `$LATEST` — Reads provisioned capacity; fails when none is set.
- `put-provisioned-concurrency-config` — Implemented action except on `$LATEST` — Takes a positive integer; adds charges even without invocations.
- `delete-provisioned-concurrency-config` — Implemented action except on `$LATEST` — Removes provisioned capacity.
- `delete-function` with `Qualifier` — Implemented action except on `$LATEST` — Deletes the version; remove aliases first.
- CLI-only — `publish-version` from a version row is on the function row; `update-function-code` creates a new version from `$LATEST`.

### Aliases tab

- `invoke` — Implemented action — Invokes the alias ARN; payload rules match the function tab.
- `get-alias` — Implemented action — Reads routing configuration.
- `update-alias` — Implemented action — Takes a version number or `$LATEST` and points the alias at it.
- `get-provisioned-concurrency-config` — Implemented action — Reads provisioned capacity for the alias qualifier.
- `put-provisioned-concurrency-config` — Implemented action — Takes a positive integer.
- `delete-alias` — Implemented action — Deletes the alias and keeps the version.
- CLI-only — `create-alias` is on the function row; weighted routing (`RoutingConfig`) edits are via `update-alias` JSON in the CLI.

### Tags tab

- `list-tags` — Implemented read — Populates tags from the `Tags` object on the function ARN.
- `untag-resource` — Implemented action — Removes the selected tag key.
- `tag-resource` — Implemented action — Updates the selected tag value (empty allowed) or adds tags from a `Tags` object.
- CLI-only — `list-tags` for versions/aliases uses the same command with a qualified ARN in the CLI.

### IAM actions

New action permissions are `lambda:InvokeFunction`, `lambda:GetFunction`,
`lambda:GetFunctionConfiguration`, `lambda:GetPolicy`,
`lambda:GetFunctionConcurrency`, `lambda:GetRuntimeManagementConfig`,
`lambda:GetFunctionRecursionConfig`, `lambda:ListVersionsByFunction`,
`lambda:ListAliases`, `lambda:ListEventSourceMappings`,
`lambda:ListFunctionEventInvokeConfigs`, `lambda:ListFunctionUrlConfigs`,
`lambda:PublishVersion`, `lambda:UpdateFunctionConfiguration`,
`lambda:PutFunctionConcurrency`, `lambda:DeleteFunctionConcurrency`,
`lambda:GetProvisionedConcurrencyConfig`,
`lambda:PutProvisionedConcurrencyConfig`,
`lambda:DeleteProvisionedConcurrencyConfig`, `lambda:GetAlias`,
`lambda:CreateAlias`, `lambda:UpdateAlias`, `lambda:DeleteAlias`,
`lambda:TagResource`, `lambda:UntagResource`, `lambda:ListTags`, and
`lambda:DeleteFunction`. Existing read permissions are `lambda:ListFunctions`,
`lambda:ListVersionsByFunction`, `lambda:ListAliases`, and `lambda:ListTags`.

### Official sources

- [Lambda AWS CLI command index](https://docs.aws.amazon.com/cli/latest/reference/lambda/)
- Functions: [list-functions](https://docs.aws.amazon.com/cli/latest/reference/lambda/list-functions.html), [invoke](https://docs.aws.amazon.com/cli/latest/reference/lambda/invoke.html), [get-function](https://docs.aws.amazon.com/cli/latest/reference/lambda/get-function.html), [get-function-configuration](https://docs.aws.amazon.com/cli/latest/reference/lambda/get-function-configuration.html), [get-policy](https://docs.aws.amazon.com/cli/latest/reference/lambda/get-policy.html), [publish-version](https://docs.aws.amazon.com/cli/latest/reference/lambda/publish-version.html), [update-function-configuration](https://docs.aws.amazon.com/cli/latest/reference/lambda/update-function-configuration.html), [delete-function](https://docs.aws.amazon.com/cli/latest/reference/lambda/delete-function.html)
- Concurrency: [put-function-concurrency](https://docs.aws.amazon.com/cli/latest/reference/lambda/put-function-concurrency.html), [get-function-concurrency](https://docs.aws.amazon.com/cli/latest/reference/lambda/get-function-concurrency.html), [delete-function-concurrency](https://docs.aws.amazon.com/cli/latest/reference/lambda/delete-function-concurrency.html), [put-provisioned-concurrency-config](https://docs.aws.amazon.com/cli/latest/reference/lambda/put-provisioned-concurrency-config.html), [get-provisioned-concurrency-config](https://docs.aws.amazon.com/cli/latest/reference/lambda/get-provisioned-concurrency-config.html)
- Aliases: [list-aliases](https://docs.aws.amazon.com/cli/latest/reference/lambda/list-aliases.html), [get-alias](https://docs.aws.amazon.com/cli/latest/reference/lambda/get-alias.html), [create-alias](https://docs.aws.amazon.com/cli/latest/reference/lambda/create-alias.html), [update-alias](https://docs.aws.amazon.com/cli/latest/reference/lambda/update-alias.html), [delete-alias](https://docs.aws.amazon.com/cli/latest/reference/lambda/delete-alias.html)
- Observability: [list-event-source-mappings](https://docs.aws.amazon.com/cli/latest/reference/lambda/list-event-source-mappings.html), [list-function-event-invoke-configs](https://docs.aws.amazon.com/cli/latest/reference/lambda/list-function-event-invoke-configs.html), [list-function-url-configs](https://docs.aws.amazon.com/cli/latest/reference/lambda/list-function-url-configs.html), [Lambda CloudWatch logs](https://docs.aws.amazon.com/lambda/latest/dg/monitoring-cloudwatchlogs.html)
- Tags: [list-tags](https://docs.aws.amazon.com/cli/latest/reference/lambda/list-tags.html), [tag-resource](https://docs.aws.amazon.com/cli/latest/reference/lambda/tag-resource.html), [untag-resource](https://docs.aws.amazon.com/cli/latest/reference/lambda/untag-resource.html)

## Full AWS CLI command index

The lists below come from local `aws SERVICE help` in AWS CLI 2.38.0. They are
complete for that version. They include resource creation, account settings,
network resources, and other operations outside the 20 tabs. Inclusion here
does not mean an action is present in Lazy AWS. The tab sections state which
commands the app supports. Check the linked current reference for the request
fields, permissions, prerequisites, and limits of a CLI-only command.

### `aws ec2` — 806 command groups

[Official command reference](https://docs.aws.amazon.com/cli/latest/reference/ec2/).

- `accept-address-transfer`, `accept-capacity-reservation-billing-ownership`, `accept-reserved-instances-exchange-quote`, `accept-transit-gateway-client-vpn-attachment`, `accept-transit-gateway-multicast-domain-associations`
- `accept-transit-gateway-peering-attachment`, `accept-transit-gateway-vpc-attachment`, `accept-vpc-endpoint-connections`, `accept-vpc-peering-connection`, `advertise-byoip-cidr`
- `allocate-address`, `allocate-hosts`, `allocate-ipam-pool-cidr`, `apply-security-groups-to-client-vpn-target-network`, `assign-ipv6-addresses`
- `assign-private-ip-addresses`, `assign-private-nat-gateway-address`, `associate-address`, `associate-application-status-check`, `associate-capacity-reservation-billing-owner`
- `associate-client-vpn-target-network`, `associate-dhcp-options`, `associate-enclave-certificate-iam-role`, `associate-iam-instance-profile`, `associate-instance-event-window`
- `associate-ipam-byoasn`, `associate-ipam-resource-discovery`, `associate-nat-gateway-address`, `associate-route-server`, `associate-route-table`
- `associate-security-group-vpc`, `associate-subnet-cidr-block`, `associate-transit-gateway-multicast-domain`, `associate-transit-gateway-policy-table`, `associate-transit-gateway-route-table`
- `associate-trunk-interface`, `associate-vpc-cidr-block`, `attach-classic-link-vpc`, `attach-image-watermark`, `attach-internet-gateway`
- `attach-network-interface`, `attach-verified-access-trust-provider`, `attach-volume`, `attach-vpn-gateway`, `authorize-client-vpn-ingress`
- `authorize-security-group-egress`, `authorize-security-group-ingress`, `batch-modify-ipam-routing-policy-registrations`, `bundle-instance`, `cancel-bundle-task`
- `cancel-capacity-reservation`, `cancel-capacity-reservation-fleets`, `cancel-conversion-task`, `cancel-declarative-policies-report`, `cancel-export-task`
- `cancel-image-launch-permission`, `cancel-import-task`, `cancel-reserved-instances-listing`, `cancel-spot-fleet-requests`, `cancel-spot-instance-requests`
- `confirm-product-instance`, `copy-fpga-image`, `copy-image`, `copy-snapshot`, `copy-volumes`
- `create-application-status-check`, `create-capacity-manager-data-export`, `create-capacity-reservation`, `create-capacity-reservation-by-splitting`, `create-capacity-reservation-cancellation-quote`
- `create-capacity-reservation-date-change-quote`, `create-capacity-reservation-fleet`, `create-carrier-gateway`, `create-client-vpn-endpoint`, `create-client-vpn-route`
- `create-coip-cidr`, `create-coip-pool`, `create-customer-gateway`, `create-default-subnet`, `create-default-vpc`
- `create-delegate-mac-volume-ownership-task`, `create-dhcp-options`, `create-egress-only-internet-gateway`, `create-fleet`, `create-flow-logs`
- `create-fpga-image`, `create-image`, `create-image-usage-report`, `create-instance-connect-endpoint`, `create-instance-event-window`
- `create-instance-export-task`, `create-internet-gateway`, `create-interruptible-capacity-reservation-allocation`, `create-ipam`, `create-ipam-external-resource-verification-token`
- `create-ipam-internet-registry-association`, `create-ipam-policy`, `create-ipam-pool`, `create-ipam-prefix-list-resolver`, `create-ipam-prefix-list-resolver-target`
- `create-ipam-resource-discovery`, `create-ipam-routing-policy-registration`, `create-ipam-scope`, `create-key-pair`, `create-launch-template`
- `create-launch-template-version`, `create-local-gateway-route`, `create-local-gateway-route-table`, `create-local-gateway-route-table-virtual-interface-group-association`, `create-local-gateway-route-table-vpc-association`
- `create-local-gateway-virtual-interface`, `create-local-gateway-virtual-interface-group`, `create-mac-system-integrity-protection-modification-task`, `create-managed-prefix-list`, `create-nat-gateway`
- `create-network-acl`, `create-network-acl-entry`, `create-network-insights-access-scope`, `create-network-insights-path`, `create-network-interface`
- `create-network-interface-permission`, `create-placement-group`, `create-public-ipv4-pool`, `create-replace-root-volume-task`, `create-reserved-instances-listing`
- `create-restore-image-task`, `create-route`, `create-route-server`, `create-route-server-endpoint`, `create-route-server-peer`
- `create-route-table`, `create-secondary-network`, `create-secondary-subnet`, `create-security-group`, `create-snapshot`
- `create-snapshots`, `create-spot-datafeed-subscription`, `create-store-image-task`, `create-subnet`, `create-subnet-cidr-reservation`
- `create-tags`, `create-traffic-mirror-filter`, `create-traffic-mirror-filter-rule`, `create-traffic-mirror-session`, `create-traffic-mirror-target`
- `create-transit-gateway`, `create-transit-gateway-connect`, `create-transit-gateway-connect-peer`, `create-transit-gateway-metering-policy`, `create-transit-gateway-metering-policy-entry`
- `create-transit-gateway-multicast-domain`, `create-transit-gateway-peering-attachment`, `create-transit-gateway-policy-table`, `create-transit-gateway-policy-table-entry`, `create-transit-gateway-prefix-list-reference`
- `create-transit-gateway-route`, `create-transit-gateway-route-table`, `create-transit-gateway-route-table-announcement`, `create-transit-gateway-vpc-attachment`, `create-verified-access-endpoint`
- `create-verified-access-group`, `create-verified-access-instance`, `create-verified-access-trust-provider`, `create-volume`, `create-vpc`
- `create-vpc-block-public-access-exclusion`, `create-vpc-encryption-control`, `create-vpc-endpoint`, `create-vpc-endpoint-connection-notification`, `create-vpc-endpoint-service-configuration`
- `create-vpc-peering-connection`, `create-vpn-concentrator`, `create-vpn-connection`, `create-vpn-connection-route`, `create-vpn-gateway`
- `delete-application-status-check`, `delete-capacity-manager-data-export`, `delete-carrier-gateway`, `delete-client-vpn-endpoint`, `delete-client-vpn-endpoint-authorization-policy`
- `delete-client-vpn-route`, `delete-coip-cidr`, `delete-coip-pool`, `delete-customer-gateway`, `delete-dhcp-options`
- `delete-egress-only-internet-gateway`, `delete-fleets`, `delete-flow-logs`, `delete-fpga-image`, `delete-image-usage-report`
- `delete-instance-connect-endpoint`, `delete-instance-event-window`, `delete-internet-gateway`, `delete-ipam`, `delete-ipam-external-resource-verification-token`
- `delete-ipam-internet-registry-association`, `delete-ipam-policy`, `delete-ipam-pool`, `delete-ipam-prefix-list-resolver`, `delete-ipam-prefix-list-resolver-target`
- `delete-ipam-resource-discovery`, `delete-ipam-routing-policy-registration`, `delete-ipam-scope`, `delete-key-pair`, `delete-launch-template`
- `delete-launch-template-versions`, `delete-local-gateway-route`, `delete-local-gateway-route-table`, `delete-local-gateway-route-table-virtual-interface-group-association`, `delete-local-gateway-route-table-vpc-association`
- `delete-local-gateway-virtual-interface`, `delete-local-gateway-virtual-interface-group`, `delete-managed-prefix-list`, `delete-nat-gateway`, `delete-network-acl`
- `delete-network-acl-entry`, `delete-network-insights-access-scope`, `delete-network-insights-access-scope-analysis`, `delete-network-insights-analysis`, `delete-network-insights-path`
- `delete-network-interface`, `delete-network-interface-permission`, `delete-placement-group`, `delete-public-ipv4-pool`, `delete-queued-reserved-instances`
- `delete-route`, `delete-route-server`, `delete-route-server-endpoint`, `delete-route-server-peer`, `delete-route-table`
- `delete-secondary-network`, `delete-secondary-subnet`, `delete-security-group`, `delete-snapshot`, `delete-spot-datafeed-subscription`
- `delete-subnet`, `delete-subnet-cidr-reservation`, `delete-tags`, `delete-traffic-mirror-filter`, `delete-traffic-mirror-filter-rule`
- `delete-traffic-mirror-session`, `delete-traffic-mirror-target`, `delete-transit-gateway`, `delete-transit-gateway-client-vpn-attachment`, `delete-transit-gateway-connect`
- `delete-transit-gateway-connect-peer`, `delete-transit-gateway-metering-policy`, `delete-transit-gateway-metering-policy-entry`, `delete-transit-gateway-multicast-domain`, `delete-transit-gateway-peering-attachment`
- `delete-transit-gateway-policy-table`, `delete-transit-gateway-policy-table-entry`, `delete-transit-gateway-prefix-list-reference`, `delete-transit-gateway-route`, `delete-transit-gateway-route-table`
- `delete-transit-gateway-route-table-announcement`, `delete-transit-gateway-vpc-attachment`, `delete-verified-access-endpoint`, `delete-verified-access-group`, `delete-verified-access-instance`
- `delete-verified-access-trust-provider`, `delete-volume`, `delete-vpc`, `delete-vpc-block-public-access-exclusion`, `delete-vpc-encryption-control`
- `delete-vpc-endpoint-connection-notifications`, `delete-vpc-endpoint-service-configurations`, `delete-vpc-endpoints`, `delete-vpc-peering-connection`, `delete-vpn-concentrator`
- `delete-vpn-connection`, `delete-vpn-connection-route`, `delete-vpn-gateway`, `deprovision-byoip-cidr`, `deprovision-ipam-byoasn`
- `deprovision-ipam-pool-cidr`, `deprovision-public-ipv4-pool-cidr`, `deregister-image`, `deregister-instance-event-notification-attributes`, `deregister-transit-gateway-multicast-group-members`
- `deregister-transit-gateway-multicast-group-sources`, `describe-account-attributes`, `describe-account-vpc-encryption-control`, `describe-address-transfers`, `describe-addresses`
- `describe-addresses-attribute`, `describe-aggregate-id-format`, `describe-application-status`, `describe-application-status-check-associations`, `describe-application-status-checks`
- `describe-availability-zones`, `describe-aws-network-performance-metric-subscriptions`, `describe-bundle-tasks`, `describe-byoip-cidrs`, `describe-capacity-block-extension-history`
- `describe-capacity-block-extension-offerings`, `describe-capacity-block-offerings`, `describe-capacity-block-status`, `describe-capacity-blocks`, `describe-capacity-manager-data-exports`
- `describe-capacity-reservation-billing-requests`, `describe-capacity-reservation-cancellation-quotes`, `describe-capacity-reservation-date-change-quotes`, `describe-capacity-reservation-fleets`, `describe-capacity-reservation-topology`
- `describe-capacity-reservations`, `describe-carrier-gateways`, `describe-classic-link-instances`, `describe-client-vpn-authorization-rules`, `describe-client-vpn-connections`
- `describe-client-vpn-endpoints`, `describe-client-vpn-routes`, `describe-client-vpn-target-networks`, `describe-coip-pools`, `describe-conversion-tasks`
- `describe-customer-gateways`, `describe-declarative-policies-reports`, `describe-dhcp-options`, `describe-egress-only-internet-gateways`, `describe-elastic-gpus`
- `describe-export-image-tasks`, `describe-export-tasks`, `describe-fast-launch-images`, `describe-fast-snapshot-restores`, `describe-fleet-history`
- `describe-fleet-instances`, `describe-fleets`, `describe-flow-logs`, `describe-fpga-image-attribute`, `describe-fpga-images`
- `describe-host-reservation-offerings`, `describe-host-reservations`, `describe-hosts`, `describe-iam-instance-profile-associations`, `describe-id-format`
- `describe-identity-id-format`, `describe-image-attribute`, `describe-image-references`, `describe-image-usage-report-entries`, `describe-image-usage-reports`
- `describe-images`, `describe-import-image-tasks`, `describe-import-snapshot-tasks`, `describe-instance-attribute`, `describe-instance-connect-endpoints`
- `describe-instance-credit-specifications`, `describe-instance-event-notification-attributes`, `describe-instance-event-windows`, `describe-instance-image-metadata`, `describe-instance-sql-ha-history-states`
- `describe-instance-sql-ha-states`, `describe-instance-status`, `describe-instance-topology`, `describe-instance-type-offerings`, `describe-instance-types`
- `describe-instances`, `describe-internet-gateways`, `describe-ipam-byoasn`, `describe-ipam-external-resource-verification-tokens`, `describe-ipam-internet-registry-associations`
- `describe-ipam-policies`, `describe-ipam-pool-allocations`, `describe-ipam-pools`, `describe-ipam-prefix-list-resolver-targets`, `describe-ipam-prefix-list-resolvers`
- `describe-ipam-resource-discoveries`, `describe-ipam-resource-discovery-associations`, `describe-ipam-scopes`, `describe-ipams`, `describe-ipv6-pools`
- `describe-key-pairs`, `describe-launch-template-versions`, `describe-launch-templates`, `describe-local-gateway-route-table-virtual-interface-group-associations`, `describe-local-gateway-route-table-vpc-associations`
- `describe-local-gateway-route-tables`, `describe-local-gateway-virtual-interface-groups`, `describe-local-gateway-virtual-interfaces`, `describe-local-gateways`, `describe-locked-snapshots`
- `describe-mac-hosts`, `describe-mac-modification-tasks`, `describe-managed-prefix-lists`, `describe-moving-addresses`, `describe-nat-gateways`
- `describe-network-acls`, `describe-network-insights-access-scope-analyses`, `describe-network-insights-access-scopes`, `describe-network-insights-analyses`, `describe-network-insights-paths`
- `describe-network-interface-attribute`, `describe-network-interface-permissions`, `describe-network-interfaces`, `describe-outpost-lags`, `describe-placement-groups`
- `describe-prefix-lists`, `describe-principal-id-format`, `describe-public-ipv4-pools`, `describe-regions`, `describe-replace-root-volume-tasks`
- `describe-reserved-instances`, `describe-reserved-instances-listings`, `describe-reserved-instances-modifications`, `describe-reserved-instances-offerings`, `describe-route-server-endpoints`
- `describe-route-server-peers`, `describe-route-servers`, `describe-route-tables`, `describe-scheduled-instance-availability`, `describe-scheduled-instances`
- `describe-secondary-interfaces`, `describe-secondary-networks`, `describe-secondary-subnets`, `describe-security-group-references`, `describe-security-group-rules`
- `describe-security-group-vpc-associations`, `describe-security-groups`, `describe-service-link-virtual-interfaces`, `describe-snapshot-attribute`, `describe-snapshot-tier-status`
- `describe-snapshots`, `describe-spot-datafeed-subscription`, `describe-spot-fleet-instances`, `describe-spot-fleet-request-history`, `describe-spot-fleet-requests`
- `describe-spot-instance-requests`, `describe-spot-price-history`, `describe-stale-security-groups`, `describe-store-image-tasks`, `describe-subnets`
- `describe-tags`, `describe-traffic-mirror-filter-rules`, `describe-traffic-mirror-filters`, `describe-traffic-mirror-sessions`, `describe-traffic-mirror-targets`
- `describe-transit-gateway-attachments`, `describe-transit-gateway-connect-peers`, `describe-transit-gateway-connects`, `describe-transit-gateway-metering-policies`, `describe-transit-gateway-multicast-domains`
- `describe-transit-gateway-peering-attachments`, `describe-transit-gateway-policy-tables`, `describe-transit-gateway-route-table-announcements`, `describe-transit-gateway-route-tables`, `describe-transit-gateway-vpc-attachments`
- `describe-transit-gateways`, `describe-trunk-interface-associations`, `describe-verified-access-endpoints`, `describe-verified-access-groups`, `describe-verified-access-instance-logging-configurations`
- `describe-verified-access-instances`, `describe-verified-access-trust-providers`, `describe-volume-attribute`, `describe-volume-status`, `describe-volumes`
- `describe-volumes-modifications`, `describe-vpc-attribute`, `describe-vpc-block-public-access-exclusions`, `describe-vpc-block-public-access-options`, `describe-vpc-classic-link`
- `describe-vpc-classic-link-dns-support`, `describe-vpc-encryption-controls`, `describe-vpc-endpoint-associations`, `describe-vpc-endpoint-connection-notifications`, `describe-vpc-endpoint-connections`
- `describe-vpc-endpoint-service-configurations`, `describe-vpc-endpoint-service-permissions`, `describe-vpc-endpoint-services`, `describe-vpc-endpoints`, `describe-vpc-peering-connections`
- `describe-vpcs`, `describe-vpn-concentrators`, `describe-vpn-connections`, `describe-vpn-gateways`, `detach-classic-link-vpc`
- `detach-image-watermark`, `detach-internet-gateway`, `detach-network-interface`, `detach-verified-access-trust-provider`, `detach-volume`
- `detach-vpn-gateway`, `disable-address-transfer`, `disable-allowed-images-settings`, `disable-application-status-check-suppression`, `disable-aws-network-performance-metric-subscription`
- `disable-capacity-manager`, `disable-ebs-encryption-by-default`, `disable-fast-launch`, `disable-fast-snapshot-restores`, `disable-image`
- `disable-image-block-public-access`, `disable-image-deprecation`, `disable-image-deregistration-protection`, `disable-instance-sql-ha-standby-detections`, `disable-ipam-organization-admin-account`
- `disable-ipam-policy`, `disable-route-server-propagation`, `disable-serial-console-access`, `disable-snapshot-block-public-access`, `disable-transit-gateway-route-table-propagation`
- `disable-vgw-route-propagation`, `disable-vpc-classic-link`, `disable-vpc-classic-link-dns-support`, `disassociate-address`, `disassociate-application-status-check`
- `disassociate-capacity-reservation-billing-owner`, `disassociate-client-vpn-target-network`, `disassociate-enclave-certificate-iam-role`, `disassociate-iam-instance-profile`, `disassociate-instance-event-window`
- `disassociate-ipam-byoasn`, `disassociate-ipam-resource-discovery`, `disassociate-nat-gateway-address`, `disassociate-route-server`, `disassociate-route-table`
- `disassociate-security-group-vpc`, `disassociate-subnet-cidr-block`, `disassociate-transit-gateway-multicast-domain`, `disassociate-transit-gateway-policy-table`, `disassociate-transit-gateway-route-table`
- `disassociate-trunk-interface`, `disassociate-vpc-cidr-block`, `enable-address-transfer`, `enable-allowed-images-settings`, `enable-application-status-check-suppression`
- `enable-aws-network-performance-metric-subscription`, `enable-capacity-manager`, `enable-ebs-encryption-by-default`, `enable-fast-launch`, `enable-fast-snapshot-restores`
- `enable-image`, `enable-image-block-public-access`, `enable-image-deprecation`, `enable-image-deregistration-protection`, `enable-instance-sql-ha-standby-detections`
- `enable-ipam-internet-registry-association`, `enable-ipam-organization-admin-account`, `enable-ipam-policy`, `enable-reachability-analyzer-organization-sharing`, `enable-route-server-propagation`
- `enable-serial-console-access`, `enable-snapshot-block-public-access`, `enable-transit-gateway-route-table-propagation`, `enable-vgw-route-propagation`, `enable-volume-io`
- `enable-vpc-classic-link`, `enable-vpc-classic-link-dns-support`, `export-client-vpn-client-certificate-revocation-list`, `export-client-vpn-client-configuration`, `export-image`
- `export-transit-gateway-routes`, `export-verified-access-instance-client-configuration`, `get-active-vpn-tunnel-status`, `get-allowed-images-settings`, `get-associated-enclave-certificate-iam-roles`
- `get-associated-ipv6-pool-cidrs`, `get-aws-network-performance-data`, `get-capacity-manager-attributes`, `get-capacity-manager-metric-data`, `get-capacity-manager-metric-dimensions`
- `get-capacity-manager-monitored-tag-keys`, `get-capacity-reservation-usage`, `get-client-vpn-endpoint-authorization-policy`, `get-coip-pool-usage`, `get-console-output`
- `get-console-screenshot`, `get-declarative-policies-report-summary`, `get-default-credit-specification`, `get-ebs-default-kms-key-id`, `get-ebs-encryption-by-default`
- `get-enabled-ipam-policy`, `get-flow-logs-integration-template`, `get-groups-for-capacity-reservation`, `get-host-reservation-purchase-preview`, `get-image-ancestry`
- `get-image-block-public-access-state`, `get-instance-metadata-defaults`, `get-instance-tpm-ek-pub`, `get-instance-types-from-instance-requirements`, `get-instance-uefi-data`
- `get-ipam-address-history`, `get-ipam-discovered-accounts`, `get-ipam-discovered-public-addresses`, `get-ipam-discovered-resource-cidrs`, `get-ipam-discovered-routes`
- `get-ipam-internet-registry-association-asns`, `get-ipam-internet-registry-association-cidrs`, `get-ipam-policy-allocation-rules`, `get-ipam-policy-organization-targets`, `get-ipam-pool-allocations`
- `get-ipam-pool-cidrs`, `get-ipam-prefix-list-resolver-rules`, `get-ipam-prefix-list-resolver-version-entries`, `get-ipam-prefix-list-resolver-versions`, `get-ipam-resource-cidrs`
- `get-ipam-route-origin-authorizations`, `get-ipam-route-protection-findings`, `get-ipam-routing-policy-registration-deltas`, `get-ipam-routing-policy-registrations`, `get-launch-template-data`
- `get-managed-prefix-list-associations`, `get-managed-prefix-list-entries`, `get-managed-resource-visibility`, `get-network-insights-access-scope-analysis-findings`, `get-network-insights-access-scope-content`
- `get-password-data`, `get-reserved-instances-exchange-quote`, `get-route-server-associations`, `get-route-server-propagations`, `get-route-server-routing-database`
- `get-security-groups-for-vpc`, `get-serial-console-access-status`, `get-snapshot-block-public-access-state`, `get-spot-placement-scores`, `get-subnet-cidr-reservations`
- `get-transit-gateway-attachment-propagations`, `get-transit-gateway-metering-policy-entries`, `get-transit-gateway-multicast-domain-associations`, `get-transit-gateway-policy-table-associations`, `get-transit-gateway-policy-table-entries`
- `get-transit-gateway-prefix-list-references`, `get-transit-gateway-route-table-associations`, `get-transit-gateway-route-table-propagations`, `get-verified-access-endpoint-policy`, `get-verified-access-endpoint-targets`
- `get-verified-access-group-policy`, `get-vpc-resources-blocking-encryption-enforcement`, `get-vpn-connection-device-sample-configuration`, `get-vpn-connection-device-types`, `get-vpn-tunnel-replacement-status`
- `import-client-vpn-client-certificate-revocation-list`, `import-image`, `import-key-pair`, `import-snapshot`, `list-images-in-recycle-bin`
- `list-snapshots-in-recycle-bin`, `list-volumes-in-recycle-bin`, `lock-snapshot`, `modify-account-vpc-encryption-control`, `modify-address-attribute`
- `modify-application-status-check`, `modify-availability-zone-group`, `modify-capacity-reservation`, `modify-capacity-reservation-fleet`, `modify-client-vpn-endpoint`
- `modify-client-vpn-endpoint-authorization-policy`, `modify-default-credit-specification`, `modify-ebs-default-kms-key-id`, `modify-fleet`, `modify-fpga-image-attribute`
- `modify-hosts`, `modify-id-format`, `modify-identity-id-format`, `modify-image-attribute`, `modify-instance-attribute`
- `modify-instance-capacity-reservation-attributes`, `modify-instance-connect-endpoint`, `modify-instance-cpu-options`, `modify-instance-credit-specification`, `modify-instance-event-start-time`
- `modify-instance-event-window`, `modify-instance-maintenance-options`, `modify-instance-metadata-defaults`, `modify-instance-metadata-options`, `modify-instance-network-performance-options`
- `modify-instance-placement`, `modify-ipam`, `modify-ipam-policy-allocation-rules`, `modify-ipam-pool`, `modify-ipam-pool-allocation`
- `modify-ipam-prefix-list-resolver`, `modify-ipam-prefix-list-resolver-target`, `modify-ipam-resource-cidr`, `modify-ipam-resource-discovery`, `modify-ipam-routing-policy-registration`
- `modify-ipam-scope`, `modify-launch-template`, `modify-local-gateway-route`, `modify-managed-prefix-list`, `modify-managed-resource-visibility`
- `modify-network-interface-attribute`, `modify-private-dns-name-options`, `modify-public-ip-dns-name-options`, `modify-reserved-instances`, `modify-route-server`
- `modify-security-group-rules`, `modify-snapshot-attribute`, `modify-snapshot-tier`, `modify-spot-fleet-request`, `modify-subnet-attribute`
- `modify-traffic-mirror-filter-network-services`, `modify-traffic-mirror-filter-rule`, `modify-traffic-mirror-session`, `modify-transit-gateway`, `modify-transit-gateway-metering-policy`
- `modify-transit-gateway-policy-table-entry`, `modify-transit-gateway-prefix-list-reference`, `modify-transit-gateway-vpc-attachment`, `modify-verified-access-endpoint`, `modify-verified-access-endpoint-policy`
- `modify-verified-access-group`, `modify-verified-access-group-policy`, `modify-verified-access-instance`, `modify-verified-access-instance-logging-configuration`, `modify-verified-access-trust-provider`
- `modify-volume`, `modify-volume-attribute`, `modify-vpc-attribute`, `modify-vpc-block-public-access-exclusion`, `modify-vpc-block-public-access-options`
- `modify-vpc-encryption-control`, `modify-vpc-endpoint`, `modify-vpc-endpoint-connection-notification`, `modify-vpc-endpoint-payer-responsibility`, `modify-vpc-endpoint-service-configuration`
- `modify-vpc-endpoint-service-payer-responsibility`, `modify-vpc-endpoint-service-permissions`, `modify-vpc-peering-connection-options`, `modify-vpc-tenancy`, `modify-vpn-connection`
- `modify-vpn-connection-options`, `modify-vpn-tunnel-certificate`, `modify-vpn-tunnel-options`, `monitor-instances`, `move-address-to-vpc`
- `move-byoip-cidr-to-ipam`, `move-capacity-reservation-instances`, `provision-byoip-cidr`, `provision-ipam-byoasn`, `provision-ipam-pool-cidr`
- `provision-public-ipv4-pool-cidr`, `purchase-capacity-block`, `purchase-capacity-block-extension`, `purchase-host-reservation`, `purchase-reserved-instances-offering`
- `purchase-scheduled-instances`, `reboot-instances`, `register-image`, `register-instance-event-notification-attributes`, `register-transit-gateway-multicast-group-members`
- `register-transit-gateway-multicast-group-sources`, `reject-capacity-reservation-billing-ownership`, `reject-transit-gateway-client-vpn-attachment`, `reject-transit-gateway-multicast-domain-associations`, `reject-transit-gateway-peering-attachment`
- `reject-transit-gateway-vpc-attachment`, `reject-vpc-endpoint-connections`, `reject-vpc-peering-connection`, `release-address`, `release-hosts`
- `release-ipam-pool-allocation`, `replace-iam-instance-profile-association`, `replace-image-criteria-in-allowed-images-settings`, `replace-image-instance-type-specification`, `replace-network-acl-association`
- `replace-network-acl-entry`, `replace-route`, `replace-route-table-association`, `replace-transit-gateway-route`, `replace-vpn-tunnel`
- `report-instance-status`, `request-spot-fleet`, `request-spot-instances`, `reset-address-attribute`, `reset-ebs-default-kms-key-id`
- `reset-fpga-image-attribute`, `reset-image-attribute`, `reset-instance-attribute`, `reset-network-interface-attribute`, `reset-snapshot-attribute`
- `restore-address-to-classic`, `restore-image-from-recycle-bin`, `restore-managed-prefix-list-version`, `restore-snapshot-from-recycle-bin`, `restore-snapshot-tier`
- `restore-volume-from-recycle-bin`, `revoke-client-vpn-ingress`, `revoke-security-group-egress`, `revoke-security-group-ingress`, `run-instances`
- `run-scheduled-instances`, `search-local-gateway-routes`, `search-transit-gateway-multicast-groups`, `search-transit-gateway-routes`, `send-diagnostic-interrupt`
- `start-declarative-policies-report`, `start-instances`, `start-network-insights-access-scope-analysis`, `start-network-insights-analysis`, `start-vpc-endpoint-service-private-dns-verification`
- `stop-instances`, `terminate-client-vpn-connections`, `terminate-instances`, `unassign-ipv6-addresses`, `unassign-private-ip-addresses`
- `unassign-private-nat-gateway-address`, `unlock-snapshot`, `unmonitor-instances`, `update-capacity-manager-monitored-tag-keys`, `update-capacity-manager-organizations-access`
- `update-interruptible-capacity-reservation-allocation`, `update-security-group-rule-descriptions-egress`, `update-security-group-rule-descriptions-ingress`, `validate-security-group-quotas-for-interface`, `wait`
- `withdraw-byoip-cidr`

### `aws ecs` — 80 command groups

[Official command reference](https://docs.aws.amazon.com/cli/latest/reference/ecs/).

- `continue-service-deployment`, `create-capacity-provider`, `create-cluster`, `create-daemon`, `create-express-gateway-service`
- `create-service`, `create-task-set`, `delete-account-setting`, `delete-attributes`, `delete-capacity-provider`
- `delete-cluster`, `delete-daemon`, `delete-daemon-task-definition`, `delete-express-gateway-service`, `delete-service`
- `delete-task-definitions`, `delete-task-set`, `deploy`, `deregister-container-instance`, `deregister-task-definition`
- `describe-capacity-providers`, `describe-clusters`, `describe-container-instances`, `describe-daemon`, `describe-daemon-deployments`
- `describe-daemon-revisions`, `describe-daemon-task-definition`, `describe-express-gateway-service`, `describe-service-deployments`, `describe-service-revisions`
- `describe-services`, `describe-task-definition`, `describe-task-sets`, `describe-tasks`, `discover-poll-endpoint`
- `execute-command`, `get-task-protection`, `list-account-settings`, `list-attributes`, `list-clusters`
- `list-container-instances`, `list-daemon-deployments`, `list-daemon-task-definitions`, `list-daemons`, `list-service-deployments`
- `list-services`, `list-services-by-namespace`, `list-tags-for-resource`, `list-task-definition-families`, `list-task-definitions`
- `list-tasks`, `monitor-express-gateway-service`, `put-account-setting`, `put-account-setting-default`, `put-attributes`
- `put-cluster-capacity-providers`, `register-container-instance`, `register-daemon-task-definition`, `register-task-definition`, `run-task`
- `start-task`, `stop-service-deployment`, `stop-task`, `submit-attachment-state-changes`, `submit-container-state-change`
- `submit-task-state-change`, `tag-resource`, `untag-resource`, `update-capacity-provider`, `update-cluster`
- `update-cluster-settings`, `update-container-agent`, `update-container-instances-state`, `update-daemon`, `update-express-gateway-service`
- `update-service`, `update-service-primary-task-set`, `update-task-protection`, `update-task-set`, `wait`

### `aws ecr` — 60 command groups

[Official command reference](https://docs.aws.amazon.com/cli/latest/reference/ecr/).

- `batch-check-layer-availability`, `batch-delete-image`, `batch-get-image`, `batch-get-repository-scanning-configuration`, `complete-layer-upload`
- `create-pull-through-cache-rule`, `create-repository`, `create-repository-creation-template`, `delete-lifecycle-policy`, `delete-pull-through-cache-rule`
- `delete-registry-policy`, `delete-repository`, `delete-repository-creation-template`, `delete-repository-policy`, `delete-signing-configuration`
- `deregister-pull-time-update-exclusion`, `describe-image-replication-status`, `describe-image-scan-findings`, `describe-image-signing-status`, `describe-images`
- `describe-pull-through-cache-rules`, `describe-registry`, `describe-repositories`, `describe-repository-creation-templates`, `get-account-setting`
- `get-authorization-token`, `get-download-url-for-layer`, `get-lifecycle-policy`, `get-lifecycle-policy-preview`, `get-login-password`
- `get-registry-policy`, `get-registry-scanning-configuration`, `get-repository-policy`, `get-signing-configuration`, `initiate-layer-upload`
- `list-image-referrers`, `list-images`, `list-pull-time-update-exclusions`, `list-tags-for-resource`, `put-account-setting`
- `put-image`, `put-image-scanning-configuration`, `put-image-tag-mutability`, `put-lifecycle-policy`, `put-registry-policy`
- `put-registry-scanning-configuration`, `put-replication-configuration`, `put-signing-configuration`, `register-pull-time-update-exclusion`, `set-repository-policy`
- `start-image-scan`, `start-lifecycle-policy-preview`, `tag-resource`, `untag-resource`, `update-image-storage-class`
- `update-pull-through-cache-rule`, `update-repository-creation-template`, `upload-layer-part`, `validate-pull-through-cache-rule`, `wait`

### `aws s3api` — 113 command groups

[Official command reference](https://docs.aws.amazon.com/cli/latest/reference/s3api/).

- `abort-multipart-upload`, `complete-multipart-upload`, `copy-object`, `create-bucket`, `create-bucket-metadata-configuration`
- `create-bucket-metadata-table-configuration`, `create-multipart-upload`, `create-session`, `delete-bucket`, `delete-bucket-analytics-configuration`
- `delete-bucket-cors`, `delete-bucket-encryption`, `delete-bucket-intelligent-tiering-configuration`, `delete-bucket-inventory-configuration`, `delete-bucket-lifecycle`
- `delete-bucket-metadata-configuration`, `delete-bucket-metadata-table-configuration`, `delete-bucket-metrics-configuration`, `delete-bucket-ownership-controls`, `delete-bucket-policy`
- `delete-bucket-replication`, `delete-bucket-tagging`, `delete-bucket-website`, `delete-object`, `delete-object-annotation`
- `delete-object-tagging`, `delete-objects`, `delete-public-access-block`, `get-bucket-abac`, `get-bucket-accelerate-configuration`
- `get-bucket-acl`, `get-bucket-analytics-configuration`, `get-bucket-cors`, `get-bucket-encryption`, `get-bucket-intelligent-tiering-configuration`
- `get-bucket-inventory-configuration`, `get-bucket-lifecycle-configuration`, `get-bucket-location`, `get-bucket-logging`, `get-bucket-metadata-configuration`
- `get-bucket-metadata-table-configuration`, `get-bucket-metrics-configuration`, `get-bucket-notification-configuration`, `get-bucket-ownership-controls`, `get-bucket-policy`
- `get-bucket-policy-status`, `get-bucket-replication`, `get-bucket-request-payment`, `get-bucket-tagging`, `get-bucket-versioning`
- `get-bucket-website`, `get-object`, `get-object-acl`, `get-object-annotation`, `get-object-attributes`
- `get-object-legal-hold`, `get-object-lock-configuration`, `get-object-retention`, `get-object-tagging`, `get-object-torrent`
- `get-public-access-block`, `head-bucket`, `head-object`, `list-bucket-analytics-configurations`, `list-bucket-intelligent-tiering-configurations`
- `list-bucket-inventory-configurations`, `list-bucket-metrics-configurations`, `list-buckets`, `list-directory-buckets`, `list-multipart-uploads`
- `list-object-annotations`, `list-object-versions`, `list-objects`, `list-objects-v2`, `list-parts`
- `put-bucket-abac`, `put-bucket-accelerate-configuration`, `put-bucket-acl`, `put-bucket-analytics-configuration`, `put-bucket-cors`
- `put-bucket-encryption`, `put-bucket-intelligent-tiering-configuration`, `put-bucket-inventory-configuration`, `put-bucket-lifecycle-configuration`, `put-bucket-logging`
- `put-bucket-metrics-configuration`, `put-bucket-notification-configuration`, `put-bucket-ownership-controls`, `put-bucket-policy`, `put-bucket-replication`
- `put-bucket-request-payment`, `put-bucket-tagging`, `put-bucket-versioning`, `put-bucket-website`, `put-object`
- `put-object-acl`, `put-object-annotation`, `put-object-legal-hold`, `put-object-lock-configuration`, `put-object-retention`
- `put-object-tagging`, `put-public-access-block`, `rename-object`, `restore-object`, `select-object-content`
- `update-bucket-metadata-annotation-table-configuration`, `update-bucket-metadata-inventory-table-configuration`, `update-bucket-metadata-journal-table-configuration`, `update-object-encryption`, `upload-part`
- `upload-part-copy`, `wait`, `write-get-object-response`

### `aws s3` — 9 command groups

[Official command reference](https://docs.aws.amazon.com/cli/latest/reference/s3/).

- `cp`, `ls`, `mb`, `mv`, `presign`
- `rb`, `rm`, `sync`, `website`

### `aws lambda` — 90 command groups

[Official command reference](https://docs.aws.amazon.com/cli/latest/reference/lambda/).

- `add-layer-version-permission`, `add-permission`, `checkpoint-durable-execution`, `create-alias`, `create-capacity-provider`
- `create-code-signing-config`, `create-event-source-mapping`, `create-function`, `create-function-url-config`, `delete-alias`
- `delete-capacity-provider`, `delete-code-signing-config`, `delete-event-source-mapping`, `delete-function`, `delete-function-code-signing-config`
- `delete-function-concurrency`, `delete-function-event-invoke-config`, `delete-function-url-config`, `delete-layer-version`, `delete-provisioned-concurrency-config`
- `delete-resource-policy`, `get-account-settings`, `get-alias`, `get-capacity-provider`, `get-code-signing-config`
- `get-durable-execution`, `get-durable-execution-history`, `get-durable-execution-state`, `get-event-source-mapping`, `get-function`
- `get-function-code-signing-config`, `get-function-concurrency`, `get-function-configuration`, `get-function-event-invoke-config`, `get-function-recursion-config`
- `get-function-scaling-config`, `get-function-url-config`, `get-layer-version`, `get-layer-version-by-arn`, `get-layer-version-policy`
- `get-policy`, `get-provisioned-concurrency-config`, `get-resource-policy`, `get-runtime-management-config`, `invoke`
- `list-aliases`, `list-capacity-providers`, `list-code-signing-configs`, `list-durable-executions-by-function`, `list-event-source-mappings`
- `list-function-event-invoke-configs`, `list-functions`, `list-functions-by-code-signing-config`, `list-function-url-configs`, `list-function-versions-by-capacity-provider`
- `list-layers`, `list-layer-versions`, `list-provisioned-concurrency-configs`, `list-tags`, `list-versions-by-function`
- `publish-layer-version`, `publish-version`, `put-function-code-signing-config`, `put-function-concurrency`, `put-function-event-invoke-config`
- `put-function-recursion-config`, `put-function-scaling-config`, `put-provisioned-concurrency-config`, `put-resource-policy`, `put-runtime-management-config`
- `remove-layer-version-permission`, `remove-permission`, `send-durable-execution-callback-failure`, `send-durable-execution-callback-heartbeat`, `send-durable-execution-callback-success`
- `stop-durable-execution`, `tag-resource`, `untag-resource`, `update-alias`, `update-capacity-provider`
- `update-code-signing-config`, `update-event-source-mapping`, `update-function-code`, `update-function-configuration`, `update-function-event-invoke-config`
- `update-function-url-config`, `wait`, `wizard`

