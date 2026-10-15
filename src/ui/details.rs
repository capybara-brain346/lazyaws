use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::Paragraph,
};

use crate::{
    app::{App, Focus},
    config::{self, COMMAND_LOG_HEIGHT, COMPACT_LOG_HEIGHT, Config},
    theme::Theme,
};

use super::{MouseTarget, chrome::pane_block, chrome::safe_text};

pub(super) fn draw_details_and_log(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    config: &Config,
    pane_regions: &mut Vec<(Rect, MouseTarget)>,
) {
    let log_height = if app.focus == Focus::Log {
        area.height
    } else if frame.area().height < config::FULL_LOG_MIN_HEIGHT {
        COMPACT_LOG_HEIGHT
    } else {
        COMMAND_LOG_HEIGHT.saturating_add(2)
    }
    .min(area.height);
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(log_height)])
        .split(area);

    draw_details(frame, parts[0], app, config, pane_regions);
    draw_logs(frame, parts[1], app, pane_regions);
}

fn draw_details(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    config: &Config,
    pane_regions: &mut Vec<(Rect, MouseTarget)>,
) {
    let theme = app.theme();
    let stale = app.error.is_some();
    let title = if app.operation_output.is_some() {
        "[0] Action output"
    } else if stale {
        "[0] Details • stale"
    } else {
        "[0] Details"
    };
    let mut lines = Vec::new();

    if let Some(output) = &app.operation_output {
        lines.extend(output.lines().map(|line| Line::from(safe_text(line))));
        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "Action completed with no output.",
                Style::default().fg(theme.muted),
            )));
        }
    } else {
        if app.focus == Focus::Status {
            append_account_details(&mut lines, app, config);
        }

        if let Some(error) = &app.error {
            lines.push(Line::from(Span::styled(
                format!("Refresh failed; showing last data: {}", safe_text(error)),
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            )));
        }

        let selected = app
            .selected_resource()
            .or_else(|| app.snapshot.panels[0].items.get(app.selected[0]));
        if let Some(resource) = selected {
            let json = serde_json::to_string_pretty(&resource.detail)
                .unwrap_or_else(|error| format!("Could not format resource: {error}"));
            lines.extend(json.lines().map(|line| json_line(line, theme)));
        } else {
            lines.push(Line::from(Span::styled(
                if app.loading {
                    "Waiting for AWS data…"
                } else {
                    "Select a resource to view its JSON."
                },
                Style::default().fg(theme.muted),
            )));
        }
    }

    let max_scroll = lines
        .len()
        .saturating_sub(usize::from(area.height.saturating_sub(2)))
        .min(usize::from(u16::MAX)) as u16;
    let scroll = app.detail_scroll.min(max_scroll);

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .scroll((scroll, 0))
            .block(pane_block(title, app.focus == Focus::Detail, theme)),
        area,
    );
    pane_regions.push((area, MouseTarget::Pane(Focus::Detail)));
}

fn append_account_details(lines: &mut Vec<Line<'static>>, app: &App, config: &Config) {
    let theme = app.theme();
    lines.push(Line::from(Span::styled(
        "AWS account context",
        Style::default()
            .fg(theme.title)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(format!(
        "Profile: {} · Region: {}",
        safe_text(
            config
                .profile
                .as_deref()
                .unwrap_or(config::DEFAULT_PROFILE_LABEL)
        ),
        safe_text(
            config
                .region
                .as_deref()
                .unwrap_or(config::DEFAULT_REGION_LABEL)
        )
    )));
    match &app.account {
        Some(Ok(account)) => {
            lines.push(Line::from(format!(
                "Account: {}",
                safe_text(&account.account)
            )));
            lines.push(Line::from(format!("ARN: {}", safe_text(&account.arn))));
            lines.push(Line::from(format!(
                "User ID: {}",
                safe_text(&account.user_id)
            )));
        }
        Some(Err(error)) => lines.push(Line::from(Span::styled(
            format!("Account lookup failed: {}", safe_text(error)),
            Style::default().fg(theme.error),
        ))),
        None => lines.push(Line::from(Span::styled(
            "Account lookup is pending.",
            Style::default().fg(theme.muted),
        ))),
    }
}

fn json_line(line: &str, theme: &Theme) -> Line<'static> {
    let (key, value) = match line.split_once(": ") {
        Some((key, value)) => (
            Some(Span::styled(
                format!("{key}: "),
                Style::default().fg(theme.json_key),
            )),
            value,
        ),
        None => (None, line),
    };
    let value_style = match value.trim_start().chars().next() {
        Some('"') => theme.json_string,
        Some('t' | 'f' | 'n') => theme.json_literal,
        Some('0'..='9' | '-') => theme.json_number,
        _ => theme.muted,
    };
    let mut spans = Vec::with_capacity(2);
    if let Some(key) = key {
        spans.push(key);
    }
    spans.push(Span::styled(
        value.to_owned(),
        Style::default().fg(value_style),
    ));
    Line::from(spans)
}

fn draw_logs(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    pane_regions: &mut Vec<(Rect, MouseTarget)>,
) {
    let theme = app.theme();
    let title = format!("[6] Command log ({})", app.logs.len());
    let visible_lines = usize::from(area.height.saturating_sub(2));
    let start = app.logs.len().saturating_sub(visible_lines);
    let lines = app
        .logs
        .iter()
        .skip(start)
        .map(|entry| Line::from(safe_text(entry)))
        .collect::<Vec<_>>();
    let lines = if lines.is_empty() {
        vec![Line::from(Span::styled(
            "No commands yet",
            Style::default().fg(theme.muted),
        ))]
    } else {
        lines
    };

    frame.render_widget(
        Paragraph::new(Text::from(lines)).block(pane_block(&title, app.focus == Focus::Log, theme)),
        area,
    );
    pane_regions.push((area, MouseTarget::Pane(Focus::Log)));
}
