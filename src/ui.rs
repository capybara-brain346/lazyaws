use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

use crate::{
    app::App,
    config::{
        self, COMMAND_LOG_HEIGHT, Config, FOOTER_HEIGHT, HELP_HEIGHT, HELP_WIDTH, MIN_HEIGHT,
        MIN_WIDTH, POLL_INTERVAL, SERVICE_BAR_HEIGHT, SIDE_PANEL_PERCENT, STATUS_PANEL_HEIGHT,
    },
    model::{Resource, Service},
};

pub fn draw(frame: &mut Frame, app: &App, config: &Config) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        draw_too_small(frame, area);
        return;
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(SERVICE_BAR_HEIGHT),
            Constraint::Min(0),
            Constraint::Length(FOOTER_HEIGHT),
        ])
        .split(area);

    draw_services(frame, layout[0], app);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(SIDE_PANEL_PERCENT),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(layout[1]);

    draw_side_panels(frame, columns[0], app, config);
    draw_details_and_log(frame, columns[2], app);
    draw_key_bar(frame, layout[2]);

    if app.help {
        draw_help(frame, area);
    }
}

fn draw_too_small(frame: &mut Frame, area: Rect) {
    let warning = Paragraph::new("Terminal is too small. Enlarge it to use lazyaws.")
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" lazyaws ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(config::WARNING_COLOR)),
        );
    frame.render_widget(warning, area);
}

fn draw_services(frame: &mut Frame, area: Rect, app: &App) {
    let mut spans = vec![Span::styled(
        " lazyaws ",
        Style::default()
            .fg(config::TITLE_COLOR)
            .add_modifier(Modifier::BOLD),
    )];

    for service in Service::ALL {
        let selected = service == app.service;
        spans.push(Span::styled(
            format!(" {} ", service.name()),
            if selected {
                Style::default()
                    .fg(config::SELECTION_TEXT_COLOR)
                    .bg(config::SELECTION_BACKGROUND_COLOR)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(config::TITLE_COLOR)
            },
        ));
    }

    spans.push(Span::styled(
        "  [/] switch service",
        Style::default().fg(config::MUTED_COLOR),
    ));
    let mut lines = vec![Line::from(spans)];
    if app.filtering || !app.filter.is_empty() {
        let label = if app.filtering {
            " Filter: "
        } else {
            " Filtered: "
        };
        let mut filter = vec![Span::styled(
            label,
            Style::default().fg(config::TITLE_COLOR),
        )];
        filter.push(Span::raw(safe_text(&app.filter)));
        if app.filtering {
            filter.push(Span::styled(
                "▏",
                Style::default().fg(config::WARNING_COLOR),
            ));
        }
        lines.push(Line::from(filter));
    }
    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn draw_side_panels(frame: &mut Frame, area: Rect, app: &App, config: &Config) {
    let status_height = STATUS_PANEL_HEIGHT.min(area.height);
    let panels_height = area.height.saturating_sub(status_height);
    let base_panel_height = panels_height / 4;
    let extra = panels_height % 4;
    let mut constraints = vec![Constraint::Length(status_height)];

    for index in 0..4 {
        constraints.push(Constraint::Length(
            base_panel_height + if index < usize::from(extra) { 1 } else { 0 },
        ));
    }

    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    draw_status(frame, areas[0], app, config);
    for panel_index in 0..4 {
        draw_resource_panel(frame, areas[panel_index + 1], app, panel_index + 1);
    }
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App, config: &Config) {
    let profile = config
        .profile
        .as_deref()
        .unwrap_or(config::DEFAULT_PROFILE_LABEL);
    let region = config
        .region
        .as_deref()
        .unwrap_or(config::DEFAULT_REGION_LABEL);
    let mode = if config.demo {
        Span::styled(
            " DEMO",
            Style::default()
                .fg(config::WARNING_COLOR)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(" LIVE", Style::default().fg(config::FOCUS_COLOR))
    };
    let mut spans = vec![
        Span::styled(" pf:", Style::default().fg(config::TITLE_COLOR)),
        Span::raw(safe_text(profile)),
        Span::styled(" rg:", Style::default().fg(config::TITLE_COLOR)),
        Span::raw(safe_text(region)),
        mode,
        Span::styled(
            format!(" {}s", POLL_INTERVAL.as_secs()),
            Style::default().fg(config::MUTED_COLOR),
        ),
    ];

    if app.loading {
        spans.push(Span::styled(
            " loading",
            Style::default().fg(config::WARNING_COLOR),
        ));
    } else if let Some(updated) = app.last_updated {
        spans.push(Span::styled(
            format!(" u{}s", updated.elapsed().as_secs()),
            Style::default().fg(config::MUTED_COLOR),
        ));
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).block(pane_block("[1] Status", app.focus == 0)),
        area,
    );
}

fn draw_resource_panel(frame: &mut Frame, area: Rect, app: &App, panel: usize) {
    let data = &app.snapshot.panels[panel - 1];
    let visible = app.visible_indices(panel);
    let title = format!(
        "[{}] {} ({}/{})",
        panel + 1,
        data.title,
        visible.len(),
        data.items.len()
    );
    let mut lines = Vec::new();

    if let Some(error) = &data.error {
        lines.push(Line::from(Span::styled(
            format!("Error: {}", safe_text(error)),
            Style::default().fg(config::ERROR_COLOR),
        )));
    }

    if visible.is_empty() {
        let message = if app.loading && data.error.is_none() {
            "Loading…"
        } else if !app.filter.is_empty() {
            "No matches"
        } else if data.error.is_some() {
            "Showing last data"
        } else {
            "No resources"
        };
        lines.push(Line::from(Span::styled(
            message,
            Style::default().fg(config::MUTED_COLOR),
        )));
    } else {
        for &index in &visible {
            let Some(resource) = data.items.get(index) else {
                continue;
            };
            let selected = app.selected[panel - 1] == index;
            lines.push(resource_line(resource, selected));
        }
    }

    let error_rows = if data.error.is_some() { 1 } else { 0 };
    let list_rows = usize::from(area.height.saturating_sub(2)).saturating_sub(error_rows);
    let scroll = visible
        .iter()
        .position(|index| *index == app.selected[panel - 1])
        .map(|selected_row| {
            let item_scroll = selected_row.saturating_sub(list_rows.saturating_sub(1));
            item_scroll + if item_scroll > 0 { error_rows } else { 0 }
        })
        .unwrap_or(0)
        .min(usize::from(u16::MAX)) as u16;

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .scroll((scroll, 0))
            .block(pane_block(&title, app.focus == panel)),
        area,
    );
}

