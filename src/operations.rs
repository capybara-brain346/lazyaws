mod args;
mod catalog;
mod parameter;

pub use catalog::available;
pub use parameter::{InvokeMode, Parameter};

#[cfg(test)]
mod tests;

use serde_json::Value;

use crate::config::MAX_ITEMS;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountContext {
    pub account: String,
    pub arn: String,
    pub user_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operation {
    pub label: &'static str,
    pub service: &'static str,
    pub name: &'static str,
    pub input: Value,
    pub region: Option<String>,
    pub target: String,
    pub warning: &'static str,
    pub parameter: Option<Parameter>,
    pub paginated: bool,
}

impl Operation {
    pub fn args(&self, value: &str) -> Result<Vec<String>, String> {
        let mut input = self.input.clone();
        let mut extra = Vec::new();

        match self.parameter {
            Some(Parameter::JsonFields) => args::json_fields(&mut input, value)?,
            Some(Parameter::TagValue) => args::tag_value(self, &mut input, value)?,
            Some(Parameter::LambdaTagValue) => args::lambda_tag_value(self, &mut input, value)?,
            Some(Parameter::InvokePayload(mode)) => return args::invoke(self, mode, value),
            Some(Parameter::ReservedConcurrency) => {
                args::reserved_concurrency(self, &mut input, value)?
            }
            Some(Parameter::ProvisionedConcurrency) => {
                args::provisioned_concurrency(self, &mut input, value)?
            }
            Some(Parameter::AliasVersion) => args::alias_version(self, &mut input, value)?,
            Some(Parameter::TaskDefinitionRevision) => {
                args::task_definition_revision(self, &mut input, value)?
            }
            Some(Parameter::ServiceDeploymentArn) => {
                args::service_deployment_arn(self, &mut input, value)?
            }
            Some(Parameter::DesiredTaskCount) => args::desired_task_count(self, &mut input, value)?,
            Some(Parameter::UploadFilePath) => {
                extra = args::upload_file(self, &mut input, value)?;
            }
            Some(Parameter::DownloadFilePath) => return args::download_file(self, &input, value),
            None => {}
        }

        let input = serde_json::to_string(&input)
            .map_err(|error| format!("cannot serialize AWS input: {error}"))?;
        let mut args = vec![
            self.service.into(),
            self.name.into(),
            "--cli-input-json".into(),
            input,
        ];
        args.extend(extra);
        if self.paginated {
            args.extend(["--max-items".into(), MAX_ITEMS.to_string()]);
        }
        Ok(args)
    }

    #[must_use]
    fn paginated(mut self) -> Self {
        self.paginated = true;
        self
    }

    pub fn initial_input(&self) -> String {
        match self.parameter {
            Some(Parameter::DesiredTaskCount) => self
                .input
                .get("desiredCount")
                .and_then(Value::as_u64)
                .map(|count| count.to_string())
                .unwrap_or_default(),
            _ => String::new(),
        }
    }
}
