use std::time::SystemTime;

use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    config::COST_PANEL_RETRY,
    cost_cache::CostCache,
    model::{PANEL_COUNT, Service, Snapshot},
};

use super::{App, input::Action};

/// The paid Cost Explorer load that waits for confirmation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CostPrompt {
    Load,
    Refresh,
}

impl App {
    // A saved result opens for free; without one, the tab opens only after confirmation.
    // The saved result is read once the account is known, so the tab waits for it.
    pub(super) fn open_cost(&mut self) -> Action {
        if self.cost_cache.is_none() && self.account.is_none() {
            self.cost_origin = Some(self.service);
            self.enter_service(Service::Cost);
            return Action::None;
        }
        self.show_cost()
    }

    fn show_cost(&mut self) -> Action {
        match self.cost_cache.clone() {
            Some(cache) => {
                self.enter_service(Service::Cost);
                self.apply(cache.snapshot);
                // Set after `apply`, which would hide the other panels behind an empty Summary.
                for (panel, time) in self.snapshot.panels.iter_mut().zip(cache.fetched_at) {
                    if time.is_none() {
                        panel.error = Some(COST_PANEL_RETRY.into());
                    }
                }
            }
            None => self.cost_prompt = Some(CostPrompt::Load),
        }
        Action::None
    }

    /// Finishes an open of the Cost tab that waited for the account. With no saved result,
    /// it returns to the service that was open and asks for the paid load.
    pub fn resume_cost(&mut self) -> Action {
        // A later identity success can find the saved result after a failed one.
        if matches!(self.account, Some(Ok(_)))
            && self.cost_prompt == Some(CostPrompt::Load)
            && self.cost_cache.is_some()
        {
            self.cost_prompt = None;
            return self.show_cost();
        }

        let busy = self.actions.is_some()
            || self.theme_picker.is_some()
            || self.help
            || self.cost_prompt.is_some();
        let Some(origin) = self.cost_origin.filter(|_| self.account.is_some() && !busy) else {
            return Action::None;
        };

        self.cost_origin = None;
        if self.cost_cache.is_some() {
            return self.show_cost();
        }
        self.enter_service(origin);
        self.cost_prompt = Some(CostPrompt::Load);
        Action::Refresh
    }

    pub(super) fn ask_cost_refresh(&mut self) -> Action {
        if !self.loading {
            self.cost_prompt = Some(CostPrompt::Refresh);
        }
        Action::None
    }

    pub(super) fn handle_cost_prompt_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('y' | 'Y') => self.confirm_cost(),
            KeyCode::Char('n' | 'N') | KeyCode::Esc => self.cancel_cost(),
            _ => Action::None,
        }
    }

    pub(super) fn confirm_cost(&mut self) -> Action {
        match self.cost_prompt.take() {
            Some(CostPrompt::Load) => {
                self.enter_service(Service::Cost);
                Action::Refresh
            }
            Some(CostPrompt::Refresh) => {
                self.loading = true;
                Action::Refresh
            }
            None => Action::None,
        }
    }

    pub(super) fn cancel_cost(&mut self) -> Action {
        self.cost_prompt = None;
        Action::None
    }

    /// Saves the panels that loaded. A failed panel keeps its older saved rows and time.
    pub fn cache_cost_snapshot(&mut self, now: SystemTime) -> Option<&CostCache> {
        if self.service != Service::Cost
            || self
                .snapshot
                .panels
                .iter()
                .all(|panel| panel.error.is_some())
        {
            return None;
        }

        let old = self.cost_cache.take();
        let mut cache = CostCache {
            fetched_at: [None; PANEL_COUNT],
            snapshot: Snapshot::empty(Service::Cost),
        };
        for (index, panel) in self.snapshot.panels.iter().enumerate() {
            let (items, time) = match (&panel.error, &old) {
                (None, _) => (panel.items.clone(), Some(now)),
                // A failed forecast keeps the Summary rows, but the panel stays due for a retry.
                (Some(_), _) if index == 0 && !panel.items.is_empty() => (
                    panel.items.clone(),
                    old.as_ref().and_then(|old| old.fetched_at[0]),
                ),
                (Some(_), Some(old)) => (
                    old.snapshot.panels[index].items.clone(),
                    old.fetched_at[index],
                ),
                (Some(_), None) => (Vec::new(), None),
            };
            cache.snapshot.panels[index].items = items;
            cache.fetched_at[index] = time;
        }
        self.cost_cache = Some(cache);
        self.cost_cache.as_ref()
    }
}
