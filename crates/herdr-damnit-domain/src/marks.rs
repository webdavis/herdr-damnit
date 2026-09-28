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
mod tests {
    use super::*;

    fn priority(value: u8) -> Priority {
        Priority::new(value).expect("a priority")
    }

    #[test]
    fn priority_one_is_the_urgent_one_and_four_carries_no_mark() {
        assert_eq!(
            Mark::of_priority(priority(1)).map(Mark::slot),
            Some(Slot::Red)
        );
        assert_eq!(
            Mark::of_priority(priority(2)).map(Mark::slot),
            Some(Slot::Orange)
        );
        assert_eq!(
            Mark::of_priority(priority(3)).map(Mark::slot),
            Some(Slot::Blue)
        );
        assert_eq!(Mark::of_priority(priority(4)), None);
    }

    #[test]
    fn the_plain_set_tells_the_three_priorities_apart_by_character() {
        let plain = |value: u8| {
            Mark::of_priority(priority(value))
                .expect("a mark")
                .glyph(IconSet::Ascii)
        };
        assert_eq!(plain(1), "!");
        assert_eq!(plain(2), "^");
        assert_eq!(plain(3), "-");
    }

    #[test]
    fn the_four_staging_marks_have_their_own_characters_and_colours() {
        for (mark, glyph, slot) in [
            (Mark::Working, "*", Slot::Yellow),
            (Mark::Staged, "+", Slot::Green),
            (Mark::Unpushed, "^", Slot::Cyan),
            (Mark::Conflict, "x", Slot::Red),
        ] {
            assert_eq!(mark.glyph(IconSet::Ascii), glyph);
            assert_eq!(mark.slot(), slot);
        }
    }

    #[test]
    fn a_notice_is_a_decision_waiting_so_it_draws_below_red_and_the_same_bang_in_either_set() {
        assert_eq!(Mark::Notice.glyph(IconSet::Ascii), "!");
        assert_eq!(Mark::Notice.glyph(IconSet::NerdFont), "!");
        assert_eq!(Mark::Notice.slot(), Slot::Orange);
    }

    #[test]
    fn a_label_count_is_the_sigil_and_the_number() {
        assert_eq!(Mark::Labels(2).glyph(IconSet::Ascii), "@2");
        assert_eq!(Mark::Labels(0).glyph(IconSet::Ascii), "@0");
        assert_eq!(Mark::Labels(2).glyph(IconSet::NerdFont), "\u{f02c}2");
    }

    #[test]
    fn every_nerd_font_mark_draws_the_font_awesome_codepoint_the_spec_names() {
        for (mark, glyph) in [
            (Mark::Priority(priority(1)), "\u{f024}"),
            (Mark::Overdue, "\u{f071}"),
            (Mark::Today, "\u{f017}"),
            (Mark::Upcoming, "\u{f073}"),
            (Mark::Recurring, "\u{f021}"),
            (Mark::Working, "\u{f040}"),
            (Mark::Staged, "\u{f067}"),
            (Mark::Unpushed, "\u{f062}"),
            (Mark::Conflict, "\u{f00d}"),
        ] {
            assert_eq!(mark.glyph(IconSet::NerdFont), glyph, "{mark:?}");
        }
    }

    #[test]
    fn a_due_state_takes_its_own_mark_except_when_there_is_no_date() {
        assert_eq!(Mark::of_due(DueState::Overdue), Some(Mark::Overdue));
        assert_eq!(Mark::of_due(DueState::Today), Some(Mark::Today));
        assert_eq!(Mark::of_due(DueState::Upcoming), Some(Mark::Upcoming));
        assert_eq!(Mark::of_due(DueState::None), None);
    }

    #[test]
    fn every_nerd_font_glyph_is_one_character_so_a_row_is_measured_by_counting_marks() {
        for mark in [
            Mark::Priority(priority(1)),
            Mark::Overdue,
            Mark::Today,
            Mark::Upcoming,
            Mark::Recurring,
            Mark::Working,
            Mark::Staged,
            Mark::Unpushed,
            Mark::Conflict,
            Mark::Notice,
        ] {
            let glyph = mark.glyph(IconSet::NerdFont);
            assert_eq!(glyph.chars().count(), 1, "{mark:?} drew {glyph:?}");
        }
    }
}
