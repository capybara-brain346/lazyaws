use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use crate::{
    config::DETAIL_SCROLL_STEP,
    model::Service,
    ui::{HitMap, MouseTarget},
};

use super::App;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    Quit,
    Refresh,
    Execute(crate::operations::Operation, String),
}

impl App {
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        if key.kind == KeyEventKind::Release {
            return Action::None;
        }

        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Action::Quit;
        }

        if self.executing {
            return Action::None;
        }

        if self.actions.is_some() {
            return self.handle_action_key(key);
        }

        if self.help {
            match key.code {
                KeyCode::Esc | KeyCode::Char('?') => self.help = false,
                KeyCode::Char('q') => return Action::Quit,
                _ => {}
            }
            return Action::None;
        }

        if self.filtering {
            return self.handle_filter_key(key);
        }

        match key.code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('r') => {
                self.loading = true;
                Action::Refresh
            }
            KeyCode::Char('a') => {
                self.open_actions();
                Action::None
            }
            KeyCode::Char('[') => self.change_service(Service::ALL[(self.service.index() + 3) % 4]),
            KeyCode::Char(']') => self.change_service(Service::ALL[(self.service.index() + 1) % 4]),
            KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.change_service(Service::ALL[(self.service.index() + 3) % 4])
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.change_service(Service::ALL[(self.service.index() + 1) % 4])
            }
            KeyCode::Char('?') => {
                self.help = !self.help;
                Action::None
            }
            KeyCode::Char('/') => {
                if (1..=4).contains(&self.focus) {
                    self.filter.clear();
                    self.filter_panel = self.focus - 1;
                    self.save_current_service_state();
                    self.filtering = true;
                }
                Action::None
            }
            KeyCode::Enter => {
                if (1..=4).contains(&self.focus) {
                    self.detail_panel = self.focus - 1;
                    self.detail_scroll = 0;
                    self.focus = 5;
                }
                Action::None
            }
            KeyCode::Esc => {
                if self.help {
                    self.help = false;
                } else if self.focus == 5 {
                    self.focus = self.detail_panel + 1;
                    self.detail_scroll = 0;
                } else {
                    self.focus = 0;
                }
                Action::None
            }
            KeyCode::Tab => {
                self.cycle_panel(1);
                Action::None
            }
            KeyCode::BackTab => {
                self.cycle_panel(-1);
                Action::None
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.cycle_panel(-1);
                Action::None
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.cycle_panel(1);
                Action::None
            }
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::Char('1') => {
                self.focus = 0;
                Action::None
            }
            KeyCode::Char(character @ '2'..='5') => {
                self.focus = character as usize - '1' as usize;
                Action::None
            }
            KeyCode::Char('0') => {
                if (1..=4).contains(&self.focus) {
                    self.detail_panel = self.focus - 1;
                }
                self.focus = 5;
                Action::None
            }
            KeyCode::Char('6') => {
                self.focus = 6;
                Action::None
            }
            KeyCode::PageUp if self.focus == 5 => {
                self.detail_scroll = self.detail_scroll.saturating_sub(DETAIL_SCROLL_STEP);
                Action::None
            }
            KeyCode::PageDown if self.focus == 5 => {
                self.detail_scroll = self.detail_scroll.saturating_add(DETAIL_SCROLL_STEP);
                Action::None
            }
            _ => Action::None,
        }
    }

    pub fn handle_mouse(&mut self, event: MouseEvent, targets: &HitMap) -> Action {
        if self.executing || self.help {
            return Action::None;
        }

        let Some(target) = targets.target_at(event.column, event.row) else {
            return Action::None;
        };

        if self.actions.is_some() {
            return match (event.kind, target) {
                (MouseEventKind::Down(MouseButton::Left), MouseTarget::ChooseAction(index)) => {
                    self.choose_action(index)
                }
                (MouseEventKind::Down(MouseButton::Left), MouseTarget::ConfirmAction) => {
                    if self.actions.as_ref().is_some_and(|dialog| dialog.editing) {
                        self.accept_action_input();
                        Action::None
                    } else {
                        self.confirm_action()
                    }
                }
                (MouseEventKind::Down(MouseButton::Left), MouseTarget::CancelAction) => {
                    self.actions = None;
                    Action::None
                }
                _ => Action::None,
            };
        }

        match event.kind {
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                let focus = match target {
                    MouseTarget::Pane(focus) | MouseTarget::Row { focus, .. } => focus,
                    _ => return Action::None,
                };
                self.focus_pane(focus);
                let delta = if event.kind == MouseEventKind::ScrollUp {
                    -1
                } else {
                    1
                };
                if focus == 5 {
                    if delta < 0 {
                        self.detail_scroll = self.detail_scroll.saturating_sub(DETAIL_SCROLL_STEP);
                    } else {
                        self.detail_scroll = self.detail_scroll.saturating_add(DETAIL_SCROLL_STEP);
                    }
                    Action::None
                } else {
                    self.move_selection(delta)
                }
            }
            MouseEventKind::Down(MouseButton::Left) => match target {
                MouseTarget::Service(service) => self.change_service(service),
                MouseTarget::OpenActions => {
                    self.open_actions();
                    Action::None
                }
                MouseTarget::Pane(focus) => {
                    self.focus_pane(focus);
                    Action::None
                }
                MouseTarget::Row {
                    panel,
                    focus,
                    index,
                } => self.select_row(panel, focus, index),
                _ => Action::None,
            },
            MouseEventKind::Down(MouseButton::Right) => match target {
                MouseTarget::Row {
                    panel,
                    focus,
                    index,
                } => {
                    let action = self.select_row(panel, focus, index);
                    if action == Action::None {
                        self.open_actions_for(panel, index);
                    }
                    action
                }
                _ => Action::None,
            },
            _ => Action::None,
        }
    }

    fn handle_filter_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Enter => self.filtering = false,
            KeyCode::Esc => {
                self.filter.clear();
                self.filtering = false;
                self.save_current_service_state();
            }
            KeyCode::Backspace => {
                self.filter.pop();
                return self.normalize_filtered_selection();
            }
            KeyCode::Char(character) => {
                self.filter.push(character);
                return self.normalize_filtered_selection();
            }
            _ => {}
        }
        Action::None
    }

    fn change_service(&mut self, service: Service) -> Action {
        if service == self.service {
            return Action::None;
        }

        self.save_current_service_state();
        self.service = service;
        let state = self.service_states[service.index()].clone();
        self.selected = state.selected;
        self.filter = state.filter;
        self.filter_panel = state.filter_panel;
        self.filtering = false;
        self.snapshot = crate::model::Snapshot::empty(service);
        self.focus = 1;
        self.detail_panel = 0;
        self.detail_scroll = 0;
        self.error = None;
        self.loading = true;
        self.operation_output = None;
        Action::Refresh
    }

    fn move_selection(&mut self, delta: isize) -> Action {
        if !(1..=4).contains(&self.focus) {
            return Action::None;
        }

        let panel = self.focus - 1;
        let visible = self.visible_indices(self.focus);
        if visible.is_empty() {
            return Action::None;
        }

        let old_primary = self.selected_primary().map(str::to_owned);
        let old_selection = self.selected[panel];
        let current = visible.iter().position(|index| *index == old_selection);
        let next = match current {
            Some(position) => {
                (position as isize + delta).rem_euclid(visible.len() as isize) as usize
            }
            None => 0,
        };
        self.selected[panel] = visible[next];
        self.detail_scroll = 0;
        if old_selection != self.selected[panel] {
            self.operation_output = None;
        }

        if panel == 0 && old_primary.as_deref() != self.selected_primary() {
            self.clear_related();
            self.loading = true;
            self.save_current_service_state();
            Action::Refresh
        } else {
            self.save_current_service_state();
            Action::None
        }
    }

    fn normalize_filtered_selection(&mut self) -> Action {
        let panel = match self.focus {
            1..=4 => self.focus - 1,
            _ => return Action::None,
        };
        let visible = self.visible_indices(self.focus);
        if visible.is_empty() || visible.contains(&self.selected[panel]) {
            self.save_current_service_state();
            return Action::None;
        }

        let old_primary = self.selected_primary().map(str::to_owned);
        self.selected[panel] = visible[0];
        self.operation_output = None;
        self.detail_scroll = 0;
        if panel == 0 && old_primary.as_deref() != self.selected_primary() {
            self.clear_related();
            self.loading = true;
            self.save_current_service_state();
            Action::Refresh
        } else {
            self.save_current_service_state();
            Action::None
        }
    }

    fn select_row(&mut self, panel: usize, focus: usize, index: usize) -> Action {
        if panel >= 4 || focus != panel + 1 || !self.visible_indices(focus).contains(&index) {
            return Action::None;
        }

        self.focus = focus;
        let old_primary = self.selected_primary().map(str::to_owned);
        if self.selected[panel] != index {
            self.selected[panel] = index;
            self.operation_output = None;
            self.detail_scroll = 0;
        }

        if panel == 0 && old_primary.as_deref() != self.selected_primary() {
            self.clear_related();
            self.loading = true;
            self.save_current_service_state();
            Action::Refresh
        } else {
            self.save_current_service_state();
            Action::None
        }
    }

    fn cycle_panel(&mut self, delta: isize) {
        let current = match self.focus {
            0..=4 => self.focus,
            5 => self.detail_panel + 1,
            _ => 0,
        };
        self.focus = (current as isize + delta).rem_euclid(5) as usize;
    }

    fn focus_pane(&mut self, focus: usize) {
        if focus == 5 && self.focus != 5 {
            if (1..=4).contains(&self.focus) {
                self.detail_panel = self.focus - 1;
            }
            self.detail_scroll = 0;
        }
        self.focus = focus;
    }
}
