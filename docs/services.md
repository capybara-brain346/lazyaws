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

EC2 tags come from the selected instance response. ECR image tags come from the
image response. S3 prefixes come from an object list with a delimiter. ECS task
definitions come from the selected cluster's services and tasks. The task list
includes RUNNING and STOPPED tasks. S3 object and settings queries use the
selected bucket's region. Lazy AWS reads the bucket region from the bucket list.
It calls GetBucketLocation if the region is missing.

Use these read permissions:

- S3: `s3:ListAllMyBuckets`, `s3:ListBucket`, `s3:GetBucketLocation`, and
  `s3:GetBucketVersioning`.
- EC2 and ECR: the operation names listed above match their IAM action names.
- ECS: use `ecs:DescribeTaskDefinition` for task definition reads.

## Actions

Focus a resource row. Press `a` or right-click the selected row to open its
action list. Choose an action with Up/Down and Enter, or click its row. Review
the target, warning, account, region, and command. Press `y` or click Run to
execute it. Press Esc or click Cancel to close the dialog. Read actions also
need confirmation. An action may be missing when the resource state or region
does not support it.

| Service and resource | Actions |
| --- | --- |
| EC2 instance | [Start](https://docs.aws.amazon.com/cli/latest/reference/ec2/start-instances.html) when stopped; [stop](https://docs.aws.amazon.com/cli/latest/reference/ec2/stop-instances.html) or [reboot](https://docs.aws.amazon.com/cli/latest/reference/ec2/reboot-instances.html) when running; [terminate](https://docs.aws.amazon.com/cli/latest/reference/ec2/terminate-instances.html) unless already terminating or terminated. |
| EC2 volume | [Create snapshot](https://docs.aws.amazon.com/cli/latest/reference/ec2/create-snapshot.html); [delete](https://docs.aws.amazon.com/cli/latest/reference/ec2/delete-volume.html) when available. |
| EC2 security group or instance tag | [Delete security group](https://docs.aws.amazon.com/cli/latest/reference/ec2/delete-security-group.html); [remove instance tag](https://docs.aws.amazon.com/cli/latest/reference/ec2/delete-tags.html). |
| ECS cluster | [Delete cluster](https://docs.aws.amazon.com/cli/latest/reference/ecs/delete-cluster.html) when it has no services or running tasks. |
| ECS service | [Start deployment](https://docs.aws.amazon.com/cli/latest/reference/ecs/update-service.html); set desired task count; [delete without force](https://docs.aws.amazon.com/cli/latest/reference/ecs/delete-service.html). |
| ECS task or task definition | [Stop task](https://docs.aws.amazon.com/cli/latest/reference/ecs/stop-task.html) unless stopped; [deregister active task definition](https://docs.aws.amazon.com/cli/latest/reference/ecs/deregister-task-definition.html). |
| ECR repository | [Delete without force](https://docs.aws.amazon.com/cli/latest/reference/ecr/delete-repository.html) when empty; [change tag mutability](https://docs.aws.amazon.com/cli/latest/reference/ecr/put-image-tag-mutability.html). |
| ECR image | [Start scan](https://docs.aws.amazon.com/cli/latest/reference/ecr/start-image-scan.html); [view scan findings](https://docs.aws.amazon.com/cli/latest/reference/ecr/describe-image-scan-findings.html); [delete image by digest](https://docs.aws.amazon.com/cli/latest/reference/ecr/batch-delete-image.html); remove an image tag with the same [batch-delete-image](https://docs.aws.amazon.com/cli/latest/reference/ecr/batch-delete-image.html) operation. |
| ECR repository tag | [Remove tag](https://docs.aws.amazon.com/cli/latest/reference/ecr/untag-resource.html). |
| S3 bucket | [Enable or suspend versioning](https://docs.aws.amazon.com/cli/latest/reference/s3api/put-bucket-versioning.html); [delete empty bucket](https://docs.aws.amazon.com/cli/latest/reference/s3api/delete-bucket.html); upload one file with [PutObject](https://docs.aws.amazon.com/cli/latest/reference/s3api/put-object.html). |
| S3 object | Download with [GetObject](https://docs.aws.amazon.com/cli/latest/reference/s3api/get-object.html); replace with [PutObject](https://docs.aws.amazon.com/cli/latest/reference/s3api/put-object.html); [delete](https://docs.aws.amazon.com/cli/latest/reference/s3api/delete-object.html); or view metadata with [HeadObject](https://docs.aws.amazon.com/cli/latest/reference/s3api/head-object.html). |

These are the IAM actions for the resource actions above:

- EC2: `ec2:StartInstances`, `ec2:StopInstances`, `ec2:RebootInstances`,
  `ec2:TerminateInstances`, `ec2:CreateSnapshot`, `ec2:DeleteVolume`,
  `ec2:DeleteSecurityGroup`, and `ec2:DeleteTags`.
- ECS: `ecs:DeleteCluster`, `ecs:UpdateService`, `ecs:DeleteService`,
  `ecs:StopTask`, and `ecs:DeregisterTaskDefinition`.
- ECR: `ecr:DeleteRepository`, `ecr:PutImageTagMutability`,
  `ecr:StartImageScan`, `ecr:DescribeImageScanFindings`,
  `ecr:BatchDeleteImage`, and `ecr:UntagResource`.
- S3: `s3:PutBucketVersioning`, `s3:DeleteBucket`, `s3:PutObject`,
  `s3:GetObject`, and `s3:DeleteObject`. HeadObject also uses `s3:GetObject`.

The action list does not include resource creation wizards, recursive deletes,
force deletes, image builds, image pushes, or Docker sessions. Bucket deletion
is not recursive. Repository and service deletion use the non-force option.

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
