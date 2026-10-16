use serde_json::json;

use crate::{aws::money, config::COST_BAR_WIDTH, model::Resource};

use super::resource;

const UNIT: &str = "USD";

pub(super) fn cost() -> [Vec<Resource>; 4] {
    let summary = [
        ("month-to-date", "Month to date", 1842.37, " (est.)"),
        ("forecast", "Month-end forecast", 3561.90, ""),
        ("last-month", "Last month", 3298.14, ""),
    ]
    .into_iter()
    .map(|(id, label, amount, suffix)| {
        row(
            id,
            label,
            &format!("{}{suffix}", money(amount, UNIT)),
            amount,
        )
    })
    .chain([row(
        "same-days-last-month",
        "Last month 1–16",
        &format!("{} · MTD +6.3%", money(1733.10, UNIT)),
        1733.10,
    )])
    .collect();

    let services = shares(&[
        ("Amazon Elastic Compute Cloud - Compute", 812.44),
        ("Amazon Relational Database Service", 401.20),
        ("Amazon Simple Storage Service", 218.75),
        ("AWS Lambda", 154.02),
        ("Amazon Elastic Container Service", 131.66),
        ("Amazon CloudWatch", 124.30),
    ]);

    let amounts = [
        48.9, 130.3, 126.7, 94.2, 101.5, 119.8, 122.4, 129.1, 125.6, 92.8, 97.3, 120.9, 118.4,
        131.2,
    ];
    let max = amounts.iter().copied().fold(0.0, f64::max);
    let daily = amounts
        .into_iter()
        .enumerate()
        .map(|(index, amount)| {
            let date = format!("2026-10-{:02}", 16 - index);
            let estimated = if index == 0 { " (est.)" } else { "" };
            let bar = "█".repeat(((amount / max) * COST_BAR_WIDTH as f64).round() as usize);
            resource(
                &date,
                &date,
                &format!("{} {bar}{estimated}", money(amount, UNIT)),
                json!({"start": date, "amount": amount, "unit": UNIT, "estimated": index == 0}),
            )
        })
        .collect();

    let regions = shares(&[
        ("us-east-1", 1288.61),
        ("eu-west-1", 471.34),
        ("global", 82.42),
    ]);
    [summary, services, daily, regions]
}

fn shares(groups: &[(&str, f64)]) -> Vec<Resource> {
    let total: f64 = groups.iter().map(|(_, amount)| amount).sum();
    groups
        .iter()
        .map(|(name, amount)| {
            let label = format!("{} · {:.0}%", money(*amount, UNIT), amount / total * 100.0);
            row(name, name, &label, *amount)
        })
        .collect()
}

// Like the AWS rows, the amount is the label and the name follows it.
fn row(id: &str, name: &str, amount_text: &str, amount: f64) -> Resource {
    resource(
        id,
        amount_text,
        name,
        json!({"name": name, "amount": amount, "unit": UNIT}),
    )
}
