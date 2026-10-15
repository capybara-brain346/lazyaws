#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvokeMode {
    Sync,
    SyncWithLogs,
    Async,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parameter {
    JsonFields,
    TagValue,
    LambdaTagValue,
    InvokePayload(InvokeMode),
    ReservedConcurrency,
    ProvisionedConcurrency,
    AliasVersion,
    TaskDefinitionRevision,
    ServiceDeploymentArn,
    DesiredTaskCount,
    UploadFilePath,
    DownloadFilePath,
}

impl Parameter {
    pub fn label(self) -> &'static str {
        match self {
            Self::JsonFields => "JSON fields",
            Self::TagValue => "Tag value",
            Self::LambdaTagValue => "Lambda tag value",
            Self::InvokePayload(InvokeMode::Sync) => "Invoke payload",
            Self::InvokePayload(InvokeMode::SyncWithLogs) => "Invoke payload with logs",
            Self::InvokePayload(InvokeMode::Async) => "Invoke payload async",
            Self::ReservedConcurrency => "Reserved concurrency",
            Self::ProvisionedConcurrency => "Provisioned concurrency",
            Self::AliasVersion => "Alias version",
            Self::TaskDefinitionRevision => "Task definition revision",
            Self::ServiceDeploymentArn => "Service deployment ARN",
            Self::DesiredTaskCount => "Desired task count",
            Self::UploadFilePath => "Upload file path",
            Self::DownloadFilePath => "Download file path",
        }
    }
}
