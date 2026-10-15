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
    time::Instant,
};

use serde_json::Value;

use crate::{
    config::{Config, REFRESH_TIMEOUT},
    model::{Service, Snapshot},
    operations::{AccountContext, Operation},
};

pub struct AwsCli {
    config: Config,
    cancel: Arc<AtomicBool>,
}

pub(crate) const INVOKE_OUTPUT_PLACEHOLDER: &str = transfers::INVOKE_OUTPUT_PLACEHOLDER;

pub(super) struct FetchContext<'a> {
    client: &'a AwsCli,
    deadline: Instant,
    timeout_logged: AtomicBool,
}

impl FetchContext<'_> {
    pub(super) fn expired(&self) -> bool {
        Instant::now() >= self.deadline
    }

    pub(super) fn remaining(&self) -> Result<std::time::Duration, String> {
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
        let context = FetchContext {
            client: self,
            deadline: Instant::now() + REFRESH_TIMEOUT,
            timeout_logged: AtomicBool::new(false),
        };
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
        let download = if operation.parameter == Some("Download file path") {
            let destination = args
                .pop()
                .ok_or("Download operation is missing its output path")?;
            let stage = transfers::DownloadStage::new(&destination)?;
            args.push(stage.output_path()?);
            Some(stage)
        } else {
            None
        };
        let invoke = if operation.service == "lambda" && operation.name == "invoke" {
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
        let context = FetchContext {
            client: self,
            deadline: Instant::now() + REFRESH_TIMEOUT,
            timeout_logged: AtomicBool::new(false),
        };
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
        if self.cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("cancelled".into());
        }

        let context = FetchContext {
            client: self,
            deadline: Instant::now() + REFRESH_TIMEOUT,
            timeout_logged: AtomicBool::new(false),
        };
        Ok(match service {
            Service::Ec2 => ec2::fetch(&context, selected),
            Service::Ecs => ecs::fetch(&context, selected),
            Service::Ecr => ecr::fetch(&context, selected),
            Service::S3 => s3::fetch(&context, selected),
            Service::Lambda => lambda::fetch(&context, selected),
        })
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
