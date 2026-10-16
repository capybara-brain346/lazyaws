use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::{
    config::{
        COST_BAR_WIDTH, COST_DAILY_DAYS, COST_DEFAULT_UNIT, COST_METRIC, COST_MIN_AMOUNT,
        COST_TOP_ROWS,
    },
    model::Resource,
};

use super::{
    super::common::{array, optional_array, resource, string},
    period::Date,
};

pub(super) struct Day {
    start: String,
    amount: f64,
    unit: String,
    estimated: bool,
}

pub(super) fn days(response: &Value) -> Result<Vec<Day>, String> {
    array(response, "ResultsByTime")?
        .iter()
        .map(|result| {
            let period = result
                .get("TimePeriod")
                .ok_or("AWS response is missing `TimePeriod`")?;
            let total = result
                .get("Total")
                .ok_or("AWS response is missing `Total`")?;
            let (amount, unit) = metric(total, COST_METRIC)?;
            Ok(Day {
                start: string(period, "Start")?.to_owned(),
                amount,
                unit,
                estimated: result.get("Estimated").and_then(Value::as_bool) == Some(true),
            })
        })
        .collect()
}

pub(super) fn forecast(response: &Value) -> Result<(f64, String), String> {
    amount(
        response
            .get("Total")
            .ok_or("AWS response is missing `Total`")?,
    )
}

pub(super) fn summary(
    days: &[Day],
    forecast: Result<(f64, String), String>,
    today: Date,
) -> Vec<Resource> {
    let unit = days
        .first()
        .map_or(COST_DEFAULT_UNIT, |day| day.unit.as_str());
    let month_start = today.month_start().to_string();
    let last_month_start = today.previous_month_start().to_string();
    let today_text = today.to_string();
    // Compare the same number of days; a short last month ends at its own last day.
    let same_days_end = today
        .previous_month_start()
        .add_days(i64::from(today.day()))
        .min(today.month_start())
        .to_string();
    let total = |from: &str, to: Option<&str>| -> f64 {
        days.iter()
            .filter(|day| day.start.as_str() >= from && to.is_none_or(|to| day.start.as_str() < to))
            .map(|day| day.amount)
            .sum()
    };

    let month_to_date = total(&month_start, None);
    let before_today = total(&month_start, Some(&today_text));
    let last_month = total(&last_month_start, Some(&month_start));
    let same_days = total(&last_month_start, Some(&same_days_end));
    let estimated = days
        .iter()
        .any(|day| day.estimated && day.start >= month_start);

    // The amount comes first in every cost row, so it stays visible in a narrow pane.
    let row = |id: &str, name: &str, amount: String, mut detail: Value| {
        detail["name"] = json!(name);
        resource(id, amount, name, detail)
    };
    let forecast_row = match forecast {
        Ok((remaining, _)) => {
            let month_end = before_today + remaining;
            row(
                "forecast",
                "Month-end forecast",
                money(month_end, unit),
                json!({"amount": month_end, "unit": unit, "costBeforeToday": before_today, "forecastFromToday": remaining}),
            )
        }
        Err(error) => row(
            "forecast",
            "Month-end forecast",
            "unavailable (see details)".into(),
            json!({"error": error}),
        ),
    };

    vec![
        row(
            "month-to-date",
            "Month to date",
            format!(
                "{}{}",
                money(month_to_date, unit),
                if estimated { " (est.)" } else { "" }
            ),
            json!({"amount": month_to_date, "unit": unit, "start": month_start, "estimated": estimated}),
        ),
        forecast_row,
        row(
            "last-month",
            "Last month",
            money(last_month, unit),
            json!({"amount": last_month, "unit": unit, "start": last_month_start, "end": month_start}),
        ),
        row(
            "same-days-last-month",
            &format!("Last month 1–{}", today.day()),
            format!(
                "{} · MTD {}",
                money(same_days, unit),
                change(month_to_date, same_days)
            ),
            json!({"amount": same_days, "unit": unit, "start": last_month_start, "end": same_days_end}),
        ),
    ]
}

pub(super) fn daily(days: &[Day]) -> Vec<Resource> {
    let recent = &days[days.len().saturating_sub(COST_DAILY_DAYS)..];
    let max = recent.iter().map(|day| day.amount).fold(0.0, f64::max);
    recent
        .iter()
        .rev()
        .map(|day| {
            resource(
                &day.start,
                &day.start,
                format!(
                    "{} {}{}",
                    money(day.amount, &day.unit),
                    bar(day.amount, max),
                    if day.estimated { " (est.)" } else { "" }
                ),
                json!({"start": day.start, "amount": day.amount, "unit": day.unit, "estimated": day.estimated}),
            )
        })
        .collect()
}

