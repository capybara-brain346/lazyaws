use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Clear, Paragraph, Wrap},
};

use crate::{
    app::{ActionDialog, App},
    config::{self, ACTION_HEIGHT, ACTION_WIDTH, Config},
    operations::Operation,
};

use super::{HitMap, MouseTarget, chrome::pane_block, chrome::safe_text};

pub(super) fn draw_action_dialog(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    dialog: &ActionDialog,
    config: &Config,
    hit_map: &mut HitMap,
) {
    let width = ACTION_WIDTH.min(area.width.saturating_sub(2));
    let height = ACTION_HEIGHT.min(area.height.saturating_sub(2));
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    let content = Rect::new(
        rect.x.saturating_add(1),
        rect.y.saturating_add(1),
        rect.width.saturating_sub(2),
        rect.height.saturating_sub(2),
    );
    let button_y = content.y.saturating_add(content.height.saturating_sub(1));
    let (title, lines, primary_button) = if dialog.confirming || dialog.editing {
        draw_selected_action(app, dialog, config)
    } else {
        draw_action_menu(content, button_y, dialog, hit_map)
    };

    frame.render_widget(Clear, rect);
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(pane_block(title, true))
            .wrap(Wrap { trim: true }),
        rect,
    );
    draw_controls(
        frame,
        content,
        button_y,
        primary_button,
        app.executing,
        hit_map,
    );
}

fn draw_action_menu(
    content: Rect,
    button_y: u16,
    dialog: &ActionDialog,
    hit_map: &mut HitMap,
) -> (&'static str, Vec<Line<'static>>, Option<&'static str>) {
    let mut lines = vec![Line::from(Span::styled(
        "↑/↓ select · Enter or click to choose · Esc close",
        Style::default().fg(config::MUTED_COLOR),
    ))];
    let first_row = content.y.saturating_add(1);
    let visible_rows = button_y.saturating_sub(first_row);
    let start = dialog
        .selected
        .saturating_sub(usize::from(visible_rows).saturating_sub(1));

    if dialog.operations.is_empty() {
        lines.push(Line::from(Span::styled(
            "No actions are available for this resource.",
            Style::default().fg(config::MUTED_COLOR),
        )));
    } else {
        for (index, operation) in dialog
            .operations
            .iter()
            .enumerate()
            .skip(start)
            .take(usize::from(visible_rows))
        {
            lines.push(action_menu_line(operation, index == dialog.selected));
            hit_map.regions.push((
                Rect::new(
                    content.x,
                    first_row.saturating_add((index - start) as u16),
                    content.width,
                    1,
                ),
                MouseTarget::ChooseAction(index),
            ));
        }
    }

    ("Choose action", lines, None)
}

fn action_menu_line(operation: &Operation, selected: bool) -> Line<'static> {
    let style = if selected {
        Style::default()
            .fg(config::SELECTION_TEXT_COLOR)
            .bg(config::SELECTION_BACKGROUND_COLOR)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    Line::from(vec![
        Span::styled(if selected { "› " } else { "  " }, style),
        Span::styled(safe_text(operation.label), style),
    ])
}

fn draw_selected_action(
    app: &App,
    dialog: &ActionDialog,
    config: &Config,
) -> (&'static str, Vec<Line<'static>>, Option<&'static str>) {
    let Some(operation) = dialog.operations.get(dialog.selected) else {
        return (
            "Choose action",
            vec![Line::from("No action is selected.")],
            None,
        );
    };

    let mut lines = vec![
        detail_line("Action", operation.label),
        detail_line("Target", &operation.target),
        account_line(app, operation, config),
        Line::from(vec![
            Span::styled(
                "Warning: ",
                Style::default()
                    .fg(config::WARNING_COLOR)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(safe_text(operation.warning)),
        ]),
    ];

    if let Some(parameter) = operation.parameter {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{}: ", parameter.label()),
                Style::default().fg(config::TITLE_COLOR),
            ),
            Span::raw(safe_text(&dialog.input)),
            if dialog.editing {
                Span::styled("▏", Style::default().fg(config::WARNING_COLOR))
            } else {
                Span::raw("")
            },
        ]));
        lines.push(Line::from(Span::styled(
            if app.executing {
                "Running action… wait for the result"
            } else if dialog.editing {
                "Enter accepts this value · Esc cancels"
            } else {
                config::ACTION_CONFIRM_PROMPT
            },
            Style::default().fg(config::MUTED_COLOR),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            if app.executing {
                "Running action… wait for the result"
            } else {
                config::ACTION_CONFIRM_PROMPT
            },
            Style::default().fg(config::MUTED_COLOR),
        )));
    }

    match &dialog.preview {
        Ok(args) => lines.push(Line::from(vec![
            Span::styled("Command: ", Style::default().fg(config::TITLE_COLOR)),
            Span::raw(command_preview(config, operation, args)),
        ])),
        Err(error) => lines.push(Line::from(Span::styled(
            format!("Command: {}", safe_text(error)),
            Style::default().fg(config::ERROR_COLOR),
        ))),
    }

    if let Some(error) = &dialog.error {
        lines.push(Line::from(Span::styled(
            format!("Input error: {}", safe_text(error)),
            Style::default().fg(config::ERROR_COLOR),
        )));
    }

    let primary_button = if app.executing {
        None
    } else if dialog.editing {
        Some("Accept")
    } else if dialog.confirming {
        Some("Y")
    } else {
        None
    };
    (
        if dialog.editing {
            "Set action input"
        } else if app.executing {
            "Running action"
        } else {
            "Confirm action"
        },
        lines,
        primary_button,
    )
}

