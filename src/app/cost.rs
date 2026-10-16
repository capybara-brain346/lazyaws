use std::time::SystemTime;

use crossterm::event::{KeyCode, KeyEvent};

use crate::{cost_cache::CostCache, model::Service};

use super::{App, input::Action};

/// The paid Cost Explorer load that waits for confirmation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CostPrompt {
    Load,
    Refresh,
}

impl App {
    // A saved result opens for free; without one, the tab opens only after confirmation.
    pub(super) fn open_cost(&mut self) -> Action {
        match self.cost_cache.clone() {
            Some(cache) => {
                self.enter_service(Service::Cost);
                self.apply(cache.snapshot);
                Action::None
            }
            None => {
                self.cost_prompt = Some(CostPrompt::Load);
                Action::None
            }
        }
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

    /// Saves a complete Cost result. If a panel failed, the older result stays.
    pub fn cache_cost_snapshot(&mut self, fetched_at: SystemTime) -> Option<&CostCache> {
        if self.service != Service::Cost
            || self
                .snapshot
                .panels
                .iter()
                .any(|panel| panel.error.is_some())
        {
            return None;
        }
        self.cost_cache = Some(CostCache {
            fetched_at,
            snapshot: self.snapshot.clone(),
        });
        self.cost_cache.as_ref()
    }
}
