mod common;
mod ec2;
mod ecr;
mod ecs;
mod lambda;
mod process;
mod s3;
mod transfers;

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use serde_json::Value;

use crate::{
    config::{COMMAND_TIMEOUT, Config, REFRESH_TIMEOUT, TRANSFER_TIMEOUT},
    model::{Service, Snapshot},
    operations::{AccountContext, Operation, Parameter},
};

pub struct AwsCli {
    config: Config,
    cancel: Arc<AtomicBool>,
}

pub(crate) const INVOKE_OUTPUT_PLACEHOLDER: &str = transfers::INVOKE_OUTPUT_PLACEHOLDER;

pub use process::outcome_unknown;

pub(super) struct FetchContext<'a> {
    client: &'a AwsCli,
    deadline: Instant,
    command_timeout: Duration,
    timeout_logged: AtomicBool,
}

impl<'a> FetchContext<'a> {
    fn new(client: &'a AwsCli, budget: Duration, command_timeout: Duration) -> Self {
        Self {
            client,
            deadline: Instant::now() + budget,
            command_timeout,
            timeout_logged: AtomicBool::new(false),
        }
    }

    pub(super) fn expired(&self) -> bool {
        Instant::now() >= self.deadline
    }

    pub(super) fn remaining(&self) -> Result<Duration, String> {
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| "AWS refresh timed out".into())
    }

    pub(super) fn log_timeout_once(&self, logs: &mut Vec<String>) {
        if !self.timeout_logged.swap(true, Ordering::Relaxed) {
            logs.push(
                "notice: AWS refresh time limit reached; remaining commands were skipped".into(),
            );
        }
    }
}

impl AwsCli {
    pub fn new(config: Config, cancel: Arc<AtomicBool>) -> Self {
        Self { config, cancel }
    }

    pub fn identity(&self, logs: &mut Vec<String>) -> Result<AccountContext, String> {
        let context = FetchContext::new(self, REFRESH_TIMEOUT, COMMAND_TIMEOUT);
        let response =
            context.invoke(logs, &["sts".into(), "get-caller-identity".into()], false)?;
        let result = (|| {
            Ok(AccountContext {
                account: identity_field(&response, "Account")?,
                arn: identity_field(&response, "Arn")?,
                user_id: identity_field(&response, "UserId")?,
            })
        })();
        if let Err(error) = &result {
            logs.push(format!("STS identity response [failed: {error}]"));
        }
        result
    }

    pub fn execute(
        &self,
        operation: &Operation,
        value: &str,
        logs: &mut Vec<String>,
    ) -> Result<Value, String> {
        let mut args = operation.args(value)?;
        let download = if operation.parameter == Some(Parameter::DownloadFilePath) {
            let destination = args
                .pop()
                .ok_or("Download operation is missing its output path")?;
            let stage = transfers::DownloadStage::new(&destination)?;
            args.push(stage.output_path()?);
            Some(stage)
        } else {
            None
        };
        let invoke = if matches!(operation.parameter, Some(Parameter::InvokePayload(_))) {
            let placeholder = args
                .pop()
                .ok_or("Invoke operation is missing its output placeholder")?;
            if placeholder != transfers::INVOKE_OUTPUT_PLACEHOLDER {
                return Err("Invoke operation is missing its output placeholder".into());
            }
            let stage = transfers::InvokeOutput::new()?;
            args.push(stage.output_path()?);
            Some(stage)
        } else {
            None
        };
        let timeout = execute_timeout(operation);
        let context = FetchContext::new(self, timeout, timeout);
        let response = context.invoke_region(logs, &args, false, operation.region.as_deref())?;
        if let Some(download) = download {
            download.commit()?;
        }
        if let Some(invoke) = invoke {
            return invoke.into_response(response);
        }
        Ok(response)
    }

    pub fn fetch(&self, service: Service, selected: Option<&str>) -> Result<Snapshot, String> {
        if self.cancel.load(Ordering::Relaxed) {
            return Err("cancelled".into());
        }

        let context = FetchContext::new(self, REFRESH_TIMEOUT, COMMAND_TIMEOUT);
        Ok(match service {
            Service::Ec2 => ec2::fetch(&context, selected),
            Service::Ecs => ecs::fetch(&context, selected),
            Service::Ecr => ecr::fetch(&context, selected),
            Service::S3 => s3::fetch(&context, selected),
            Service::Lambda => lambda::fetch(&context, selected),
        })
    }
}

fn execute_timeout(operation: &Operation) -> Duration {
    match operation.parameter {
        Some(Parameter::UploadFilePath | Parameter::DownloadFilePath) => TRANSFER_TIMEOUT,
        _ => COMMAND_TIMEOUT,
    }
}

fn identity_field(response: &Value, field: &str) -> Result<String, String> {
    response
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("AWS identity response is missing string `{field}`"))
}
