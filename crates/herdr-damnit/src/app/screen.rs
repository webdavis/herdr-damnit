#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    List,
    Status,
    Done,
    Detail,
}

const TAB_ORDER: [Screen; 3] = [Screen::List, Screen::Status, Screen::Done];

impl Screen {
    pub(super) fn label(self) -> Option<&'static str> {
        match self {
            Self::List => None,
            Self::Status => "status".into(),
            Self::Done => "done".into(),
            Self::Detail => "detail".into(),
        }
    }

    fn step(self, by: isize) -> Self {
        let at = TAB_ORDER
            .iter()
            .position(|screen| *screen == self)
            .unwrap_or(0) as isize;
        TAB_ORDER[(at + by).rem_euclid(TAB_ORDER.len() as isize) as usize]
    }

    pub(super) fn next(self) -> Self {
        self.step(1)
    }

    pub(super) fn previous(self) -> Self {
        self.step(-1)
    }
}
