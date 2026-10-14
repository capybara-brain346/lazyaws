use super::super::AwsCli;
use super::*;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::{Mutex, atomic::AtomicU64},
};

use crate::config::Config;

// Serialize fixture writes and process creation to prevent inherited write handles.
static FAKE_CLI_LOCK: Mutex<()> = Mutex::new(());

static NEXT_FAKE: AtomicU64 = AtomicU64::new(0);

struct FakeCli {
    dir: PathBuf,
    binary: PathBuf,
}

impl FakeCli {
    fn new(script: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "lazyaws-fake-{}-{}",
            std::process::id(),
            NEXT_FAKE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        let binary = dir.join("aws");
        fs::write(&binary, format!("#!/bin/sh\n{script}\n")).unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
        Self { dir, binary }
    }

    fn client(&self) -> AwsCli {
        let config = Config {
            aws_bin: self.binary.to_string_lossy().into_owned(),
            profile: Some("fake-profile".into()),
            region: Some("eu-west-1".into()),
            ..Config::default()
        };
        AwsCli::new(config, Arc::new(AtomicBool::new(false)))
    }

    fn enable_transcript(&self) -> PathBuf {
        let transcript = self.dir.join("commands");
        let script = fs::read_to_string(&self.binary)
            .unwrap()
            .replace("LOG_PATH", &transcript.to_string_lossy());
        fs::write(&self.binary, script).unwrap();
        transcript
    }
}

impl Drop for FakeCli {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn fake_cli_timeout_and_output_limit_are_enforced() {
    let _guard = FAKE_CLI_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let cancel = Arc::new(AtomicBool::new(false));
    let fake = FakeCli::new("sleep 3 & wait");
    let timed_out = run_process(
        fake.binary.to_str().unwrap(),
        &[],
        &cancel,
        Duration::from_millis(50),
        1024,
    );
    let error = timed_out.unwrap_err();
    assert!(error.contains("timed out"), "{error}");

    let fake = FakeCli::new("exec /usr/bin/yes");
    let start = Instant::now();
    let too_large = run_process(
        fake.binary.to_str().unwrap(),
        &[],
        &cancel,
        Duration::from_secs(2),
        4,
    );
    assert!(too_large.unwrap_err().contains("exceeded 4 bytes"));
    assert!(start.elapsed() < Duration::from_secs(1));
}

#[test]
fn fake_cli_cancellation_stops_the_process() {
    let _guard = FAKE_CLI_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let cancel = Arc::new(AtomicBool::new(false));
    let fake = FakeCli::new("sleep 3 & wait");
    let trigger = cancel.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(60));
        trigger.store(true, Ordering::Relaxed);
    });
    let start = Instant::now();
    let result = run_process(
        fake.binary.to_str().unwrap(),
        &[],
        &cancel,
        Duration::from_secs(2),
        1024,
    );
    assert!(result.unwrap_err().contains("cancelled"));
    assert!(start.elapsed() < Duration::from_secs(1));
}

#[test]
fn fake_wrapper_exit_does_not_leave_a_pipe_holding_child() {
    let _guard = FAKE_CLI_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let fake = FakeCli::new("sleep 3 & exit 0");
    let start = Instant::now();
    let result = run_process(
        fake.binary.to_str().unwrap(),
        &[],
        &Arc::new(AtomicBool::new(false)),
        Duration::from_secs(2),
        1024,
    );

    assert_eq!(result.unwrap(), Vec::<u8>::new());
    assert!(start.elapsed() < Duration::from_secs(1));
}

#[test]
fn fake_cli_errors_and_invalid_json_are_logged() {
    let _guard = FAKE_CLI_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let fake = FakeCli::new("echo denied >&2; exit 9");
    let snapshot = fake
        .client()
        .fetch(crate::model::Service::Ec2, None)
        .unwrap();
    assert!(
        snapshot.panels[0]
            .error
            .as_deref()
            .unwrap()
            .contains("denied"),
        "{:?}",
        snapshot.panels[0].error
    );
    assert!(snapshot.logs[0].contains("--profile\" \"fake-profile"));
    assert!(snapshot.logs[0].contains("failed"));

    let fake = FakeCli::new("printf '{bad json'");
    let snapshot = fake
        .client()
        .fetch(crate::model::Service::Ec2, None)
        .unwrap();
    assert!(
        snapshot.panels[0]
            .error
            .as_deref()
            .unwrap()
            .contains("invalid JSON"),
        "{:?}",
        snapshot.panels[0].error
    );
    assert!(snapshot.logs[0].contains("failed"));
}

