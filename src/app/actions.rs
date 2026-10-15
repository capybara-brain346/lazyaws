use crate::{
    model::PANEL_COUNT,
    operations::{Operation, available},
};

use super::{App, input::Action};

#[derive(Clone, Debug)]
pub struct ActionDialog {
    pub operations: Vec<Operation>,
    pub selected: usize,
    pub confirming: bool,
    pub editing: bool,
    pub input: String,
    pub error: Option<String>,
    pub preview: Result<Vec<String>, String>,
}

impl ActionDialog {
    pub fn new(operations: Vec<Operation>) -> Self {
        Self {
            operations,
            selected: 0,
            confirming: false,
            editing: false,
            input: String::new(),
            error: None,
            preview: Ok(Vec::new()),
        }
    }

    // The dialog draws on every frame; arguments can touch the filesystem, so build them on change.
    fn refresh_preview(&mut self) {
        self.preview = self
            .operations
            .get(self.selected)
            .ok_or_else(|| "No action is selected".to_string())
            .and_then(|operation| operation.args(&self.input));
    }
}

impl App {
    pub fn open_actions(&mut self) {
        let Some(panel) = self.action_panel() else {
            return;
        };
        self.open_actions_for(panel, self.selected[panel]);
    }

    pub fn choose_action(&mut self, index: usize) -> Action {
        let Some(dialog) = self.actions.as_mut() else {
            return Action::None;
        };
        if dialog.operations.is_empty() {
            return Action::None;
        }
        let Some(operation) = dialog.operations.get(index) else {
            return Action::None;
        };

        dialog.selected = index;
        dialog.error = None;
        dialog.input = operation.initial_input();
        dialog.editing = operation.parameter.is_some();
        dialog.confirming = !dialog.editing;
        dialog.refresh_preview();
        Action::None
    }

    pub fn confirm_action(&mut self) -> Action {
        let Some(dialog) = self.actions.as_mut() else {
            return Action::None;
        };
        if !dialog.confirming || dialog.editing {
            return Action::None;
        }

        let Some(operation) = dialog.operations.get(dialog.selected) else {
            return Action::None;
        };
        if let Err(error) = operation.args(&dialog.input) {
            dialog.error = Some(error);
            return Action::None;
        }

        let action = Action::Execute(operation.clone(), dialog.input.clone());
        self.executing = true;
        action
    }

    pub(super) fn accept_action_input(&mut self) {
        let Some(dialog) = self.actions.as_mut() else {
            return;
        };
        if !dialog.editing {
            return;
        }

        let Some(operation) = dialog.operations.get(dialog.selected) else {
            return;
        };
        match operation.args(&dialog.input) {
            Ok(_) => {
                dialog.editing = false;
                dialog.confirming = true;
                dialog.error = None;
            }
            Err(error) => dialog.error = Some(error),
        }
    }

    pub(super) fn open_actions_for(&mut self, panel: usize, index: usize) {
        if self.filtering
            || self.executing
            || self.actions.is_some()
            || !matches!(self.account.as_ref(), Some(Ok(_)))
            || panel >= PANEL_COUNT
            || self.selected[panel] != index
            || !self.visible_indices(panel).contains(&index)
            || self.snapshot.panels[panel].error.is_some()
            || self.snapshot.panels[0].error.is_some()
        {
            return;
        }

        let Some(resource) = self.snapshot.panels[panel].items.get(index) else {
            return;
        };
        let Some(primary) = self.snapshot.panels[0].items.get(self.selected[0]) else {
            return;
        };
        let operations = available(self.service, panel, resource, primary, &self.snapshot);
        if operations.is_empty() {
            return;
        }

        self.actions = Some(ActionDialog::new(operations));
    }

    pub(super) fn handle_action_key(&mut self, key: crossterm::event::KeyEvent) -> Action {
        use crossterm::event::{KeyCode, KeyModifiers};

        if self.executing {
            return Action::None;
        }

        if key.code == KeyCode::Esc {
            self.actions = None;
            return Action::None;
        }

        let Some(dialog) = self.actions.as_mut() else {
            return Action::None;
        };
        if dialog.operations.is_empty() {
            return Action::None;
        }

        if dialog.editing {
            match key.code {
                KeyCode::Enter => self.accept_action_input(),
                KeyCode::Backspace => {
                    dialog.input.pop();
                    dialog.error = None;
                    dialog.refresh_preview();
                }
                KeyCode::Char(character)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    dialog.input.push(character);
                    dialog.error = None;
                    dialog.refresh_preview();
                }
                _ => {}
            }
        } else if dialog.confirming {
            match key.code {
                KeyCode::Char('y' | 'Y') => return self.confirm_action(),
                KeyCode::Char('n' | 'N') => self.actions = None,
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Enter => {
                    let selected = dialog.selected;
                    return self.choose_action(selected);
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    dialog.selected = (dialog.selected + 1) % dialog.operations.len();
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    dialog.selected = dialog
                        .selected
                        .checked_sub(1)
                        .unwrap_or(dialog.operations.len() - 1);
                }
                _ => {}
            }
        }

        Action::None
    }
}
