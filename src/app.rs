use std::{collections::VecDeque, time::Instant};

use crate::{
    config::LOG_CAPACITY,
    model::{Resource, Service, Snapshot},
    operations::AccountContext,
};

mod actions;
mod input;

pub use actions::ActionDialog;
pub use input::Action;

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
    pub actions: Option<ActionDialog>,
    pub executing: bool,
    pub account: Option<Result<AccountContext, String>>,
    pub operation_output: Option<String>,
    service_states: [ServiceState; 5],
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
            focus: 1,
            selected: [0; 4],
            filter: String::new(),
            filtering: false,
            help: false,
            detail_scroll: 0,
            loading: true,
            error: None,
            logs: VecDeque::new(),
            last_updated: None,
            actions: None,
            executing: false,
            account: None,
            operation_output: None,
            service_states: std::array::from_fn(|_| ServiceState::default()),
            detail_panel: 0,
            filter_panel: 0,
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

    fn clear_related(&mut self) {
        Self::clear_snapshot_related(&mut self.snapshot);
        self.selected[1..].fill(0);
        self.detail_scroll = 0;
        self.error = None;
        self.operation_output = None;
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
mod tests;
