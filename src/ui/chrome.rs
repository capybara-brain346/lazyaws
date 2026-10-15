use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

use crate::{
    config::{ACTIONS_KEY_LABEL, ACTIONS_KEY_TEXT, HELP_HEIGHT, HELP_WIDTH},
    theme::Theme,
};

pub(super) fn draw_key_bar(frame: &mut Frame, area: Rect, theme: &Theme) {
    let line = Line::from(vec![
        Span::styled(ACTIONS_KEY_LABEL, Style::default().fg(theme.key)),
        Span::raw(ACTIONS_KEY_TEXT),
        Span::styled(" 1 ", Style::default().fg(theme.key)),
        Span::raw("status  "),
        Span::styled("2–5", Style::default().fg(theme.key)),
        Span::raw(" lists  "),
        Span::styled("0", Style::default().fg(theme.key)),
        Span::raw(" detail  "),
        Span::styled("6", Style::default().fg(theme.key)),
        Span::raw(" log  "),
        Span::styled("j/k", Style::default().fg(theme.key)),
        Span::raw(" move  "),
        Span::styled("Tab", Style::default().fg(theme.key)),
        Span::raw(" panels  "),
        Span::styled("/", Style::default().fg(theme.key)),
        Span::raw(" filter  "),
        Span::styled("[ ]", Style::default().fg(theme.key)),
        Span::raw(" services  "),
        Span::styled("t", Style::default().fg(theme.key)),
        Span::raw(" themes  "),
        Span::styled("?", Style::default().fg(theme.key)),
        Span::raw(" help  "),
        Span::styled("q", Style::default().fg(theme.key)),
        Span::raw(" quit"),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

pub(super) fn draw_help(frame: &mut Frame, area: Rect, theme: &Theme) {
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
        ("Themes", "t choose; j/k preview; Enter apply; Esc cancel"),
        ("Help", "? show or close this screen"),
        ("Quit", "q or Ctrl+C quit"),
    ]
    .into_iter()
    .map(|(key, description)| {
        Line::from(vec![
            Span::styled(format!("{key:<10}"), Style::default().fg(theme.title)),
            Span::raw(description),
        ])
    })
    .collect::<Vec<_>>();

    frame.render_widget(Clear, rect);
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(pane_block("Keybindings", true, theme))
            .wrap(Wrap { trim: true }),
        rect,
    );
}

pub(super) fn pane_block(title: &str, focused: bool, theme: &Theme) -> Block<'static> {
    Block::default()
        .style(theme.base())
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if focused {
            Style::default()
                .fg(theme.focus)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.border)
        })
        .title_style(
            Style::default()
                .fg(theme.title)
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
