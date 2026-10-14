use std::{collections::VecDeque, time::Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::{
    config::{DETAIL_SCROLL_STEP, LOG_CAPACITY},
    model::{Resource, Service, Snapshot},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    None,
    Quit,
    Refresh,
}

#[derive(Clone, Debug, Default)]
struct ServiceState {
    selected: [usize; 4],
    selected_ids: [Option<String>; 4],
    filter: String,
    filter_panel: usize,
}

pub struct App {
    pub service: Service,
    pub snapshot: Snapshot,
    pub focus: usize,
    pub selected: [usize; 4],
    pub filter: String,
    pub filtering: bool,
    pub help: bool,
    pub detail_scroll: u16,
    pub loading: bool,
    pub error: Option<String>,
    pub logs: VecDeque<String>,
    pub last_updated: Option<Instant>,
    service_states: [ServiceState; 4],
    detail_panel: usize,
    filter_panel: usize,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let service = Service::Ec2;
        Self {
            service,
            snapshot: Snapshot::empty(service),
            focus: 0,
            selected: [0; 4],
            filter: String::new(),
            filtering: false,
            help: false,
            detail_scroll: 0,
            loading: true,
            error: None,
            logs: VecDeque::new(),
            last_updated: None,
            service_states: std::array::from_fn(|_| ServiceState::default()),
            detail_panel: 0,
            filter_panel: 0,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        if key.kind == KeyEventKind::Release {
            return Action::None;
        }

        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Action::Quit;
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
            KeyCode::Char('[') => self.change_service(-1),
            KeyCode::Char(']') => self.change_service(1),
            KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => self.change_service(-1),
            KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => self.change_service(1),
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

    pub fn apply(&mut self, snapshot: Snapshot) {
        if snapshot.service != self.service {
            return;
        }

        let old_state = self.current_service_state();
        let old_primary = self.selected_primary().map(str::to_owned);
        let old_selections = old_state.selected;
        let old_ids = old_state.selected_ids;
        let primary_failed =
            snapshot.panels[0].error.is_some() && snapshot.panels[0].items.is_empty();
        let mut primary = snapshot.panels[0].clone();
        if primary_failed {
            primary = self.snapshot.panels[0].clone();
            primary.error.clone_from(&snapshot.panels[0].error);
            primary.title.clone_from(&snapshot.panels[0].title);
        }
        self.selected[0] = old_ids[0]
            .as_ref()
            .and_then(|id| primary.items.iter().position(|item| item.id == *id))
            .unwrap_or(0);
        let new_primary = primary
            .items
            .get(self.selected[0])
            .map(|resource| resource.id.as_str());
        let primary_changed = old_primary
            .as_deref()
            .is_some_and(|old| Some(old) != new_primary);

        let mut updated = Snapshot::empty(snapshot.service);
        updated.panels[0] = primary;
        for index in 1..4 {
            let incoming = &snapshot.panels[index];
            updated.panels[index] = if primary_failed {
                let mut stale = self.snapshot.panels[index].clone();
                stale.error.clone_from(&snapshot.panels[0].error);
                stale
            } else if incoming.error.is_some() && incoming.items.is_empty() && !primary_changed {
                let mut stale = self.snapshot.panels[index].clone();
                stale.title.clone_from(&incoming.title);
                stale.error.clone_from(&incoming.error);
                stale
            } else {
                incoming.clone()
            };
            self.selected[index] = old_ids[index]
                .as_ref()
                .and_then(|id| {
                    updated.panels[index]
                        .items
                        .iter()
                        .position(|item| item.id == *id)
                })
                .unwrap_or_else(|| {
                    old_selections[index].min(updated.panels[index].items.len().saturating_sub(1))
                });
        }
        updated.logs = snapshot.logs;

        if primary_changed {
            self.selected[1..].fill(0);
            self.clear_related_selection_state();
        }

        self.snapshot = updated;
        let new_primary = self.selected_primary().map(str::to_owned);
        if self.snapshot.panels[0].error.is_none() && self.snapshot.panels[0].items.is_empty() {
            self.service_states[self.service.index()].selected_ids[0] = None;
        } else if let Some(id) = new_primary {
            self.service_states[self.service.index()].selected_ids[0] = Some(id);
        }
        let logs = self.snapshot.logs.clone();
        self.append_logs(logs);
        self.error = self.snapshot.panels.iter().find_map(|panel| {
            panel
                .error
                .as_ref()
                .map(|error| format!("{}: {error}", panel.title))
        });
        self.loading = false;
        if self.error.is_none() {
            self.last_updated = Some(Instant::now());
        }
        self.save_current_service_state();
    }

    pub fn selected_primary(&self) -> Option<&str> {
        self.snapshot.panels[0]
            .items
            .get(self.selected[0])
            .map(|resource| resource.id.as_str())
            .or_else(|| self.service_states[self.service.index()].selected_ids[0].as_deref())
    }

    pub fn selected_resource(&self) -> Option<&Resource> {
        let panel = if self.focus == 5 {
            self.detail_panel
        } else if (1..=4).contains(&self.focus) {
            self.focus - 1
        } else {
            return None;
        };
        self.snapshot.panels[panel].items.get(self.selected[panel])
    }

    pub fn visible_indices(&self, panel: usize) -> Vec<usize> {
        let Some(data) = panel
            .checked_sub(1)
            .and_then(|index| self.snapshot.panels.get(index))
        else {
            return Vec::new();
        };
        if self.filter.is_empty() || panel - 1 != self.filter_panel {
            return (0..data.items.len()).collect();
        }

        let needle = self.filter.to_lowercase();
        data.items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let detail = item.detail.to_string();
                [
                    item.id.as_str(),
                    item.label.as_str(),
                    item.summary.as_str(),
                    detail.as_str(),
                ]
                .iter()
                .any(|text| text.to_lowercase().contains(&needle))
                .then_some(index)
            })
            .collect()
    }

    pub fn context_summary(&self) -> String {
        self.snapshot.panels[0]
            .items
            .get(self.selected[0])
            .map(|resource| format!("{} ({})", resource.label, resource.id))
            .unwrap_or_else(|| self.service.name().to_owned())
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

    fn change_service(&mut self, delta: isize) -> Action {
        let target = (self.service.index() as isize + delta).rem_euclid(Service::ALL.len() as isize)
            as usize;
        let service = Service::ALL[target];
        if service == self.service {
            return Action::None;
        }

        self.save_current_service_state();
        self.service = service;
        let state = self.service_states[target].clone();
        self.selected = state.selected;
        self.filter = state.filter;
        self.filter_panel = state.filter_panel;
        self.filtering = false;
        self.snapshot = Snapshot::empty(service);
        self.focus = 0;
        self.detail_panel = 0;
        self.detail_scroll = 0;
        self.error = None;
        self.loading = true;
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
        let current = visible
            .iter()
            .position(|index| *index == self.selected[panel]);
        let next = match current {
            Some(position) => {
                (position as isize + delta).rem_euclid(visible.len() as isize) as usize
            }
            None => 0,
        };
        self.selected[panel] = visible[next];
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

    fn cycle_panel(&mut self, delta: isize) {
        let current = if (1..=4).contains(&self.focus) {
            self.focus - 1
        } else {
            0
        };
        self.focus = ((current as isize + delta).rem_euclid(4) as usize) + 1;
    }

    fn clear_related(&mut self) {
        Self::clear_snapshot_related(&mut self.snapshot);
        self.selected[1..].fill(0);
        self.detail_scroll = 0;
        self.error = None;
        self.clear_related_selection_state();
    }

    fn clear_related_selection_state(&mut self) {
        let state = &mut self.service_states[self.service.index()];
        state.selected[1..].fill(0);
        state.selected_ids[1..].fill(None);
    }

    fn clear_snapshot_related(snapshot: &mut Snapshot) {
        for panel in &mut snapshot.panels[1..] {
            panel.items.clear();
            panel.error = None;
        }
    }

    fn current_service_state(&self) -> ServiceState {
        let mut state = self.service_states[self.service.index()].clone();
        state.selected = self.selected;
        state.filter.clone_from(&self.filter);
        state.filter_panel = self.filter_panel;
        for index in 0..4 {
            state.selected_ids[index] = self.snapshot.panels[index]
                .items
                .get(self.selected[index])
                .map(|resource| resource.id.clone())
                .or_else(|| state.selected_ids[index].clone());
        }
        state
    }

    fn save_current_service_state(&mut self) {
        let state = self.current_service_state();
        self.service_states[self.service.index()] = state;
    }

    fn append_logs(&mut self, lines: impl IntoIterator<Item = String>) {
        self.logs.extend(lines);
        while self.logs.len() > LOG_CAPACITY {
            self.logs.pop_front();
        }
    }
}

#[cfg(test)]
#[path = "../tests/fixtures/demo_app.rs"]
mod tests;
