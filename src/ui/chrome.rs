use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

use crate::config::{self, ACTIONS_KEY_LABEL, ACTIONS_KEY_TEXT, HELP_HEIGHT, HELP_WIDTH};

pub(super) fn draw_key_bar(frame: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled(ACTIONS_KEY_LABEL, Style::default().fg(config::KEY_COLOR)),
        Span::raw(ACTIONS_KEY_TEXT),
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

pub(super) fn draw_help(frame: &mut Frame, area: Rect) {
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
        ("Mouse", "Click tabs, panes, rows, and action buttons"),
        ("Copy", "Drag text; release to copy (OSC 52 required)"),
        ("Actions", "a or right-click a resource; confirm with y"),
        (
            "S3 files",
            "Upload or download one file; existing downloads stay safe",
        ),
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

pub(super) fn pane_block(title: &str, focused: bool) -> Block<'static> {
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

pub(super) fn safe_text(value: &str) -> String {
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
