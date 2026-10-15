use crossterm::event::{KeyCode, KeyEvent};

use crate::theme::Theme;

use super::{App, input::Action};

impl App {
    /// Returns the previewed theme while the picker is open, else the active theme.
    pub fn theme(&self) -> &Theme {
        let index = self.theme_picker.unwrap_or(self.theme);
        self.themes.get(index).unwrap_or(&self.themes[0])
    }

    pub(super) fn open_themes(&mut self) {
        self.theme_picker = Some(self.theme);
    }

    pub(super) fn apply_theme(&mut self, index: usize) {
        if index < self.themes.len() {
            self.theme = index;
        }
        self.theme_picker = None;
    }

    pub(super) fn handle_theme_key(&mut self, key: KeyEvent) -> Action {
        let Some(selected) = self.theme_picker else {
            return Action::None;
        };
        let count = self.themes.len();

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => self.theme_picker = Some((selected + 1) % count),
            KeyCode::Char('k') | KeyCode::Up => {
                self.theme_picker = Some((selected + count - 1) % count);
            }
            KeyCode::Enter => self.apply_theme(selected),
            KeyCode::Esc | KeyCode::Char('t') => self.theme_picker = None,
            KeyCode::Char('q') => return Action::Quit,
            _ => {}
        }
        Action::None
    }
}
