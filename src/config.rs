use std::time::Duration;

pub const POLL_INTERVAL: Duration = Duration::from_secs(5);
pub const EVENT_WAIT: Duration = Duration::from_millis(50);
pub const COMMAND_TIMEOUT: Duration = Duration::from_secs(20);
pub const PROCESS_CHECK_INTERVAL: Duration = Duration::from_millis(20);
pub const CLI_CONNECT_TIMEOUT_SECS: u64 = 5;
pub const CLI_READ_TIMEOUT_SECS: u64 = 10;
pub const MAX_ITEMS: usize = 500;
pub const MAX_OUTPUT_BYTES: u64 = 16 * 1024 * 1024;
pub const ECS_SERVICES_BATCH: usize = 10;
pub const ECS_TASKS_BATCH: usize = 100;
pub const ECS_CLUSTERS_BATCH: usize = 100;
pub const LOG_CAPACITY: usize = 100;
pub const SIDE_PANEL_PERCENT: u16 = 33;
pub const COMMAND_LOG_HEIGHT: u16 = 8;
pub const STATUS_PANEL_HEIGHT: u16 = 3;
pub const SERVICE_BAR_HEIGHT: u16 = 2;
pub const FOOTER_HEIGHT: u16 = 1;
pub const MIN_WIDTH: u16 = 60;
pub const MIN_HEIGHT: u16 = 20;
pub const HELP_WIDTH: u16 = 64;
pub const HELP_HEIGHT: u16 = 23;
pub const DEFAULT_AWS_BIN: &str = "aws";

#[derive(Clone, Debug)]
pub struct Config {
    pub aws_bin: String,
    pub profile: Option<String>,
    pub region: Option<String>,
    pub demo: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            aws_bin: DEFAULT_AWS_BIN.into(),
            profile: None,
            region: None,
            demo: false,
        }
    }
}

pub const USAGE: &str = "lazyaws [--demo] [--profile NAME] [--region REGION] [--aws-bin PATH]\n\n\
Keys: [/] services, Tab panels, j/k rows, Enter details, / filter, r refresh, ? help, q quit.\n\
AWS mode uses your AWS CLI credentials and read commands. --demo needs no credentials.\n\
Environment: LAZYAWS_AWS_BIN overrides the AWS executable. AWS_PROFILE and AWS_REGION\n\
are passed through to the AWS CLI when no command-line override is supplied.";

pub enum Cli {
    Run(Config),
    Help,
    Version,
}

impl Config {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Cli, String> {
        let mut config = Self {
            aws_bin: std::env::var("LAZYAWS_AWS_BIN")
                .unwrap_or_else(|_| DEFAULT_AWS_BIN.into()),
            ..Self::default()
        };
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" => return Ok(Cli::Help),
                "--version" | "-V" => return Ok(Cli::Version),
                "--demo" => config.demo = true,
                "--profile" | "--region" | "--aws-bin" => {
                    let value = args.next().filter(|value| !value.is_empty() && !value.starts_with('-'))
                        .ok_or_else(|| format!("{arg} needs a value"))?;

                    match arg.as_str() {
                        "--profile" => config.profile = Some(value),
                        "--region" => config.region = Some(value),
                        _ => config.aws_bin = value,
                    }
                }
                _ => return Err(format!("Unknown option: {arg}")),
            }
        }

        if config.aws_bin.is_empty() {
            return Err("The AWS executable path is empty".into());
        }

        Ok(Cli::Run(config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_context_and_rejects_missing_or_unknown_options() {
        let args = ["--demo", "--profile", "dev", "--region", "eu-west-1"];
        let Cli::Run(config) = Config::parse(args.map(String::from)).unwrap() else {
            panic!("expected run configuration");
        };
        assert!(config.demo);
        assert_eq!(config.profile.as_deref(), Some("dev"));
        assert_eq!(config.region.as_deref(), Some("eu-west-1"));
        assert!(Config::parse(["--profile".into()]).is_err());
        assert!(Config::parse(["--region".into(), "--demo".into()]).is_err());
        assert!(Config::parse(["--unknown".into()]).is_err());
    }
}
