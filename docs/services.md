# Service guide

The AWS CLI supplies credentials. Lazy AWS passes profile and region options
to each command. It disables the pager and interactive prompts. Commands return
JSON. Errors appear in the pane and the command log.

| Service | Panes after Status | Read operations |
| --- | --- | --- |
| EC2 | Instances, Volumes, Security groups, Tags | DescribeInstances, DescribeVolumes, DescribeSecurityGroups |
| ECS | Clusters, Services, Tasks, Task definitions | ListClusters, DescribeClusters, ListServices, DescribeServices, ListTasks, DescribeTasks, DescribeTaskDefinition |
| ECR | Repositories, Images, Image tags, Repository tags | DescribeRepositories, DescribeImages, ListTagsForResource |
| S3 | Buckets, Objects, Prefixes, Bucket settings | ListBuckets, ListObjectsV2, GetBucketLocation, GetBucketVersioning |

EC2 tags come from the selected instance response. ECR image tags come from the
image response. S3 prefixes come from object keys. ECS task definitions come
from the selected cluster's services and tasks.

Use IAM permissions for these operations. S3 requires `s3:ListAllMyBuckets`,
`s3:ListBucket`, `s3:GetBucketLocation`, and `s3:GetBucketVersioning`.
EC2 and ECR operation names match their IAM action names. ECS uses
`ecs:DescribeTaskDefinition` for task definition reads.

```text
Primary selection         Related panes
Instance ID ------------> attached volumes, security groups, tags
Cluster ARN ------------> services, tasks, task definitions
Repository name --------> images, image tags, repository tags
Bucket name ------------> objects, prefixes, settings
```

Only the active service sends commands. Changing the primary selection causes
a refresh. Missing read permissions can cause a partial result. A pane with an
error shows its last successful data if the selected parent is unchanged.

Large lists have a limit. Read the command log for truncation notices.
This release does not start or stop instances, deploy services, push images,
or upload or delete objects.