fn resource_line(resource: &Resource, selected: bool) -> Line<'static> {
    let style = if selected {
        Style::default()
            .fg(config::SELECTION_TEXT_COLOR)
            .bg(config::SELECTION_BACKGROUND_COLOR)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let label = if resource.label.is_empty() {
        &resource.id
    } else {
        &resource.label
    };

    Line::from(vec![
        Span::styled(" ", style),
        Span::styled(safe_text(label), style),
        Span::styled("  ", style),
        Span::styled(safe_text(&resource.summary), style),
    ])
}

fn draw_details_and_log(frame: &mut Frame, area: Rect, app: &App) {
    let log_height = COMMAND_LOG_HEIGHT.min(area.height);
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(log_height)])
        .split(area);

    draw_details(frame, parts[0], app);
    draw_logs(frame, parts[1], app);
}

fn draw_details(frame: &mut Frame, area: Rect, app: &App) {
    let stale = app.error.is_some();
    let title = if stale {
        "[0] Details • stale"
    } else {
        "[0] Details"
    };
    let mut lines = Vec::new();

    if let Some(error) = &app.error {
        lines.push(Line::from(Span::styled(
            format!("Refresh failed; showing last data: {}", safe_text(error)),
            Style::default()
                .fg(config::ERROR_COLOR)
                .add_modifier(Modifier::BOLD),
        )));
    }

    let selected = app
        .selected_resource()
        .or_else(|| app.snapshot.panels[0].items.get(app.selected[0]));
    if let Some(resource) = selected {
        let json = serde_json::to_string_pretty(&resource.detail)
            .unwrap_or_else(|error| format!("Could not format resource: {error}"));
        lines.extend(json.lines().map(json_line));
    } else {
        lines.push(Line::from(Span::styled(
            if app.loading {
                "Waiting for AWS data…"
            } else {
                "Select a resource to view its JSON."
            },
            Style::default().fg(config::MUTED_COLOR),
        )));
    }

    let max_scroll = lines
        .len()
        .saturating_sub(usize::from(area.height.saturating_sub(2)))
        .min(usize::from(u16::MAX)) as u16;
    let scroll = app.detail_scroll.min(max_scroll);

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .scroll((scroll, 0))
            .block(pane_block(title, app.focus == 5)),
        area,
    );
}

