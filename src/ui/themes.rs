use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Clear, Paragraph},
};

use crate::{
    app::App,
    config::{THEME_HEIGHT, THEME_WIDTH},
    theme::Theme,
};

use super::{HitMap, MouseTarget, chrome::pane_block};

pub(super) fn draw_theme_picker(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    selected: usize,
    hit_map: &mut HitMap,
) {
    let theme = app.theme();
    let width = THEME_WIDTH.min(area.width.saturating_sub(2));
    let height = THEME_HEIGHT.min(area.height.saturating_sub(2));
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

    let mut lines = vec![Line::from(Span::styled(
        "↑/↓ preview · Enter or click to apply · Esc cancel",
        Style::default().fg(theme.muted),
    ))];
    let first_row = content.y.saturating_add(1);
    let visible_rows = usize::from(content.height.saturating_sub(1));
    let start = selected.saturating_sub(visible_rows.saturating_sub(1));

    for (index, preset) in app.themes.iter().enumerate().skip(start).take(visible_rows) {
        lines.push(theme_line(
            preset,
            index == selected,
            index == app.theme,
            theme,
        ));
        hit_map.regions.push((
            Rect::new(
                content.x,
                first_row.saturating_add((index - start) as u16),
                content.width,
                1,
            ),
            MouseTarget::ChooseTheme(index),
        ));
    }

    frame.render_widget(Clear, rect);
    frame.render_widget(
        Paragraph::new(Text::from(lines)).block(pane_block("Themes", true, theme)),
        rect,
    );
}

fn theme_line(preset: &Theme, selected: bool, active: bool, theme: &Theme) -> Line<'static> {
    let style = if selected {
        Style::default()
            .fg(theme.selection_text)
            .bg(theme.selection_background)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let swatch = |color| Span::styled("■", Style::default().fg(color).bg(preset.background));

    Line::from(vec![
        Span::styled(if selected { "› " } else { "  " }, style),
        Span::styled(format!("{:<22}", preset.name), style),
        Span::styled(format!("{:<9}", preset.appearance), style),
        Span::styled(" ", Style::default().bg(preset.background)),
        swatch(preset.text),
        swatch(preset.focus),
        swatch(preset.key),
        swatch(preset.warning),
        swatch(preset.error),
        Span::styled(" ", Style::default().bg(preset.background)),
        Span::styled(if active { "  ✓ current" } else { "" }, style),
    ])
}
