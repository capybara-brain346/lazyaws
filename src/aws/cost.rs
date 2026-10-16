use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

mod period;
mod rows;

use crate::{
    config::{
        COST_ACCESS_DENIED, COST_EXPLORER_REGION, COST_FILTER, COST_FORECAST_DENIED_LABEL,
        COST_FORECAST_METRIC, COST_FORECAST_NO_DATA, COST_METRIC, COST_PERMISSION_RECHECK,
    },
    model::{PANEL_COUNT, Service, Snapshot},
};

use super::{FetchContext, common::panel_error};

use period::Date;

pub(crate) use rows::money;

// Each Cost Explorer request is charged, and a long result needs more than one page.
pub(super) fn fetch(cli: &FetchContext<'_>, forecast_denied_at: Option<u64>) -> Snapshot {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    fetch_for(cli, Date::today(), forecast_denied_at, now)
}

// A recent denial means the forecast permission is still missing, so no request is sent.
// A denial time in the future comes from a wrong clock, so it does not skip the request.
fn forecast_skipped(denied_at: Option<u64>, now: u64) -> bool {
    denied_at.is_some_and(|at| at <= now && now - at < COST_PERMISSION_RECHECK.as_secs())
}

fn fetch_for(
    cli: &FetchContext<'_>,
    today: Date,
    forecast_denied_at: Option<u64>,
    now: u64,
) -> Snapshot {
    let mut snapshot = Snapshot::empty(Service::Cost);
    let month_start = today.month_start();
    let tomorrow = today.add_days(1);

    let daily = usage(
        cli,
        &mut snapshot,
        usage_command(today.previous_month_start(), tomorrow, "DAILY", None),
    )
    .and_then(|response| rows::days(&response));
    let days = match daily {
        Ok(days) => days,
        Err(error) => {
            // The other requests would fail the same way, and each one is charged.
            for panel in 0..PANEL_COUNT {
                panel_error(&mut snapshot, panel, &error);
            }
            return snapshot;
        }
    };

    let skipped = forecast_skipped(forecast_denied_at, now);
    let forecast = if skipped {
        Err(COST_ACCESS_DENIED.to_owned())
    } else {
        invoke(cli, &mut snapshot, forecast_command(today))
            .and_then(|response| rows::forecast(&response))
    };
    let denied_at = if skipped {
        forecast_denied_at
    } else {
        Some(now)
    };
    apply_forecast(&mut snapshot, &days, forecast, today, denied_at);
    snapshot.panels[2].items = rows::daily(&days);

    for (panel, dimension, other) in [
        (1, "SERVICE", "Other services"),
        (3, "REGION", "Other regions"),
    ] {
        let result = usage(
            cli,
            &mut snapshot,
            usage_command(month_start, tomorrow, "MONTHLY", Some(dimension)),
        )
        .and_then(|response| rows::grouped(&response, other));
        match result {
            Ok(items) => snapshot.panels[panel].items = items,
            Err(error) => panel_error(&mut snapshot, panel, error),
        }
    }
    snapshot
}

// No history and no permission last long, so those results are saved. A denial keeps its
// time in the forecast row. Other errors may pass, so the panel keeps its rows but is
// marked as failed.
fn apply_forecast(
    snapshot: &mut Snapshot,
    days: &[rows::Day],
    forecast: Result<(f64, String), String>,
    today: Date,
    denied_at: Option<u64>,
) {
    let denied = matches!(&forecast, Err(error) if error.contains(COST_ACCESS_DENIED));
    if let Err(error) = &forecast
        && !denied
        && !error.contains(COST_FORECAST_NO_DATA)
    {
        panel_error(snapshot, 0, error);
    }

    snapshot.panels[0].items = rows::summary(days, forecast, today);
    if let (true, Some(row)) = (
        denied,
        snapshot.panels[0]
            .items
            .iter_mut()
            .find(|item| item.id == "forecast"),
    ) {
        row.label = COST_FORECAST_DENIED_LABEL.into();
        row.detail = serde_json::json!({
            "name": "Month-end forecast",
            "error": "Missing permission ce:GetCostForecast",
            "permissionDenied": true,
            "deniedAt": denied_at,
        });
    }
}

fn permission_error(error: String, action: &str) -> String {
    if error.contains(COST_ACCESS_DENIED) {
        format!("Missing permission {action}: {error}")
    } else {
        error
    }
}

// Follows every page of a usage request. Each page is another charged request.
fn usage(
    cli: &FetchContext<'_>,
    snapshot: &mut Snapshot,
    command: Vec<String>,
) -> Result<serde_json::Value, String> {
    let denied = |error| permission_error(error, "ce:GetCostAndUsage");
    let mut response = invoke(cli, snapshot, command.clone()).map_err(denied)?;
    let mut token = next_token(&mut response);
    let mut sent = HashSet::new();

    while let Some(next) = token {
        sent.insert(next.clone());
        let mut page_command = command.clone();
        page_command.extend(["--next-page-token".into(), next]);
        let page = invoke(cli, snapshot, page_command).map_err(denied)?;
        token = merge_page(&mut response, page, &sent)?;
    }
    Ok(response)
}

