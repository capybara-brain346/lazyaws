use std::{collections::VecDeque, mem, time::Instant};

use crate::{
    config::{DEFAULT_THEME, LOG_CAPACITY, THEMES_JSON},
    cost_cache::CostCache,
    model::{PANEL_COUNT, Panel, Resource, Service, Snapshot},
    operations::AccountContext,
    theme::{self, Theme},
};

mod actions;
mod cost;
mod focus;
mod input;
mod themes;

pub use actions::ActionDialog;
pub use cost::CostPrompt;
pub use focus::Focus;
pub use input::Action;

#[derive(Clone, Debug, Default)]
struct ServiceState {
    selected: [usize; PANEL_COUNT],
    selected_ids: [Option<String>; PANEL_COUNT],
    filter: String,
    filter_panel: usize,
}

pub struct App {
    pub service: Service,
    pub snapshot: Snapshot,
    pub focus: Focus,
    pub selected: [usize; PANEL_COUNT],
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
    pub themes: Vec<Theme>,
    pub theme: usize,
    pub theme_picker: Option<usize>,
    pub cost_prompt: Option<CostPrompt>,
    pub cost_cache: Option<CostCache>,
    cost_origin: Option<Service>,
    service_states: [ServiceState; Service::ALL.len()],
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
        let mut themes = vec![DEFAULT_THEME];
        let mut logs = VecDeque::new();
        match theme::load(THEMES_JSON) {
            Ok(presets) => themes.extend(presets),
            Err(error) => logs.push_back(format!("Themes unavailable: {error}")),
        }

        Self {
            service,
            snapshot: Snapshot::empty(service),
            focus: Focus::List(0),
            selected: [0; PANEL_COUNT],
            filter: String::new(),
            filtering: false,
            help: false,
            detail_scroll: 0,
            loading: true,
            error: None,
            logs,
            last_updated: None,
            actions: None,
            executing: false,
            account: None,
            operation_output: None,
            themes,
            theme: 0,
            theme_picker: None,
            cost_prompt: None,
            cost_cache: None,
            cost_origin: None,
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
        let Snapshot {
            service,
            panels: mut incoming,
            logs,
        } = snapshot;
        let mut previous = mem::replace(&mut self.snapshot, Snapshot::empty(service)).panels;

        let primary_failed = incoming[0].error.is_some() && incoming[0].items.is_empty();
        if primary_failed {
            let failed = mem::take(&mut incoming[0]);
            incoming[0] = Panel {
                items: mem::take(&mut previous[0].items),
                ..failed
            };
        }
        self.selected[0] = old_state.selected_ids[0]
            .as_ref()
            .and_then(|id| incoming[0].items.iter().position(|item| item.id == *id))
            .unwrap_or(0);
        let new_primary = incoming[0]
            .items
            .get(self.selected[0])
            .map(|resource| resource.id.as_str());
        let primary_changed = old_primary
            .as_deref()
            .is_some_and(|old| Some(old) != new_primary);

        for index in 1..PANEL_COUNT {
            if primary_failed {
                let mut stale = mem::take(&mut previous[index]);
                stale.error.clone_from(&incoming[0].error);
                incoming[index] = stale;
            } else if incoming[index].error.is_some()
                && incoming[index].items.is_empty()
                && !primary_changed
            {
                let failed = mem::take(&mut incoming[index]);
                incoming[index] = Panel {
                    items: mem::take(&mut previous[index].items),
                    ..failed
                };
            }
            self.selected[index] = old_state.selected_ids[index]
                .as_ref()
                .and_then(|id| incoming[index].items.iter().position(|item| item.id == *id))
                .unwrap_or_else(|| {
                    old_state.selected[index].min(incoming[index].items.len().saturating_sub(1))
                });
        }

        if primary_changed {
            self.selected[1..].fill(0);
            self.clear_related_selection_state();
        }

        self.snapshot = Snapshot {
            service,
            panels: incoming,
            logs: Vec::new(),
        };
        let new_primary = self.selected_primary().map(str::to_owned);
        if self.snapshot.panels[0].error.is_none() && self.snapshot.panels[0].items.is_empty() {
            self.service_states[self.service.index()].selected_ids[0] = None;
        } else if let Some(id) = new_primary {
            self.service_states[self.service.index()].selected_ids[0] = Some(id);
        }
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
        let panel = self.action_panel()?;
        self.snapshot.panels[panel].items.get(self.selected[panel])
    }

    fn action_panel(&self) -> Option<usize> {
        match self.focus {
            Focus::List(panel) => Some(panel),
            Focus::Detail => Some(self.detail_panel),
            Focus::Status | Focus::Log => None,
        }
    }

    pub fn visible_indices(&self, panel: usize) -> Vec<usize> {
        let Some(data) = self.snapshot.panels.get(panel) else {
            return Vec::new();
        };
        if self.filter.is_empty() || panel != self.filter_panel {
            return (0..data.items.len()).collect();
        }

        let needle = self.filter.to_lowercase();
        let matches = |text: &str| text.to_lowercase().contains(&needle);
        data.items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                // Serialize the JSON detail only when the short fields do not match.
                (matches(&item.id)
                    || matches(&item.label)
                    || matches(&item.summary)
                    || matches(&item.detail.to_string()))
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
        for index in 0..PANEL_COUNT {
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

    pub fn append_logs(&mut self, lines: impl IntoIterator<Item = String>) {
        self.logs.extend(lines);
        while self.logs.len() > LOG_CAPACITY {
            self.logs.pop_front();
        }
    }
}

#[cfg(test)]
mod tests;
