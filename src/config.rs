use std::{borrow::Cow, time::Duration};

use ratatui::style::Color;

use crate::theme::Theme;

pub const DEFAULT_THEME: Theme = Theme {
    name: Cow::Borrowed("Terminal"),
    appearance: Cow::Borrowed("terminal"),
    background: Color::Reset,
    text: Color::Reset,
    border: Color::Reset,
    focus: Color::Green,
    title: Color::Reset,
    muted: Color::DarkGray,
    key: Color::Blue,
    warning: Color::Yellow,
    error: Color::Red,
    selection_text: Color::Reset,
    selection_background: Color::Blue,
    inactive_selection_background: Color::Rgb(77, 77, 77),
    text_selection_text: Color::Black,
    text_selection_background: Color::LightYellow,
    json_key: Color::Blue,
    json_string: Color::Green,
    json_literal: Color::Magenta,
    json_number: Color::Yellow,
};
pub const THEMES_JSON: &str = include_str!("../assets/themes.json");

pub const POLL_INTERVAL: Duration = Duration::from_secs(5);
pub const EVENT_WAIT: Duration = Duration::from_millis(50);
pub const COMMAND_TIMEOUT: Duration = Duration::from_secs(20);
pub const REFRESH_TIMEOUT: Duration = Duration::from_secs(30);
pub const TRANSFER_TIMEOUT: Duration = Duration::from_secs(60 * 60);
pub const PROCESS_CHECK_INTERVAL: Duration = Duration::from_millis(20);
pub const CLI_CONNECT_TIMEOUT_SECS: u64 = 5;
pub const CLI_READ_TIMEOUT_SECS: u64 = 10;
pub const CLI_MAX_ATTEMPTS: u8 = 1;
pub const OUTPUT_BUFFER_BYTES: usize = 8192;
pub const MAX_ITEMS: usize = 500;
pub const MAX_OUTPUT_BYTES: u64 = 16 * 1024 * 1024;
pub const S3_MAX_UPLOAD_BYTES: u64 = 5 * 1024 * 1024 * 1024;
pub const LAMBDA_SYNC_PAYLOAD_BYTES: usize = 6 * 1024 * 1024;
pub const LAMBDA_ASYNC_PAYLOAD_BYTES: usize = 1024 * 1024;
pub const INVOKE_RESPONSE_LIMIT_BYTES: u64 = 8 * 1024 * 1024;
pub const ERROR_EXCERPT_CHARS: usize = 1000;
pub const ECS_SERVICES_BATCH: usize = 10;
pub const ECS_TASKS_BATCH: usize = 100;
pub const ECS_CLUSTERS_BATCH: usize = 100;
pub const DOWNLOAD_STAGE_ATTEMPTS: usize = 100;
pub const LOG_CAPACITY: usize = 100;
pub const SIDE_PANEL_WIDTH_NUMERATOR: u32 = 1;
pub const SIDE_PANEL_WIDTH_DENOMINATOR: u32 = 3;
pub const COMMAND_LOG_HEIGHT: u16 = 8;
pub const COMPACT_LOG_HEIGHT: u16 = 3;
pub const FULL_LOG_MIN_HEIGHT: u16 = 40;
pub const STATUS_PANEL_HEIGHT: u16 = 3;
pub const COMPACT_SIDE_PANEL_HEIGHT: u16 = 3;
pub const NORMAL_LAYOUT_MIN_HEIGHT: u16 = 28;
pub const COMPACT_LAYOUT_MIN_HEIGHT: u16 = 21;
pub const SERVICE_BAR_HEIGHT: u16 = 3;
pub const APP_TITLE: &str = " lazyaws ";
pub const ACTIONS_KEY_LABEL: &str = " a ";
pub const ACTIONS_KEY_TEXT: &str = "actions  ";
pub const FOOTER_HEIGHT: u16 = 1;
pub const MIN_WIDTH: u16 = 60;
pub const MIN_HEIGHT: u16 = 20;
pub const HELP_WIDTH: u16 = 64;
pub const HELP_HEIGHT: u16 = 23;
pub const ACTION_WIDTH: u16 = 88;
pub const ACTION_HEIGHT: u16 = 18;
pub const THEME_WIDTH: u16 = 58;
pub const THEME_HEIGHT: u16 = 24;
pub const ACTION_CONFIRM_PROMPT: &str = "Click Y for yes, N for cancel, or Escape to cancel.";
pub const DETAIL_SCROLL_STEP: u16 = 10;
pub const DEFAULT_AWS_BIN: &str = "aws";
pub const DEFAULT_PROFILE_LABEL: &str = "AWS chain";
pub const DEFAULT_REGION_LABEL: &str = "AWS config";
pub const PROCESS_KILL_BIN: &str = "kill";
pub const COST_EXPLORER_REGION: &str = "us-east-1";
pub const COST_METRIC: &str = "UnblendedCost";
pub const COST_FORECAST_METRIC: &str = "UNBLENDED_COST";
pub const COST_FILTER: &str =
    r#"{"Not":{"Dimensions":{"Key":"RECORD_TYPE","Values":["Credit","Refund"]}}}"#;
