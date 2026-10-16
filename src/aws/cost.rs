mod period;
mod rows;

use crate::{
    config::{COST_EXPLORER_REGION, COST_FILTER, COST_FORECAST_METRIC, COST_METRIC},
    model::{PANEL_COUNT, Service, Snapshot},
};

use super::{FetchContext, common::panel_error};

use period::Date;

pub(crate) use rows::money;

// Each Cost Explorer request costs money. Keep this count equal to COST_REQUESTS_PER_LOAD.
pub(super) fn fetch(cli: &FetchContext<'_>) -> Snapshot {
    fetch_for(cli, Date::today())
}

fn fetch_for(cli: &FetchContext<'_>, today: Date) -> Snapshot {
    let mut snapshot = Snapshot::empty(Service::Cost);
    let month_start = today.month_start();
    let tomorrow = today.add_days(1);

    let daily = invoke(
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

    let forecast = invoke(cli, &mut snapshot, forecast_command(today))
        .and_then(|response| rows::forecast(&response));
    snapshot.panels[0].items = rows::summary(&days, forecast, today);
    snapshot.panels[2].items = rows::daily(&days);

    for (panel, dimension, other) in [
        (1, "SERVICE", "Other services"),
        (3, "REGION", "Other regions"),
    ] {
        let result = invoke(
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
