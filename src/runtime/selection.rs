use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::{Frame, buffer::Buffer, layout::Rect, style::Style, text::Span};

use crate::{
    app::{Action, App},
    theme::Theme,
    ui::HitMap,
};

#[derive(Default)]
pub(super) struct TextSelection {
    drag: Option<Drag>,
}

struct Drag {
    screen: Buffer,
    press: MouseEvent,
    end: (u16, u16),
    moved: bool,
}

impl TextSelection {
    pub(super) fn clear(&mut self) {
        self.drag = None;
    }

    pub(super) fn handle_mouse(
        &mut self,
        event: MouseEvent,
        screen: &Buffer,
        app: &mut App,
        targets: &HitMap,
    ) -> Action {
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.drag = screen.cell((event.column, event.row)).map(|_| Drag {
                    screen: screen.clone(),
                    press: event,
                    end: (event.column, event.row),
                    moved: false,
                });
                Action::None
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(drag) = &mut self.drag {
                    drag.update(event);
                }
                Action::None
            }
            MouseEventKind::Up(MouseButton::Left) => {
                let Some(mut drag) = self.drag.take() else {
                    return Action::None;
                };
                drag.update(event);
                if !drag.moved {
                    return app.handle_mouse(drag.press, targets);
                }

                let text = drag.text();
                if text.trim().is_empty() {
                    Action::None
                } else {
                    Action::Copy(text)
                }
            }
            MouseEventKind::Moved => Action::None,
            _ => {
                self.clear();
                app.handle_mouse(event, targets)
            }
        }
    }

    pub(super) fn draw(&mut self, frame: &mut Frame, theme: &Theme) -> bool {
        let Some(drag) = &self.drag else {
            return false;
        };
        if drag.screen.area != frame.area() {
            self.clear();
            return false;
        }

        frame.buffer_mut().clone_from(&drag.screen);
        if drag.moved {
            frame.buffer_mut().set_style(
                drag.area(),
                Style::default()
                    .fg(theme.text_selection_text)
                    .bg(theme.text_selection_background),
            );
        }
        true
    }
}

impl Drag {
    fn update(&mut self, event: MouseEvent) {
        self.end = (
            event.column.clamp(
                self.screen.area.left(),
                self.screen.area.right().saturating_sub(1),
            ),
            event.row.clamp(
                self.screen.area.top(),
                self.screen.area.bottom().saturating_sub(1),
            ),
        );
        self.moved |= self.end != (self.press.column, self.press.row);
    }

    fn area(&self) -> Rect {
        Rect::new(
            self.press.column.min(self.end.0),
            self.press.row.min(self.end.1),
            self.press.column.abs_diff(self.end.0).saturating_add(1),
            self.press.row.abs_diff(self.end.1).saturating_add(1),
        )
    }

    fn text(&self) -> String {
        let area = self.area();
        let mut lines = Vec::new();
        for y in area.top()..area.bottom() {
            let mut line = String::new();
            let mut x = self.screen.area.left();
            while x < area.right() {
                let symbol = self.screen[(x, y)].symbol();
                let width = Span::raw(symbol).width().max(1) as u16;
                if x.saturating_add(width) > area.left() {
                    line.push_str(symbol);
                }
                x = x.saturating_add(width);
            }
            lines.push(line.trim_end().to_owned());
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests;