#[test]
fn fake_cli_scopes_s3_calls_to_the_selected_bucket_region() {
    let _guard = FAKE_CLI_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let fake = FakeCli::new(
            r#"
printf '%s\n' "$*" >> LOG_PATH
case "$*" in
  *"s3api list-buckets"*) printf '%s\n' '{"Buckets":[{"Name":"archive","BucketRegion":"ap-southeast-2"},{"Name":"other","BucketRegion":"us-east-1"}]}' ;;
  *"s3api list-objects-v2"*"--delimiter /"*) printf '%s\n' '{"CommonPrefixes":[{"Prefix":"logs/"}]}' ;;
  *"s3api list-objects-v2"*) printf '%s\n' '{"Contents":[{"Key":"logs/a.txt","Size":12}]}' ;;
  *"s3api get-bucket-versioning"*) printf '%s\n' '{}' ;;
  *) echo 'unexpected command' >&2; exit 2 ;;
esac
"#
            .trim(),
        );
    let transcript = fake.dir.join("commands");
    let script = fs::read_to_string(&fake.binary)
        .unwrap()
        .replace("LOG_PATH", &transcript.to_string_lossy());
    fs::write(&fake.binary, script).unwrap();
    let cli = fake.client();
    let snapshot = cli
        .fetch(crate::model::Service::S3, Some("archive"))
        .unwrap();

    assert_eq!(snapshot.panels[1].items[0].id, "logs/a.txt");
    assert_eq!(snapshot.panels[2].items[0].id, "logs/");
    let commands = fs::read_to_string(transcript)
        .unwrap_or_else(|error| panic!("{error}: {:?}", snapshot.logs));
    let scoped_calls = commands
        .lines()
        .filter(|line| {
            line.contains("s3api list-objects-v2") || line.contains("s3api get-bucket-versioning")
        })
        .collect::<Vec<_>>();
    assert_eq!(scoped_calls.len(), 3);
    assert!(
        scoped_calls
            .iter()
            .all(|line| line.contains("--region ap-southeast-2"))
    );
    assert!(
        scoped_calls
            .iter()
            .all(|line| line.contains("--bucket archive"))
    );
    assert!(
        !scoped_calls
            .iter()
            .any(|line| line.contains("--region eu-west-1"))
    );
    assert_eq!(snapshot.logs.len(), 4);
}

#[test]
fn fake_cli_keeps_successful_panels_when_one_s3_call_fails() {
    let _guard = FAKE_CLI_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let fake = FakeCli::new(
            r#"
case "$*" in
  *"s3api list-buckets"*) printf '%s\n' '{"Buckets":[{"Name":"archive","BucketRegion":"ap-southeast-2"}]}' ;;
  *"s3api list-objects-v2"*"--delimiter /"*) printf '%s\n' '{"CommonPrefixes":[{"Prefix":"logs/"}]}' ;;
  *"s3api list-objects-v2"*) echo 'object access denied' >&2; exit 1 ;;
  *"s3api get-bucket-versioning"*) printf '%s\n' '{}' ;;
  *) echo 'unexpected command' >&2; exit 2 ;;
esac
"#
            .trim(),
        );
    let snapshot = fake
        .client()
        .fetch(crate::model::Service::S3, Some("archive"))
        .unwrap();

    assert!(
        snapshot.panels[1]
            .error
            .as_deref()
            .unwrap()
            .contains("object access denied")
    );
    assert_eq!(snapshot.panels[2].items[0].id, "logs/");
    assert_eq!(snapshot.panels[3].items.len(), 2);
    assert!(snapshot.logs.iter().any(|line| line.contains("failed")));
}

#[test]
fn fake_cli_scopes_ec2_calls_to_the_selected_instance() {
    let _guard = FAKE_CLI_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let fake = FakeCli::new(
            r#"
printf '%s\n' "$*" >> LOG_PATH
case "$*" in
  *"ec2 describe-instances"*) printf '%s\n' '{"Reservations":[{"Instances":[{"InstanceId":"i-1","InstanceType":"t3.micro","State":{"Name":"running"},"SecurityGroups":[{"GroupId":"sg-1"}]},{"InstanceId":"i-2","InstanceType":"t3.small","State":{"Name":"stopped"},"SecurityGroups":[{"GroupId":"sg-2"}],"Tags":[{"Key":"Name","Value":"selected"}]}]}]}' ;;
  *"ec2 describe-volumes"*) printf '%s\n' '{"Volumes":[]}' ;;
  *"ec2 describe-security-groups"*) printf '%s\n' '{"SecurityGroups":[{"GroupId":"sg-2","GroupName":"selected-group"}]}' ;;
  *) echo 'unexpected command' >&2; exit 2 ;;
