use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

mod actions;
mod chrome;
mod details;
mod mouse;

pub use mouse::{HitMap, MouseTarget};

use chrome::{draw_help, draw_key_bar, pane_block, safe_text};
use details::draw_details_and_log;

use crate::{
    app::App,
    config::{
        self, Config, FOOTER_HEIGHT, MIN_HEIGHT, MIN_WIDTH, SERVICE_BAR_HEIGHT, STATUS_PANEL_HEIGHT,
    },
    model::{Resource, Service},
};

pub fn draw(frame: &mut Frame, app: &App, config: &Config) {
    let _ = draw_interactive(frame, app, config);
}

pub fn draw_interactive(frame: &mut Frame, app: &App, config: &Config) -> HitMap {
    let mut hit_map = HitMap::default();
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        draw_too_small(frame, area);
        return hit_map;
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(SERVICE_BAR_HEIGHT),
            Constraint::Min(0),
            Constraint::Length(FOOTER_HEIGHT),
        ])
        .split(area);

    draw_services(frame, layout[0], app, &mut hit_map);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(
                config::SIDE_PANEL_WIDTH_NUMERATOR,
                config::SIDE_PANEL_WIDTH_DENOMINATOR,
            ),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(layout[1]);

    let mut pane_regions = Vec::new();
    let mut row_regions = Vec::new();
    draw_side_panels(
        frame,
        columns[0],
        app,
        config,
        &mut pane_regions,
        &mut row_regions,
    );
    draw_details_and_log(frame, columns[2], app, config, &mut pane_regions);
    draw_key_bar(frame, layout[2]);
    hit_map.regions.extend(pane_regions);
    hit_map.regions.extend(row_regions);
    hit_map.regions.push((
        Rect::new(
            layout[2].x.saturating_add(1),
            layout[2].y,
            9.min(layout[2].width.saturating_sub(1)),
            1.min(layout[2].height),
        ),
        MouseTarget::OpenActions,
    ));

    if let Some(dialog) = &app.actions {
        actions::draw_action_dialog(frame, area, app, dialog, config, &mut hit_map);
    } else if app.help {
        draw_help(frame, area);
    }

    hit_map
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

fn draw_services(frame: &mut Frame, area: Rect, app: &App, hit_map: &mut HitMap) {
    let mut spans = vec![Span::styled(
        " lazyaws ",
        Style::default()
            .fg(config::TITLE_COLOR)
            .add_modifier(Modifier::BOLD),
    )];

    let mut tab_x = area.x.saturating_add(9);
    for service in Service::ALL {
        let selected = service == app.service;
        let width = service.name().len() as u16 + 2;
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
        hit_map.regions.push((
            Rect::new(
                tab_x,
                area.y,
                width.min(area.right().saturating_sub(tab_x)),
                1,
            ),
            MouseTarget::Service(service),
        ));
        tab_x = tab_x.saturating_add(width);
    }

    spans.push(Span::styled(
        "  [/] switch service",
        Style::default().fg(config::MUTED_COLOR),
    ));
    let account = match &app.account {
        Some(Ok(account)) => format!(
            " Account: {} · {}",
            safe_text(&account.account),
            safe_text(short_principal(&account.arn))
        ),
        Some(Err(error)) => format!(" Account: unavailable · {}", safe_text(error)),
        None => " Account: loading".into(),
    };
    let mut lines = vec![Line::from(spans), Line::from(account)];
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
    } else {
        lines.push(Line::default());
    }
    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn short_principal(arn: &str) -> &str {
    arn.splitn(6, ':').nth(5).unwrap_or(arn)
}