pub(super) fn grouped(response: &Value, other_label: &str) -> Result<Vec<Resource>, String> {
    let mut totals = BTreeMap::<String, f64>::new();
    let mut unit = COST_DEFAULT_UNIT.to_owned();
    for result in array(response, "ResultsByTime")? {
        for group in optional_array(result, "Groups")? {
            let key = optional_array(group, "Keys")?
                .first()
                .and_then(Value::as_str)
                .filter(|key| !key.is_empty())
                .unwrap_or("No region");
            let metrics = group
                .get("Metrics")
                .ok_or("AWS response is missing `Metrics`")?;
            let (amount, group_unit) = metric(metrics, COST_METRIC)?;
            unit = group_unit;
            *totals.entry(key.to_owned()).or_default() += amount;
        }
    }

    let mut groups = totals
        .into_iter()
        .filter(|(_, amount)| amount.abs() >= COST_MIN_AMOUNT)
        .collect::<Vec<_>>();
    groups.sort_by(|left, right| right.1.total_cmp(&left.1));
    let total: f64 = groups.iter().map(|(_, amount)| amount).sum();
    let row = |id: &str, name: &str, amount: f64| {
        resource(
            id,
            format!("{} · {}", money(amount, &unit), share(amount, total)),
            name,
            json!({"name": name, "amount": amount, "unit": unit}),
        )
    };

    let rest = groups.split_off(groups.len().min(COST_TOP_ROWS));
    let mut items = groups
        .iter()
        .map(|(name, amount)| row(name, name, *amount))
        .collect::<Vec<_>>();
    if !rest.is_empty() {
        let amount = rest.iter().map(|(_, amount)| amount).sum();
        items.push(row(
            "other",
            &format!("{other_label} ({})", rest.len()),
            amount,
        ));
    }
    Ok(items)
}

pub(crate) fn money(amount: f64, unit: &str) -> String {
    let cents = (amount * 100.0).round() as i64;
    let sign = if cents < 0 { "-" } else { "" };
    let cents = cents.unsigned_abs();
    let digits = (cents / 100).to_string();
    let mut whole = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            whole.push(',');
        }
        whole.push(digit);
    }
    let number = format!("{whole}.{:02}", cents % 100);
    if unit == "USD" {
        format!("{sign}${number}")
    } else {
        format!("{sign}{number} {unit}")
    }
}

fn metric(metrics: &Value, name: &str) -> Result<(f64, String), String> {
    amount(
        metrics
            .get(name)
            .ok_or_else(|| format!("AWS response is missing metric `{name}`"))?,
    )
}

fn amount(value: &Value) -> Result<(f64, String), String> {
    let amount = string(value, "Amount")?
        .parse::<f64>()
        .map_err(|_| "AWS response `Amount` must be a number".to_string())?;
    let unit = string(value, "Unit").unwrap_or(COST_DEFAULT_UNIT);
    Ok((amount, unit.to_owned()))
}

fn change(current: f64, previous: f64) -> String {
    if previous.abs() < COST_MIN_AMOUNT {
        return "n/a".into();
    }
    format!("{:+.1}%", (current - previous) / previous * 100.0)
}

fn share(amount: f64, total: f64) -> String {
    if total.abs() < COST_MIN_AMOUNT {
        return "0%".into();
    }
    format!("{:.0}%", amount / total * 100.0)
}

fn bar(amount: f64, max: f64) -> String {
    if max <= 0.0 || amount < COST_MIN_AMOUNT {
        return String::new();
    }
    let width = ((amount / max) * COST_BAR_WIDTH as f64).round() as usize;
    "█".repeat(width.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../../../tests/fixtures/aws_ce.json")).unwrap()
    }

    #[test]
    fn summary_compares_equal_day_ranges_and_adds_forecast_to_cost_before_today() {
        let fixture = fixture();
        let days = days(&fixture["daily"]).unwrap();
        let today = Date::new(2026, 10, 3);
        let rows = summary(&days, forecast(&fixture["forecast"]), today);
        let amounts = rows
            .iter()
            .map(|row| row.label.as_str())
            .collect::<Vec<_>>();

        // October: 10 + 20 + 5 (estimated). September: 30 days of 4, days 1-3 are 12.
        assert_eq!(amounts[0], "$35.00 (est.)");
        assert_eq!(amounts[1], "$1,030.00");
        assert_eq!(amounts[2], "$120.00");
        assert_eq!(amounts[3], "$12.00 · MTD +191.7%");

        let error = summary(&days, Err("DataUnavailableException".into()), today);
        assert_eq!(error[1].label, "unavailable (see details)");
    }

    #[test]
    fn grouped_rows_sort_by_amount_and_collect_the_rest_as_other() {
        let rows = grouped(&fixture()["services"], "Other services").unwrap();
        assert_eq!(rows.len(), COST_TOP_ROWS + 1);
        assert_eq!(rows[0].label, "$1,200.00 · 15%");
        assert_eq!(rows[0].summary, "Service 12");
        assert_eq!(rows[COST_TOP_ROWS].summary, "Other services (2)");
        assert!(rows.iter().all(|row| row.summary != "Tax"));
    }
}