esac
"#
            .trim(),
        );
    let transcript = fake.dir.join("commands");
    let script = fs::read_to_string(&fake.binary)
        .unwrap()
        .replace("LOG_PATH", &transcript.to_string_lossy());
    fs::write(&fake.binary, script).unwrap();
    let snapshot = fake
        .client()
        .fetch(crate::model::Service::Ec2, Some("i-2"))
        .unwrap();

    let commands = fs::read_to_string(transcript)
        .unwrap_or_else(|error| panic!("{error}: {:?}", snapshot.logs));
    let volume_call = commands
        .lines()
        .find(|line| line.contains("describe-volumes"))
        .unwrap();
    assert!(volume_call.contains("--filters Name=attachment.instance-id,Values=i-2"));
    assert!(!volume_call.contains("Values i-2"));
    let group_call = commands
        .lines()
        .find(|line| line.contains("describe-security-groups"))
        .unwrap();
    assert!(group_call.contains("--group-ids sg-2"));
    assert!(!group_call.contains("sg-1"));
    assert_eq!(snapshot.panels[2].items[0].id, "sg-2");
    assert_eq!(snapshot.panels[3].items[0].id, "Name");
}

#[test]
fn fake_cli_fetches_batched_ecs_and_selected_ecr_resources() {
    let _guard = FAKE_CLI_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let ecs = FakeCli::new(
        r#"
printf '%s\n' "$*" >> LOG_PATH
case "$*" in
  *"ecs list-clusters"*) cat <<'JSON'
{"clusterArns":["arn:aws:ecs:eu-west-1:111111111111:cluster/other","arn:aws:ecs:eu-west-1:111111111111:cluster/selected"]}
JSON
    ;;
  *"ecs describe-clusters"*) cat <<'JSON'
{"clusters":[{"clusterArn":"arn:aws:ecs:eu-west-1:111111111111:cluster/other","clusterName":"other","status":"ACTIVE"},{"clusterArn":"arn:aws:ecs:eu-west-1:111111111111:cluster/selected","clusterName":"selected","status":"ACTIVE"}]}
JSON
    ;;
  *"ecs list-services"*) cat <<'JSON'
{"serviceArns":["arn:aws:ecs:eu-west-1:111111111111:service/selected/service-00","arn:aws:ecs:eu-west-1:111111111111:service/selected/service-01","arn:aws:ecs:eu-west-1:111111111111:service/selected/service-02","arn:aws:ecs:eu-west-1:111111111111:service/selected/service-03","arn:aws:ecs:eu-west-1:111111111111:service/selected/service-04","arn:aws:ecs:eu-west-1:111111111111:service/selected/service-05","arn:aws:ecs:eu-west-1:111111111111:service/selected/service-06","arn:aws:ecs:eu-west-1:111111111111:service/selected/service-07","arn:aws:ecs:eu-west-1:111111111111:service/selected/service-08","arn:aws:ecs:eu-west-1:111111111111:service/selected/service-09","arn:aws:ecs:eu-west-1:111111111111:service/selected/service-10"]}
JSON
    ;;
  *"ecs describe-services"*"service-10"*) cat <<'JSON'
{"services":[{"serviceArn":"arn:aws:ecs:eu-west-1:111111111111:service/selected/service-10","serviceName":"service-10","status":"ACTIVE","taskDefinition":"arn:aws:ecs:eu-west-1:111111111111:task-definition/app:2"}]}
JSON
    ;;
  *"ecs describe-services"*) cat <<'JSON'
{"services":[{"serviceArn":"arn:aws:ecs:eu-west-1:111111111111:service/selected/service-00","serviceName":"service-00","status":"ACTIVE","taskDefinition":"arn:aws:ecs:eu-west-1:111111111111:task-definition/app:1"},{"serviceArn":"arn:aws:ecs:eu-west-1:111111111111:service/selected/service-01","serviceName":"service-01","status":"ACTIVE","taskDefinition":"arn:aws:ecs:eu-west-1:111111111111:task-definition/app:1"},{"serviceArn":"arn:aws:ecs:eu-west-1:111111111111:service/selected/service-02","serviceName":"service-02","status":"ACTIVE","taskDefinition":"arn:aws:ecs:eu-west-1:111111111111:task-definition/app:1"},{"serviceArn":"arn:aws:ecs:eu-west-1:111111111111:service/selected/service-03","serviceName":"service-03","status":"ACTIVE","taskDefinition":"arn:aws:ecs:eu-west-1:111111111111:task-definition/app:1"},{"serviceArn":"arn:aws:ecs:eu-west-1:111111111111:service/selected/service-04","serviceName":"service-04","status":"ACTIVE","taskDefinition":"arn:aws:ecs:eu-west-1:111111111111:task-definition/app:1"},{"serviceArn":"arn:aws:ecs:eu-west-1:111111111111:service/selected/service-05","serviceName":"service-05","status":"ACTIVE","taskDefinition":"arn:aws:ecs:eu-west-1:111111111111:task-definition/app:1"},{"serviceArn":"arn:aws:ecs:eu-west-1:111111111111:service/selected/service-06","serviceName":"service-06","status":"ACTIVE","taskDefinition":"arn:aws:ecs:eu-west-1:111111111111:task-definition/app:1"},{"serviceArn":"arn:aws:ecs:eu-west-1:111111111111:service/selected/service-07","serviceName":"service-07","status":"ACTIVE","taskDefinition":"arn:aws:ecs:eu-west-1:111111111111:task-definition/app:1"},{"serviceArn":"arn:aws:ecs:eu-west-1:111111111111:service/selected/service-08","serviceName":"service-08","status":"ACTIVE","taskDefinition":"arn:aws:ecs:eu-west-1:111111111111:task-definition/app:1"},{"serviceArn":"arn:aws:ecs:eu-west-1:111111111111:service/selected/service-09","serviceName":"service-09","status":"ACTIVE","taskDefinition":"arn:aws:ecs:eu-west-1:111111111111:task-definition/app:1"}]}
JSON
    ;;
  *"ecs list-tasks"*"--desired-status RUNNING"*) cat <<'JSON'
{"taskArns":["arn:aws:ecs:eu-west-1:111111111111:task/selected/running-task"]}
JSON
    ;;
  *"ecs list-tasks"*"--desired-status STOPPED"*) cat <<'JSON'
{"taskArns":["arn:aws:ecs:eu-west-1:111111111111:task/selected/stopped-task"]}
JSON
    ;;
  *"ecs describe-tasks"*) cat <<'JSON'
{"tasks":[{"taskArn":"arn:aws:ecs:eu-west-1:111111111111:task/selected/running-task","lastStatus":"RUNNING","taskDefinitionArn":"arn:aws:ecs:eu-west-1:111111111111:task-definition/app:1"},{"taskArn":"arn:aws:ecs:eu-west-1:111111111111:task/selected/stopped-task","lastStatus":"STOPPED","taskDefinitionArn":"arn:aws:ecs:eu-west-1:111111111111:task-definition/app:2"}]}
JSON
    ;;
  *"ecs describe-task-definition"*"app:2"*) cat <<'JSON'
{"taskDefinition":{"taskDefinitionArn":"arn:aws:ecs:eu-west-1:111111111111:task-definition/app:2","family":"app","revision":2}}
JSON
    ;;
  *"ecs describe-task-definition"*) cat <<'JSON'
{"taskDefinition":{"taskDefinitionArn":"arn:aws:ecs:eu-west-1:111111111111:task-definition/app:1","family":"app","revision":1}}
JSON
    ;;
  *) echo 'unexpected ECS command' >&2; exit 2 ;;
