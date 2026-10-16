use std::time::SystemTime;

use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span, Text},
    widgets::{Clear, Paragraph, Wrap},
};

use crate::{
    app::{App, CostPrompt},
    config::{ACTION_CONFIRM_PROMPT, COST_PROMPT_HEIGHT, COST_PROMPT_WIDTH, COST_STALE_AFTER},
    cost_cache::age_text,
};

use super::{HitMap, actions::draw_controls, chrome::pane_block};

fn saved_age(app: &App) -> Option<(String, bool)> {
    let age = app.cost_cache.as_ref()?.age(SystemTime::now());
    Some((age_text(age), age >= COST_STALE_AFTER))
}

pub(super) fn draw_cost_prompt(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    prompt: CostPrompt,
    demo: bool,
    hit_map: &mut HitMap,
) {
    let width = COST_PROMPT_WIDTH.min(area.width.saturating_sub(2));
    let height = COST_PROMPT_HEIGHT.min(area.height.saturating_sub(2));
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
    let theme = app.theme();
    let (title, request_line, cancel_line) = match prompt {
        CostPrompt::Load => (
            "Load costs?",
            "Loading Cost data causes charges on your AWS bill.",
            "Cancel keeps you on the current tab. There is no charge.",
        ),
        CostPrompt::Refresh => (
            "Refresh costs?",
            "Refreshing Cost data causes charges on your AWS bill.",
            "Cancel keeps the saved data. There is no charge.",
        ),
    };

    let mut lines = vec![
        Line::from("AWS Cost Explorer charges for each API request."),
        Line::from(Span::styled(
            request_line,
            Style::default().fg(theme.warning),
        )),
    ];
    if prompt == CostPrompt::Refresh {
        let saved = saved_age(app).map_or("none".into(), |(age, _)| format!("fetched {age}"));
        lines.push(Line::from(format!("Saved data: {saved}.")));
    }
    lines.extend([
        Line::default(),
        Line::from("The result is saved on this computer. Opening the tab again is free."),
        Line::from("This tab does not refresh automatically. Press r on this tab to refresh."),
        Line::from(cancel_line),
    ]);
    if demo {
        lines.push(Line::from(Span::styled(
            "Demo mode: no AWS request is sent, so there is no charge.",
            Style::default().fg(theme.warning),
        )));
    }
    lines.push(Line::from(Span::styled(
        ACTION_CONFIRM_PROMPT,
        Style::default().fg(theme.muted),
    )));

    frame.render_widget(Clear, rect);
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(pane_block(title, true, theme))
            .wrap(Wrap { trim: true }),
        rect,
    );
    let button_y = content.y.saturating_add(content.height.saturating_sub(1));
    draw_controls(frame, content, button_y, Some("Y"), false, theme, hit_map);
}

pub(super) fn status_line(app: &App) -> Line<'static> {
    let theme = app.theme();
    let label = Span::styled(" Cost data: ", Style::default().fg(theme.title));
    let refresh = " · r refresh (charged) · no auto-refresh";
    let Some((age, stale)) = saved_age(app) else {
        let state = if app.loading {
            "loading…"
        } else {
            "not loaded"
        };
        return Line::from(vec![label, Span::raw(state), Span::raw(refresh)]);
    };
    let age_style = Style::default().fg(if stale { theme.warning } else { theme.muted });
    let state = if app.loading {
        "refreshing… shown data fetched "
    } else {
        "fetched "
    };
    Line::from(vec![
        label,
        Span::raw(state),
        Span::styled(age, age_style),
        Span::raw(refresh),
    ])
}
