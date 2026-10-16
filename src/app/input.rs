use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use crate::{
    config::DETAIL_SCROLL_STEP,
    model::{PANEL_COUNT, Service, Snapshot},
    ui::{HitMap, MouseTarget},
};

use super::{App, Focus};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    Quit,
    Refresh,
    Copy(String),
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

        if self.cost_prompt.is_some() {
            return self.handle_cost_prompt_key(key);
        }

        if self.actions.is_some() {
            return self.handle_action_key(key);
        }

        if self.theme_picker.is_some() {
            return self.handle_theme_key(key);
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
            KeyCode::Char('r') if self.service == Service::Cost => self.ask_cost_refresh(),
            KeyCode::Char('r') => {
                self.loading = true;
                Action::Refresh
            }
            KeyCode::Char('a') => {
                self.open_actions();
                Action::None
            }
            KeyCode::Char('t') => {
                self.open_themes();
                Action::None
            }
            KeyCode::Char('[') => self.change_service(self.service.previous()),
            KeyCode::Char(']') => self.change_service(self.service.next()),
            KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.change_service(self.service.previous())
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.change_service(self.service.next())
            }
            KeyCode::Char('?') => {
                self.help = !self.help;
                Action::None
            }
            KeyCode::Char('/') => {
                if let Some(panel) = self.focus.list() {
                    self.filter.clear();
                    self.filter_panel = panel;
                    self.save_current_service_state();
                    self.filtering = true;
                }
                Action::None
            }
            KeyCode::Enter => {
                if let Some(panel) = self.focus.list() {
                    self.detail_panel = panel;
                    self.detail_scroll = 0;
                    self.focus = Focus::Detail;
                }
                Action::None
            }
            KeyCode::Esc => {
                if self.focus == Focus::Detail {
                    self.focus = Focus::List(self.detail_panel);
                    self.detail_scroll = 0;
                } else {
                    self.focus = Focus::Status;
                }
                Action::None
            }
            KeyCode::Tab | KeyCode::Char('l') | KeyCode::Right => {
                self.cycle_panel(1);
                Action::None
            }
            KeyCode::BackTab | KeyCode::Char('h') | KeyCode::Left => {
                self.cycle_panel(-1);
                Action::None
            }
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::Char('1') => {
                self.focus = Focus::Status;
                Action::None
            }
            KeyCode::Char(character @ '2'..='5') => {
                self.focus = Focus::List(character as usize - '2' as usize);
                Action::None
            }
            KeyCode::Char('0') => {
                if let Some(panel) = self.focus.list() {
                    self.detail_panel = panel;
                }
                self.focus = Focus::Detail;
                Action::None
            }
            KeyCode::Char('6') => {
                self.focus = Focus::Log;
                Action::None
            }
            KeyCode::PageUp if self.focus == Focus::Detail => {
                self.scroll_detail(-1);
                Action::None
            }
            KeyCode::PageDown if self.focus == Focus::Detail => {
                self.scroll_detail(1);
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

        if self.cost_prompt.is_some() {
            return match (event.kind, target) {
                (MouseEventKind::Down(MouseButton::Left), MouseTarget::ConfirmAction) => {
                    self.confirm_cost()
                }
                (MouseEventKind::Down(MouseButton::Left), MouseTarget::CancelAction) => {
                    self.cancel_cost()
                }
                _ => Action::None,
            };
        }

        if self.theme_picker.is_some() {
            if let (MouseEventKind::Down(MouseButton::Left), MouseTarget::ChooseTheme(index)) =
                (event.kind, target)
            {
                self.apply_theme(index);
            }
            return Action::None;
        }

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
                    MouseTarget::Pane(focus) => focus,
                    MouseTarget::Row { panel, .. } => Focus::List(panel),
                    _ => return Action::None,
                };
                self.focus_pane(focus);
                let delta = if event.kind == MouseEventKind::ScrollUp {
                    -1
                } else {
                    1
                };
                if focus == Focus::Detail {
                    self.scroll_detail(delta);
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
                MouseTarget::Row { panel, index } => self.select_row(panel, index),
                _ => Action::None,
            },
            MouseEventKind::Down(MouseButton::Right) => match target {
                MouseTarget::Row { panel, index } => {
                    let action = self.select_row(panel, index);
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
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
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

        self.cost_origin = None;
        if service == Service::Cost {
            return self.open_cost();
        }

        self.enter_service(service);
        Action::Refresh
    }

    pub(super) fn enter_service(&mut self, service: Service) {
        self.save_current_service_state();
        self.service = service;
        let state = self.service_states[service.index()].clone();
        self.selected = state.selected;
        self.filter = state.filter;
        self.filter_panel = state.filter_panel;
        self.filtering = false;
        self.snapshot = Snapshot::empty(service);
        self.focus = Focus::List(0);
        self.detail_panel = 0;
        self.detail_scroll = 0;
        self.error = None;
        self.loading = true;
        self.operation_output = None;
    }

    fn move_selection(&mut self, delta: isize) -> Action {
        let Some(panel) = self.focus.list() else {
            return Action::None;
        };

        let visible = self.visible_indices(panel);
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

        self.finish_selection(panel, old_primary)
    }

    fn normalize_filtered_selection(&mut self) -> Action {
        let Some(panel) = self.focus.list() else {
            return Action::None;
        };

        let visible = self.visible_indices(panel);
        if visible.is_empty() || visible.contains(&self.selected[panel]) {
            self.save_current_service_state();
            return Action::None;
        }

        let old_primary = self.selected_primary().map(str::to_owned);
        self.selected[panel] = visible[0];
        self.operation_output = None;
        self.detail_scroll = 0;
        self.finish_selection(panel, old_primary)
    }

    fn select_row(&mut self, panel: usize, index: usize) -> Action {
        if panel >= PANEL_COUNT || !self.visible_indices(panel).contains(&index) {
            return Action::None;
        }

        self.focus = Focus::List(panel);
        let old_primary = self.selected_primary().map(str::to_owned);
        if self.selected[panel] != index {
            self.selected[panel] = index;
            self.operation_output = None;
            self.detail_scroll = 0;
        }

        self.finish_selection(panel, old_primary)
    }

    // A new primary resource invalidates the related panels, so they must load again.
    // Cost panels do not depend on the selected row, and each load is charged.
    fn finish_selection(&mut self, panel: usize, old_primary: Option<String>) -> Action {
        let action = if panel == 0
            && self.service.auto_refresh()
            && old_primary.as_deref() != self.selected_primary()
        {
            self.clear_related();
            self.loading = true;
            Action::Refresh
        } else {
            Action::None
        };
        self.save_current_service_state();
        action
    }

    fn cycle_panel(&mut self, delta: isize) {
        self.focus = self.focus.cycle(delta, self.detail_panel);
    }

    fn focus_pane(&mut self, focus: Focus) {
        if focus == Focus::Detail && self.focus != Focus::Detail {
            if let Some(panel) = self.focus.list() {
                self.detail_panel = panel;
            }
            self.detail_scroll = 0;
        }
        self.focus = focus;
    }

    fn scroll_detail(&mut self, delta: isize) {
        self.detail_scroll = if delta < 0 {
            self.detail_scroll.saturating_sub(DETAIL_SCROLL_STEP)
        } else {
            self.detail_scroll.saturating_add(DETAIL_SCROLL_STEP)
        };
    }
}