esac
"#
        .trim(),
    );
    let ecs_transcript = ecs.enable_transcript();
    let ecs_snapshot = ecs
        .client()
        .fetch(
            crate::model::Service::Ecs,
            Some("arn:aws:ecs:eu-west-1:111111111111:cluster/selected"),
        )
        .unwrap();

    assert_eq!(ecs_snapshot.panels[0].items.len(), 2);
    assert_eq!(ecs_snapshot.panels[1].items.len(), 11);
    assert_eq!(ecs_snapshot.panels[2].items.len(), 2);
    assert_eq!(ecs_snapshot.panels[3].items.len(), 2);
    assert!(
        ecs_snapshot
            .panels
            .iter()
            .all(|panel| panel.error.is_none())
    );
    let ecs_commands = fs::read_to_string(ecs_transcript).unwrap();
    let ecs_lines = ecs_commands.lines().collect::<Vec<_>>();
    let service_batches = ecs_lines
        .iter()
        .filter(|line| line.contains("ecs describe-services"))
        .collect::<Vec<_>>();
    assert_eq!(service_batches.len(), 2);
    assert!(service_batches[0].contains("service-00"));
    assert!(service_batches[0].contains("service-09"));
    assert!(service_batches[1].contains("service-10"));
    assert!(ecs_lines.iter().any(|line| {
        line.contains("ecs list-services")
            && line.contains("--cluster arn:aws:ecs:eu-west-1:111111111111:cluster/selected")
    }));
    for status in ["RUNNING", "STOPPED"] {
        assert!(ecs_lines.iter().any(|line| {
            line.contains("ecs list-tasks")
                && line.contains(&format!("--desired-status {status}"))
                && line.contains("--cluster arn:aws:ecs:eu-west-1:111111111111:cluster/selected")
        }));
    }
    assert!(ecs_lines.iter().any(|line| {
        line.contains("--task-definition arn:aws:ecs:eu-west-1:111111111111:task-definition/app:1")
    }));
    assert!(ecs_lines.iter().any(|line| {
        line.contains("--task-definition arn:aws:ecs:eu-west-1:111111111111:task-definition/app:2")
    }));
    assert_eq!(
        ecs_lines
            .iter()
            .filter(|line| line.contains("ecs describe-task-definition"))
            .count(),
        2
    );

    let ecr = FakeCli::new(
        r#"
printf '%s\n' "$*" >> LOG_PATH
case "$*" in
  *"ecr describe-repositories"*) cat <<'JSON'
{"repositories":[{"repositoryName":"team/other","repositoryArn":"arn:aws:ecr:eu-west-1:111111111111:repository/team/other","repositoryUri":"111111111111.dkr.ecr.eu-west-1.amazonaws.com/team/other"},{"repositoryName":"team/app","repositoryArn":"arn:aws:ecr:eu-west-1:111111111111:repository/team/app","repositoryUri":"111111111111.dkr.ecr.eu-west-1.amazonaws.com/team/app"}]}
JSON
    ;;
  *"ecr describe-images"*"--repository-name team/app"*) cat <<'JSON'
{"imageDetails":[{"imageDigest":"sha256:abc123","imageTags":["latest","v1"],"imagePushedAt":1767323045}]}
JSON
    ;;
  *"ecr list-tags-for-resource"*"repository/team/app"*) cat <<'JSON'
{"tags":[{"Key":"owner","Value":"platform"},{"Key":"stage","Value":"prod"}]}
JSON
    ;;
  *) echo 'unexpected ECR command' >&2; exit 2 ;;
