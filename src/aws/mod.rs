mod common;
mod ec2;
mod ecr;
mod ecs;
mod process;
mod s3;

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use crate::{
    config::{Config, REFRESH_TIMEOUT},
    model::{Service, Snapshot},
};

pub struct AwsCli {
    config: Config,
    cancel: Arc<AtomicBool>,
}

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
        })
    }
}