fn draw_side_panels(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    config: &Config,
    pane_regions: &mut Vec<(Rect, MouseTarget)>,
    row_regions: &mut Vec<(Rect, MouseTarget)>,
) {
    let constraints = if area.height >= config::NORMAL_LAYOUT_MIN_HEIGHT {
        vec![
            Constraint::Length(STATUS_PANEL_HEIGHT),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
            if app.focus == 4 {
                Constraint::Fill(1)
            } else {
                Constraint::Length(config::COMPACT_SIDE_PANEL_HEIGHT)
            },
        ]
    } else {
        let height = if area.height >= config::COMPACT_LAYOUT_MIN_HEIGHT {
            config::COMPACT_SIDE_PANEL_HEIGHT
        } else {
            1
        };
        (0..5)
            .map(|panel| {
                if panel == app.focus.min(4) {
                    Constraint::Fill(1)
                } else {
                    Constraint::Length(height)
                }
            })
            .collect()
    };

    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    draw_status(frame, areas[0], app, config, pane_regions);
    for panel_index in 0..4 {
        draw_resource_panel(
            frame,
            areas[panel_index + 1],
            app,
            panel_index + 1,
            pane_regions,
            row_regions,
        );
    }
}

fn draw_status(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    config: &Config,
    pane_regions: &mut Vec<(Rect, MouseTarget)>,
) {
    let profile = config
        .profile
        .as_deref()
        .unwrap_or(config::DEFAULT_PROFILE_LABEL);
    let region = config
        .region
        .as_deref()
        .unwrap_or(config::DEFAULT_REGION_LABEL);
    let mut spans = vec![
        Span::styled(" pf:", Style::default().fg(config::TITLE_COLOR)),
        Span::raw(safe_text(profile)),
        Span::styled(" rg:", Style::default().fg(config::TITLE_COLOR)),
        Span::raw(safe_text(region)),
    ];

    if config.demo {
        spans.push(Span::styled(
            " DEMO",
            Style::default()
                .fg(config::WARNING_COLOR)
                .add_modifier(Modifier::BOLD),
        ));
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).block(pane_block("[1] Status", app.focus == 0)),
        area,
    );
    pane_regions.push((area, MouseTarget::Pane(0)));
}

fn draw_resource_panel(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    panel: usize,
    pane_regions: &mut Vec<(Rect, MouseTarget)>,
    row_regions: &mut Vec<(Rect, MouseTarget)>,
) {
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
            lines.push(resource_line(resource, selected, app.focus == panel));
        }
    }

    let error_rows = if data.error.is_some() { 1 } else { 0 };
    let list_rows = usize::from(area.height.saturating_sub(2)).saturating_sub(error_rows);
    let scroll = resource_scroll(&visible, app.selected[panel - 1], list_rows, error_rows);

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .scroll((scroll, 0))
            .block(pane_block(&title, app.focus == panel)),
        area,
    );

    pane_regions.push((area, MouseTarget::Pane(panel)));
    let content = Rect::new(
        area.x.saturating_add(1),
        area.y.saturating_add(1),
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    );
    for (row, index) in visible.into_iter().enumerate() {
        let line = row
            .saturating_add(error_rows)
            .saturating_sub(usize::from(scroll));
        if row + error_rows < usize::from(scroll) || line >= usize::from(content.height) {
            continue;
        }
        let y = content.y.saturating_add(line as u16);
        row_regions.push((
            Rect::new(content.x, y, content.width, 1),
            MouseTarget::Row {
                panel: panel - 1,
                focus: panel,
                index,
            },
        ));
    }
}

fn resource_scroll(visible: &[usize], selected: usize, list_rows: usize, error_rows: usize) -> u16 {
    visible
        .iter()
        .position(|index| *index == selected)
        .map(|selected_row| {
            let item_scroll = selected_row.saturating_sub(list_rows.saturating_sub(1));
            item_scroll + if item_scroll > 0 { error_rows } else { 0 }
        })
        .unwrap_or(0)
        .min(usize::from(u16::MAX)) as u16
}

fn resource_line(resource: &Resource, selected: bool, focused: bool) -> Line<'static> {
    let style = if selected {
        Style::default()
            .fg(config::SELECTION_TEXT_COLOR)
            .bg(if focused {
                config::SELECTION_BACKGROUND_COLOR
            } else {
                config::INACTIVE_SELECTION_BACKGROUND_COLOR
            })
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
