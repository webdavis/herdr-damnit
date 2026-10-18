use crate::{DueState, Priority, Slot};

const FONT_AWESOME_FLAG: &str = "\u{f024}";
const FONT_AWESOME_WARNING: &str = "\u{f071}";
const FONT_AWESOME_CLOCK: &str = "\u{f017}";
const FONT_AWESOME_CALENDAR: &str = "\u{f073}";
const FONT_AWESOME_REFRESH: &str = "\u{f021}";
const FONT_AWESOME_PENCIL: &str = "\u{f040}";
const FONT_AWESOME_PLUS: &str = "\u{f067}";
const FONT_AWESOME_ARROW_UP: &str = "\u{f062}";
const FONT_AWESOME_CROSS: &str = "\u{f00d}";
const FONT_AWESOME_TAGS: &str = "\u{f02c}";
const NOTICE_IN_EITHER_SET: &str = "!";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconSet {
    #[default]
    NerdFont,
    Ascii,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Priority(Priority),
    Overdue,
    Today,
    Upcoming,
    Recurring,
    Labels(usize),
    Working,
    Staged,
    Unpushed,
    Conflict,
    Notice,
}

impl Mark {
    pub fn of_priority(priority: Priority) -> Option<Self> {
        (!priority.is_lowest()).then_some(Self::Priority(priority))
    }

    pub fn of_due(state: DueState) -> Option<Self> {
        match state {
            DueState::Overdue => Some(Self::Overdue),
            DueState::Today => Some(Self::Today),
            DueState::Upcoming => Some(Self::Upcoming),
            DueState::None => None,
        }
    }

    pub fn slot(self) -> Slot {
        match self {
            Self::Priority(priority) => match priority.get() {
                1 => Slot::Red,
                2 => Slot::Orange,
                _ => Slot::Blue,
            },
            Self::Overdue | Self::Conflict => Slot::Red,
            Self::Today | Self::Working => Slot::Yellow,
            Self::Upcoming => Slot::Blue,
            Self::Recurring | Self::Staged => Slot::Green,
            Self::Labels(_) => Slot::Purple,
            Self::Unpushed => Slot::Cyan,
            Self::Notice => Slot::Orange,
        }
    }

    pub fn glyph(self, set: IconSet) -> String {
        match self {
            Self::Labels(count) => format!("{}{count}", Self::labels_sigil(set)),
            other => other.single(set).to_string(),
        }
    }

    fn labels_sigil(set: IconSet) -> &'static str {
        match set {
            IconSet::NerdFont => FONT_AWESOME_TAGS,
            IconSet::Ascii => "@",
        }
    }

    fn single(self, set: IconSet) -> &'static str {
        match (self, set) {
            (Self::Priority(_), IconSet::NerdFont) => FONT_AWESOME_FLAG,
            (Self::Priority(priority), IconSet::Ascii) => match priority.get() {
                1 => "!",
                2 => "^",
                _ => "-",
            },
            (Self::Overdue, IconSet::NerdFont) => FONT_AWESOME_WARNING,
            (Self::Overdue, IconSet::Ascii) => "<",
            (Self::Today, IconSet::NerdFont) => FONT_AWESOME_CLOCK,
            (Self::Today, IconSet::Ascii) => "*",
            (Self::Upcoming, IconSet::NerdFont) => FONT_AWESOME_CALENDAR,
            (Self::Upcoming, IconSet::Ascii) => ">",
            (Self::Recurring, IconSet::NerdFont) => FONT_AWESOME_REFRESH,
            (Self::Recurring, IconSet::Ascii) => "~",
            (Self::Working, IconSet::NerdFont) => FONT_AWESOME_PENCIL,
            (Self::Working, IconSet::Ascii) => "*",
            (Self::Staged, IconSet::NerdFont) => FONT_AWESOME_PLUS,
            (Self::Staged, IconSet::Ascii) => "+",
            (Self::Unpushed, IconSet::NerdFont) => FONT_AWESOME_ARROW_UP,
            (Self::Unpushed, IconSet::Ascii) => "^",
            (Self::Conflict, IconSet::NerdFont) => FONT_AWESOME_CROSS,
            (Self::Conflict, IconSet::Ascii) => "x",
            (Self::Notice, _) => NOTICE_IN_EITHER_SET,
            (Self::Labels(_), _) => Self::labels_sigil(set),
        }
    }
}

#[cfg(test)]
mod tests;
