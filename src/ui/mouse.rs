use ratatui::layout::Rect;

use crate::model::Service;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseTarget {
    Service(Service),
    Pane(usize),
    Row {
        panel: usize,
        focus: usize,
        index: usize,
    },
    OpenActions,
    ChooseAction(usize),
    ConfirmAction,
    CancelAction,
    ChooseTheme(usize),
}

#[derive(Clone, Debug, Default)]
pub struct HitMap {
    pub regions: Vec<(Rect, MouseTarget)>,
}

impl HitMap {
    pub fn target_at(&self, x: u16, y: u16) -> Option<MouseTarget> {
        self.regions
            .iter()
            .rev()
            .find(|(area, _)| x >= area.x && y >= area.y && x < area.right() && y < area.bottom())
            .map(|(_, target)| *target)
    }
}
