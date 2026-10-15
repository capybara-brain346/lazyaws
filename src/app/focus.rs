use crate::model::PANEL_COUNT;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Status,
    List(usize),
    Detail,
    Log,
}

impl Focus {
    pub fn list(self) -> Option<usize> {
        match self {
            Self::List(panel) => Some(panel),
            _ => None,
        }
    }

    // Tab order is Status followed by each list; Detail and Log continue from their list.
    pub(super) fn cycle(self, delta: isize, detail_panel: usize) -> Self {
        let position = match self {
            Self::Status | Self::Log => 0,
            Self::List(panel) => panel + 1,
            Self::Detail => detail_panel + 1,
        };
        let stops = PANEL_COUNT as isize + 1;
        match (position as isize + delta).rem_euclid(stops) as usize {
            0 => Self::Status,
            next => Self::List(next - 1),
        }
    }
}