fn account_line(app: &App, operation: &Operation, config: &Config) -> Line<'static> {
    let profile = config
        .profile
        .as_deref()
        .unwrap_or(config::DEFAULT_PROFILE_LABEL);
    let region = operation
        .region
        .as_deref()
        .or(config.region.as_deref())
        .unwrap_or(config::DEFAULT_REGION_LABEL);
    let identity = match &app.account {
        Some(Ok(account)) => safe_text(&account.account),
        Some(Err(_)) => "unavailable".into(),
        None => "loading".into(),
    };
    Line::from(vec![
        Span::styled("Account: ", Style::default().fg(config::TITLE_COLOR)),
        Span::raw(identity),
        Span::raw(" · "),
        Span::styled("Profile: ", Style::default().fg(config::TITLE_COLOR)),
        Span::raw(safe_text(profile)),
        Span::raw(" · "),
        Span::styled("Region: ", Style::default().fg(config::TITLE_COLOR)),
        Span::raw(safe_text(region)),
    ])
}

fn detail_line(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default().fg(config::TITLE_COLOR),
        ),
        Span::raw(safe_text(value)),
    ])
}

fn draw_controls(
    frame: &mut Frame,
    content: Rect,
    y: u16,
    primary: Option<&str>,
    executing: bool,
    hit_map: &mut HitMap,
) {
    if executing {
        frame.render_widget(
            Paragraph::new(Line::default()),
            Rect::new(content.x, y, content.width, 1),
        );
        return;
    }

    let start = content.x.saturating_add(2);
    let primary_text = primary.map(|label| format!("[ {label} ]"));
    let primary_width = primary_text.as_ref().map_or(0, |label| label.len() as u16);
    let cancel_text = if primary == Some("Y") {
        "[ N ]"
    } else {
        "[ Cancel ]"
    };
    let cancel_x = if primary.is_some() {
        start.saturating_add(primary_width).saturating_add(3)
    } else {
        start
    };
    let controls = if let Some(primary_text) = &primary_text {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(primary_text, Style::default().fg(config::KEY_COLOR)),
            Span::raw("   "),
            Span::styled(cancel_text, Style::default().fg(config::KEY_COLOR)),
        ])
    } else {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(cancel_text, Style::default().fg(config::KEY_COLOR)),
        ])
    };
    frame.render_widget(
        Paragraph::new(controls),
        Rect::new(content.x, y, content.width, 1),
    );

    if primary.is_some() {
        hit_map.regions.push((
            Rect::new(
                start,
                y,
                primary_width.min(content.right().saturating_sub(start)),
                1,
            ),
            MouseTarget::ConfirmAction,
        ));
    }
    hit_map.regions.push((
        Rect::new(
            cancel_x,
            y,
            (cancel_text.len() as u16).min(content.right().saturating_sub(cancel_x)),
            1,
        ),
        MouseTarget::CancelAction,
    ));
}

fn command_preview(config: &Config, operation: &Operation, args: &[String]) -> String {
    let mut command_args = Vec::new();
    if let Some(profile) = &config.profile {
        command_args.extend(["--profile".to_string(), profile.clone()]);
    }
    if let Some(region) = operation.region.as_deref().or(config.region.as_deref()) {
        command_args.extend(["--region".to_string(), region.to_string()]);
    }
    command_args.extend_from_slice(args);
    std::iter::once(shell_quote(&config.aws_bin))
        .chain(command_args.iter().map(|arg| shell_quote(arg)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_quote(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-._/:=,[]{}".contains(character))
    {
        value.to_owned()
    } else {
        format!("'{}'", safe_text(value).replace('\'', "'\\''"))
    }
}
