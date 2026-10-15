# Service guide

The AWS CLI supplies credentials. Lazy AWS passes the profile and region to
each command. It disables the pager and prompts. The AWS CLI returns JSON.
Query errors appear in the panes and command log.

The header shows the AWS account ID and short principal. Focus Status with `1`
to see the full ARN, user ID, profile, and region in Details. Lazy AWS gets the
identity with [`sts get-caller-identity`](https://docs.aws.amazon.com/cli/latest/reference/sts/get-caller-identity.html).
AWS does not require an IAM permission for this call.

## Resource panes

| Service | Panes after Status | Read operations |
| --- | --- | --- |
| EC2 | Instances, Volumes, Security groups, Tags | DescribeInstances, DescribeVolumes, DescribeSecurityGroups |
| ECS | Clusters, Services, Tasks, Task definitions | ListClusters, DescribeClusters, ListServices, DescribeServices, ListTasks, DescribeTasks, DescribeTaskDefinition |
| ECR | Repositories, Images, Image tags, Repository tags | DescribeRepositories, DescribeImages, ListTagsForResource |
| S3 | Buckets, Objects, Prefixes, Bucket settings | ListBuckets, ListObjectsV2, GetBucketLocation, GetBucketVersioning |
| Lambda | Functions, Versions, Aliases, Tags | ListFunctions, ListVersionsByFunction, ListAliases, ListTags |

EC2 tags come from the selected instance response. ECR image tags come from the
image response. S3 prefixes come from an object list with a delimiter. ECS task
definitions come from the selected cluster's services and tasks. The task list
includes RUNNING and STOPPED tasks. S3 object and settings queries use the
selected bucket's region. Lazy AWS reads the bucket region from the bucket list.
It calls GetBucketLocation if the region is missing. Lambda versions, aliases,
and tags are scoped to the selected function. Lambda tag values come from the
`Tags` object returned by `list-tags`.

Use these read permissions:

- S3: `s3:ListAllMyBuckets`, `s3:ListBucket`, `s3:GetBucketLocation`, and
  `s3:GetBucketVersioning`.
- EC2 and ECR: the operation names listed above match their IAM action names.
- ECS: use `ecs:DescribeTaskDefinition` for task definition reads.
- Lambda: `lambda:ListFunctions`, `lambda:ListVersionsByFunction`,
  `lambda:ListAliases`, and `lambda:ListTags`. `lambda:GetFunction` also
  returns tags when the read action requests them.

## Actions

Focus a resource row. Press `a` or right-click the selected row to open its
action list. Choose an action with Up/Down and Enter, or click its row. Review
the target, warning, account, region, and command. Press `y` or click Run to
execute it. Press Esc or click Cancel to close the dialog. Read actions also
need confirmation. The preview shows full input; the command log hides JSON
request values because task overrides can contain sensitive data. An action may be missing when the resource state or region
does not support it.

The [AWS CLI action inventory](aws-cli-actions.md) lists commands by tab,
required input, limits, and commands that remain CLI-only. It also contains the
full command index from AWS CLI 2.38.0. Use a current AWS CLI v2 for deployment
history and rollback commands.

| Service and resource | Actions |
| --- | --- |
| EC2 instance | Start, stop, reboot, terminate, read status and console output, add tags. |
| EC2 volume | Create snapshot, delete when available, change size with JSON fields, read modification status, detach from the selected instance. |
| EC2 security group or tag | Add/remove inbound and outbound rules; delete group; add tags or update/remove the selected instance tag. |
| ECS cluster | Delete, update settings, list services and tasks. |
| ECS service | Start deployment, set task count, delete without force, deploy a specific task definition revision, list deployments, roll back an ongoing ECS deployment. |
| ECS task or task definition | Stop task, start a standalone task from its definition, read definition and tags, list definition revisions, deregister active definition. |
| ECR repository | Delete when empty, change tag mutability, get/set/delete policy and lifecycle policy, toggle legacy scan-on-push, add tags. |
| ECR image or image tag | Start scan, read findings, read metadata or manifest, delete digest or remove tag. |
| ECR repository tag | Remove tag, update value, add tags. |
| S3 bucket or object | Versioning, empty bucket deletion, upload/download, object deletion/metadata, read/replace tags, remove object tags, list versions by key prefix. |
| S3 prefix | List objects, versions, and incomplete multipart uploads with this prefix. |
| S3 bucket settings | Read versioning, encryption, public access block, policy, lifecycle, CORS, logging, ownership, replication, notifications, and tags; set encryption, lifecycle, CORS, tags, or versioning. |
| Lambda function | Invoke sync/async/with log tail, read function and configuration, read policy and concurrency, list versions/aliases/mappings/configs, publish version, update configuration, set/remove reserved concurrency, create alias, add tags, delete. |
| Lambda version | Invoke, read function and configuration, read/set/remove provisioned concurrency except on $LATEST, delete version except $LATEST. |
| Lambda alias | Invoke, read alias, move to version, read/set provisioned concurrency, delete alias. |
| Lambda tag | Remove tag, update value, add tags. |

JSON input must be a non-empty object. It cannot replace captured fields or
set `registryId`. AWS CLI checks request fields; AWS checks permissions and
state. For example, EC2 resize takes `{"Size":128}`; ECS cluster settings take
`{"settings":[{"name":"containerInsights","value":"enhanced"}]}`.
Read the inventory for other input examples. The selected tag value input
accepts an empty value and keeps the selected key.

ECS rollback needs the deployment ARN from List service deployments. The ARN
must match the selected service, account, and Region. Deploy task definition
revision accepts `family:revision` or a task definition ARN in the selected
service's account and Region. It starts a new deployment with that definition;
it does not restore all prior service settings. CodeDeploy and external
controllers do not show these ECS deployment actions.

Standalone task creation can add compute charges. Fargate and other `awsvpc`
tasks need `networkConfiguration`, with subnet IDs and optional security groups.
An API failure is shown as an error even if the AWS CLI exits successfully.
Some tasks can start before a partial failure. Check the cluster before retry.

EC2 detach requires an attached-volume entry for the selected instance. Root
volumes require a stopped instance. Unmount the file system before detach.
The resize action does not extend the guest file system. Security group rule
changes affect every resource that uses the group.

ECR mutability changes keep captured exclusion filters. Repository scan-on-push
is a deprecated API; registry scanning configuration remains CLI-only.
Repository policies use `force:false`. ECR manifest and deletion failures are
shown as action errors. Resource creation wizards, recursive deletes, force
deletes, image builds/pushes, and interactive sessions remain CLI-only.

Lambda invoke writes the function payload to a private temporary file, then
shows it in Details as `Payload`. A function error is reported as an action
error with a payload excerpt. Invoke with log tail decodes the last 4 KB of
logs into `LogResultDecoded`. The command log hides the `--payload` value
because event payloads can contain sensitive data. Sync payloads are limited
to 6 MB and async payloads to 1 MB. Updating configuration restarts the
function. Provisioned concurrency adds charges even without invocations.
Deleting a function removes its versions; remove event source mappings first.
Function creation, code updates, layer publish, and event mapping edits remain
CLI-only.

These are the IAM actions for the resource actions above. Grant only the
permissions that the operator needs:

- EC2: `ec2:StartInstances`, `ec2:StopInstances`, `ec2:RebootInstances`,
  `ec2:TerminateInstances`, `ec2:CreateSnapshot`, `ec2:DeleteVolume`,
  `ec2:DeleteSecurityGroup`, `ec2:DeleteTags`, `ec2:CreateTags`,
  `ec2:DescribeInstanceStatus`, `ec2:GetConsoleOutput`, `ec2:ModifyVolume`,
  `ec2:DescribeVolumesModifications`, `ec2:DetachVolume`,
  `ec2:AuthorizeSecurityGroupIngress`, `ec2:RevokeSecurityGroupIngress`,
  `ec2:AuthorizeSecurityGroupEgress`, `ec2:RevokeSecurityGroupEgress`.
- ECS: `ecs:DeleteCluster`, `ecs:UpdateClusterSettings`, `ecs:ListServices`,
  `ecs:ListTasks`, `ecs:UpdateService`, `ecs:DeleteService`,
  `ecs:ListServiceDeployments`, `ecs:StopServiceDeployment`, `ecs:StopTask`,
  `ecs:RunTask`, `ecs:DescribeTaskDefinition`, `ecs:ListTaskDefinitions`,
  `ecs:DeregisterTaskDefinition`. RunTask can also need `iam:PassRole` for the
  task and execution roles, and `ecs:TagResource` when tags are supplied.
- ECR: `ecr:DeleteRepository`, `ecr:PutImageTagMutability`,
  `ecr:GetRepositoryPolicy`, `ecr:SetRepositoryPolicy`,
  `ecr:DeleteRepositoryPolicy`, `ecr:GetLifecyclePolicy`,
  `ecr:PutLifecyclePolicy`, `ecr:DeleteLifecyclePolicy`,
  `ecr:PutImageScanningConfiguration`, `ecr:StartImageScan`,
  `ecr:DescribeImageScanFindings`, `ecr:DescribeImages`, `ecr:BatchGetImage`,
  `ecr:BatchDeleteImage`, `ecr:TagResource`, `ecr:UntagResource`.
- S3: `s3:PutBucketVersioning`, `s3:DeleteBucket`, `s3:PutObject`,
  `s3:GetObject`, `s3:DeleteObject`, `s3:GetBucketTagging`,
  `s3:PutBucketTagging`, `s3:GetObjectTagging`, `s3:PutObjectTagging`,
  `s3:DeleteObjectTagging`, `s3:ListBucket`, `s3:ListBucketVersions`,
  `s3:ListBucketMultipartUploads`, `s3:GetBucketVersioning`,
  `s3:GetEncryptionConfiguration`, `s3:PutEncryptionConfiguration`,
  `s3:GetBucketPublicAccessBlock`, `s3:GetBucketPolicy`,
  `s3:GetLifecycleConfiguration`, `s3:PutLifecycleConfiguration`,
  `s3:GetBucketCORS`, `s3:PutBucketCORS`, `s3:GetBucketLogging`,
  `s3:GetBucketOwnershipControls`, `s3:GetReplicationConfiguration`,
  `s3:GetBucketNotification`. HeadObject uses `s3:GetObject`; encrypted object
  reads and writes can also need KMS permissions.
- Lambda: `lambda:InvokeFunction`, `lambda:GetFunction`,
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
  `lambda:TagResource`, `lambda:UntagResource`, `lambda:DeleteFunction`.

See the AWS [S3 operation-to-IAM map](https://docs.aws.amazon.com/service-authorization/latest/reference/list_s3.html)
and [ECS operation-to-IAM map](https://docs.aws.amazon.com/service-authorization/latest/reference/list_ecs.html).
See the [Lambda operation-to-IAM map](https://docs.aws.amazon.com/service-authorization/latest/reference/list_lambda.html).

## S3 file transfers

The file field takes one local file path. A bucket upload uses the file name as
the key. An upload from an object row keeps that object's key and replaces its
current object. Lazy AWS uses a single PutObject request. AWS limits that
request to 5 GB; it does not use multipart upload. See the AWS guide for
[uploading objects](https://docs.aws.amazon.com/AmazonS3/latest/userguide/upload-objects.html).

A download fails if the destination exists. Lazy AWS writes to a private
temporary folder beside the destination. It then creates the destination
without replacing a file. It removes the temporary folder after the command.
The [GetObject command](https://docs.aws.amazon.com/cli/latest/reference/s3api/get-object.html)
uses the selected bucket's region.

Each AWS command has a 20-second time limit. A large transfer can time out.
An upload may finish in AWS even if Lazy AWS reports a timeout or is stopped.
Check the object before you try the upload again.

```text
Selected resource --> action catalog --> confirmed command --> AWS CLI
Local file ---------> PutObject -----------------------------> S3 object
S3 object ----------> GetObject --> temporary file --> new local file
```

Only the active service sends refresh commands. Changing the primary selection
starts a refresh. A pane with an error shows its last successful data if its
parent selection is still valid.

Large lists have a limit. Read the command log for truncation notices. On
Windows, a custom AWS wrapper's child processes are not stopped as a group. Use
the AWS executable directly on Windows.
