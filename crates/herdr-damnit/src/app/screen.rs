//! The screens the Tab key cycles, and the order it walks them in.

/// The three screens the Tab key cycles. Detail is not one of them: it is about one object rather
/// than about a set of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    List,
    Status,
    Done,
}

/// The cycle, in the order Tab walks it.
const CYCLE: [Screen; 3] = [Screen::List, Screen::Status, Screen::Done];

impl Screen {
    /// What this screen calls itself in the status line. The List screen is named by the view it
    /// is showing, which the caller supplies.
    pub(super) fn label(self) -> Option<&'static str> {
        match self {
            Self::List => None,
            Self::Status => "status".into(),
            Self::Done => "done".into(),
        }
    }

    fn step(self, by: isize) -> Self {
        let at = CYCLE.iter().position(|screen| *screen == self).unwrap_or(0) as isize;
        CYCLE[(at + by).rem_euclid(CYCLE.len() as isize) as usize]
    }

    pub(super) fn next(self) -> Self {
        self.step(1)
    }

    pub(super) fn previous(self) -> Self {
        self.step(-1)
    }
}