// Adds a page to the result and returns the next token. A token that was sent before
// would repeat a charged request for ever, so it is an error.
fn merge_page(
    all: &mut serde_json::Value,
    mut page: serde_json::Value,
    sent: &HashSet<String>,
) -> Result<Option<String>, String> {
    let token = next_token(&mut page);
    if token.as_ref().is_some_and(|token| sent.contains(token)) {
        return Err("Cost Explorer returned the same page token again".into());
    }

    let more = page["ResultsByTime"].as_array_mut().map(std::mem::take);
    if let (Some(all), Some(mut more)) = (all["ResultsByTime"].as_array_mut(), more) {
        all.append(&mut more);
    }
    Ok(token)
}

fn next_token(response: &mut serde_json::Value) -> Option<String> {
    let token = response.get_mut("NextPageToken")?.take();
    token
        .as_str()
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
}

fn invoke(
    cli: &FetchContext<'_>,
    snapshot: &mut Snapshot,
    command: Vec<String>,
) -> Result<serde_json::Value, String> {
    cli.invoke_region(
        &mut snapshot.logs,
        &command,
        false,
        Some(COST_EXPLORER_REGION),
    )
}

fn usage_command(start: Date, end: Date, granularity: &str, group_by: Option<&str>) -> Vec<String> {
    let mut command = vec![
        "ce".into(),
        "get-cost-and-usage".into(),
        "--time-period".into(),
        format!("Start={start},End={end}"),
        "--granularity".into(),
        granularity.into(),
        "--metrics".into(),
        COST_METRIC.into(),
        "--filter".into(),
        COST_FILTER.into(),
    ];
    if let Some(dimension) = group_by {
        command.extend([
            "--group-by".into(),
            format!("Type=DIMENSION,Key={dimension}"),
        ]);
    }
    command
}

// The forecast starts today, so the month-end total adds it to the cost before today.
fn forecast_command(today: Date) -> Vec<String> {
    vec![
        "ce".into(),
        "get-cost-forecast".into(),
        "--time-period".into(),
        format!("Start={today},End={}", today.next_month_start()),
        "--granularity".into(),
        "MONTHLY".into(),
        "--metric".into(),
        COST_FORECAST_METRIC.into(),
        "--filter".into(),
        COST_FILTER.into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn pages_are_merged_and_a_repeated_token_stops_the_loop() {
        let mut all = json!({"ResultsByTime": [1], "NextPageToken": "a"});
        let first = next_token(&mut all);
        assert_eq!(first.as_deref(), Some("a"));

        let page = json!({"ResultsByTime": [2, 3]});
        let sent = HashSet::from(["a".to_string()]);
        assert_eq!(merge_page(&mut all, page, &sent), Ok(None));
        assert_eq!(all["ResultsByTime"], json!([1, 2, 3]));

        let repeat = json!({"ResultsByTime": [4], "NextPageToken": "a"});
        assert!(merge_page(&mut all, repeat, &sent).is_err());
        assert_eq!(all["ResultsByTime"], json!([1, 2, 3]));

        // A cycle A -> B -> A is caught on the second return of A.
        let sent = HashSet::from(["a".to_string(), "b".to_string()]);
        let cycle = json!({"ResultsByTime": [5], "NextPageToken": "a"});
        assert!(merge_page(&mut all, cycle, &sent).is_err());
    }

    #[test]
    fn only_a_forecast_with_no_history_counts_as_loaded() {
        let days = rows::days(&serde_json::json!({"ResultsByTime": []})).unwrap();
        let today = Date::new(2026, 10, 3);

        let mut no_history = Snapshot::empty(Service::Cost);
        apply_forecast(
            &mut no_history,
            &days,
            Err("DataUnavailableException".into()),
            today,
            None,
        );
        assert!(no_history.panels[0].error.is_none());
        assert_eq!(
            no_history.panels[0].items[1].label,
            "unavailable (see details)"
        );

        let mut throttled = Snapshot::empty(Service::Cost);
        apply_forecast(
            &mut throttled,
            &days,
            Err("ThrottlingException".into()),
            today,
            None,
        );
        assert!(throttled.panels[0].error.is_some());
        assert_eq!(throttled.panels[0].items.len(), 4);
    }

    #[test]
    fn a_denied_forecast_is_saved_with_its_time_and_not_asked_again_for_a_day() {
        let days = rows::days(&serde_json::json!({"ResultsByTime": []})).unwrap();
        let mut snapshot = Snapshot::empty(Service::Cost);
        let err = "AccessDeniedException".into();
        apply_forecast(
            &mut snapshot,
            &days,
            Err(err),
            Date::new(2026, 10, 3),
            Some(1_000),
        );

        assert!(snapshot.panels[0].error.is_none());
        assert_eq!(
            snapshot.panels[0].items[1].label,
            COST_FORECAST_DENIED_LABEL
        );
        assert_eq!(snapshot.panels[0].items[1].detail["deniedAt"], 1_000);

        let day = COST_PERMISSION_RECHECK.as_secs();
        assert!(forecast_skipped(Some(1_000), 1_000 + day - 1));
        assert!(!forecast_skipped(Some(1_000), 1_000 + day));
        assert!(!forecast_skipped(None, 1_000));
        assert!(!forecast_skipped(Some(1_000 + day), 1_000));
    }
}