fn json_line(line: &str) -> Line<'static> {
    let (key, value) = match line.split_once(": ") {
        Some((key, value)) => (
            Some(Span::styled(
                format!("{key}: "),
                Style::default().fg(config::KEY_COLOR),
            )),
            value,
        ),
        None => (None, line),
    };
    let value_style = match value.trim_start().chars().next() {
        Some('"') => config::JSON_STRING_COLOR,
        Some('t' | 'f' | 'n') => config::JSON_LITERAL_COLOR,
        Some('0'..='9' | '-') => config::JSON_NUMBER_COLOR,
        _ => config::MUTED_COLOR,
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

fn draw_logs(frame: &mut Frame, area: Rect, app: &App) {
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
            Style::default().fg(config::MUTED_COLOR),
        ))]
    } else {
        lines
    };

    frame.render_widget(
        Paragraph::new(Text::from(lines)).block(pane_block(&title, app.focus == 6)),
        area,
    );
}

fn draw_key_bar(frame: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled(" 1 ", Style::default().fg(config::KEY_COLOR)),
        Span::raw("status  "),
        Span::styled("2–5", Style::default().fg(config::KEY_COLOR)),
        Span::raw(" lists  "),
        Span::styled("0", Style::default().fg(config::KEY_COLOR)),
        Span::raw(" detail  "),
        Span::styled("6", Style::default().fg(config::KEY_COLOR)),
        Span::raw(" log  "),
        Span::styled("j/k", Style::default().fg(config::KEY_COLOR)),
        Span::raw(" move  "),
        Span::styled("Tab", Style::default().fg(config::KEY_COLOR)),
        Span::raw(" panels  "),
        Span::styled("/", Style::default().fg(config::KEY_COLOR)),
        Span::raw(" filter  "),
        Span::styled("[ ]", Style::default().fg(config::KEY_COLOR)),
        Span::raw(" services  "),
        Span::styled("?", Style::default().fg(config::KEY_COLOR)),
        Span::raw(" help  "),
        Span::styled("q", Style::default().fg(config::KEY_COLOR)),
        Span::raw(" quit"),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let width = HELP_WIDTH.min(area.width.saturating_sub(2));
    let height = HELP_HEIGHT.min(area.height.saturating_sub(2));
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    let lines = [
        ("Service", "[ / ] or Shift+←/→ switch service"),
        ("Focus", "1 status  2–5 lists  0 detail  6 log"),
        ("Move", "j/k or ↑/↓ select a resource"),
        ("Panels", "Tab/Shift+Tab or h/l cycle lists"),
        ("Filter", "/ then type; Enter/Esc finish editing"),
        ("Details", "Enter opens  Esc returns  PgUp/PgDn scroll"),
        ("Refresh", "r refresh AWS data"),
        ("Help", "? show or close this screen"),
        ("Quit", "q or Ctrl+C quit"),
    ]
    .into_iter()
    .map(|(key, description)| {
        Line::from(vec![
            Span::styled(
                format!("{key:<10}"),
                Style::default().fg(config::TITLE_COLOR),
            ),
            Span::raw(description),
        ])
    })
    .collect::<Vec<_>>();

    frame.render_widget(Clear, rect);
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(pane_block("Keybindings", true))
            .wrap(Wrap { trim: true }),
        rect,
    );
}

fn pane_block(title: &str, focused: bool) -> Block<'static> {
    Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if focused {
            Style::default()
                .fg(config::FOCUS_COLOR)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(config::BORDER_COLOR)
        })
        .title_style(
            Style::default()
                .fg(config::TITLE_COLOR)
                .add_modifier(Modifier::BOLD),
        )
}

fn safe_text(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect()
}