esac
"#
        .trim(),
    );
    let ecr_transcript = ecr.enable_transcript();
    let ecr_snapshot = ecr
        .client()
        .fetch(crate::model::Service::Ecr, Some("team/app"))
        .unwrap();

    assert_eq!(ecr_snapshot.panels[0].items.len(), 2);
    assert_eq!(ecr_snapshot.panels[1].items.len(), 1);
    assert_eq!(ecr_snapshot.panels[1].items[0].id, "sha256:abc123");
    assert_eq!(ecr_snapshot.panels[2].items.len(), 2);
    assert_eq!(ecr_snapshot.panels[3].items.len(), 2);
    assert!(
        ecr_snapshot
            .panels
            .iter()
            .all(|panel| panel.error.is_none())
    );
    let ecr_commands = fs::read_to_string(ecr_transcript).unwrap();
    assert!(ecr_commands.contains("--repository-name team/app"));
    assert!(
        ecr_commands
            .contains("--resource-arn arn:aws:ecr:eu-west-1:111111111111:repository/team/app")
    );
    assert!(!ecr_commands.contains("--repository-name team/other"));
    assert!(!ecr_commands.contains("repository/team/other"));
}

#[test]
fn expired_refresh_deadline_skips_commands() {
    let _guard = FAKE_CLI_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let fake = FakeCli::new("echo '{}' > LOG_PATH");
    let transcript = fake.dir.join("commands");
    let script = fs::read_to_string(&fake.binary)
        .unwrap()
        .replace("LOG_PATH", &transcript.to_string_lossy());
    fs::write(&fake.binary, script).unwrap();
    let client = fake.client();
    let context = FetchContext {
        client: &client,
        deadline: Instant::now(),
        timeout_logged: AtomicBool::new(false),
    };
    let mut logs = Vec::new();
    let error = context
        .invoke(
            &mut logs,
            &["ec2".into(), "describe-instances".into()],
            true,
        )
        .unwrap_err();

    assert!(error.contains("timed out"));
    assert_eq!(logs.len(), 1);
    assert!(logs[0].contains("remaining commands were skipped"));
    assert!(!transcript.exists());
}