// A forecast with no history is a normal state, so only this error is saved as a result.
pub const COST_FORECAST_NO_DATA: &str = "DataUnavailableException";
pub const COST_PANEL_RETRY: &str = "Not fully loaded. Press r to refresh it.";
pub const COST_ACCESS_DENIED: &str = "AccessDeniedException";
pub const COST_FORECAST_DENIED_LABEL: &str = "no permission (ce:GetCostForecast)";
// A denied forecast is not asked again until this time has passed.
pub const COST_PERMISSION_RECHECK: Duration = Duration::from_secs(24 * 60 * 60);
pub const COST_DEFAULT_UNIT: &str = "USD";
pub const COST_TOP_ROWS: usize = 10;
pub const COST_DAILY_DAYS: usize = 14;
pub const COST_BAR_WIDTH: usize = 10;
pub const COST_MIN_AMOUNT: f64 = 0.005;
pub const COST_STALE_AFTER: Duration = Duration::from_secs(24 * 60 * 60);
pub const COST_CACHE_DIR: &str = "lazyaws";
pub const COST_CACHE_HOME_LINUX: &str = ".cache";
pub const COST_CACHE_HOME_MACOS: &str = "Library/Caches";
pub const COST_CACHE_FILE_PREFIX: &str = "cost-";
pub const COST_CACHE_FORMAT: u64 = 2;
pub const COST_PROMPT_WIDTH: u16 = 80;
pub const COST_PROMPT_HEIGHT: u16 = 14;

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
Keys: [ or ] services, Tab panels, 0–6 panes, j/k rows, Enter details, / filter, r refresh, t themes, ? help, q quit.\n\
The Cost tab asks before each paid Cost Explorer load, saves the result on this computer, and never refreshes by itself.\n\
AWS mode uses your AWS CLI credentials for read commands and actions after confirmation.\n\
Press a to choose an action. Use the mouse to select services, panels, and rows.\n\
--demo needs no credentials.\n\
Environment: LAZYAWS_AWS_BIN overrides the AWS executable. AWS_PROFILE and AWS_REGION\n\
are passed through to the AWS CLI when no command-line override is supplied.";

pub enum Cli {
    Run(Config),
    Help,
    Version,
}

impl Config {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Cli, String> {
        Self::parse_with_env(args, |name| std::env::var(name).ok())
    }

    pub fn parse_with_env(
        args: impl IntoIterator<Item = String>,
        env: impl Fn(&str) -> Option<String>,
    ) -> Result<Cli, String> {
        let mut config = Self {
            aws_bin: env("LAZYAWS_AWS_BIN").unwrap_or_else(|| DEFAULT_AWS_BIN.into()),
            profile: env("AWS_PROFILE").or_else(|| env("AWS_DEFAULT_PROFILE")),
            region: env("AWS_REGION").or_else(|| env("AWS_DEFAULT_REGION")),
            ..Self::default()
        };
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" => return Ok(Cli::Help),
                "--version" | "-V" => return Ok(Cli::Version),
                "--demo" => config.demo = true,
                "--profile" | "--region" | "--aws-bin" => {
                    let value = args
                        .next()
                        .filter(|value| !value.is_empty() && !value.starts_with('-'))
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
        let env = |name: &str| (name == "AWS_DEFAULT_REGION").then(|| "us-east-1".to_string());
        let parse =
            |args: &[&str]| Config::parse_with_env(args.iter().map(|arg| arg.to_string()), env);

        let Cli::Run(config) = parse(&["--demo", "--profile", "dev"]).unwrap() else {
            panic!("expected run configuration");
        };
        assert!(config.demo);
        assert_eq!(config.profile.as_deref(), Some("dev"));
        assert_eq!(config.region.as_deref(), Some("us-east-1"));

        let Cli::Run(config) = parse(&["--region", "eu-west-1"]).unwrap() else {
            panic!("expected run configuration");
        };
        assert_eq!(config.region.as_deref(), Some("eu-west-1"));
        assert!(parse(&["--profile"]).is_err());
        assert!(parse(&["--region", "--demo"]).is_err());
        assert!(parse(&["--unknown"]).is_err());
    }
}
