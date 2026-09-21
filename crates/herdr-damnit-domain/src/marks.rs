//! The marks a row carries: the object's own state, and the staging state `dam status` reports for
//! it. Two sets draw them. The Nerd Font set uses glyphs from the Font Awesome block every Nerd
//! Font patches in, each one cell wide; the plain set uses one character per mark, for a terminal
//! whose font has none of those glyphs.

use crate::{DueState, Priority, Slot};

/// Which set of marks the pane draws, chosen in the config file.
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
}

impl Mark {
    /// The mark a priority draws, or `None` for 4, which `dam` treats as no priority at all.
    pub fn of_priority(priority: Priority) -> Option<Self> {
        (!priority.is_lowest()).then_some(Self::Priority(priority))
    }

    /// The mark a due state draws, or `None` when the object has no date.
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
            IconSet::NerdFont => "\u{f02c}",
            IconSet::Ascii => "@",
        }
    }

    fn single(self, set: IconSet) -> &'static str {
        match (self, set) {
            (Self::Priority(_), IconSet::NerdFont) => "\u{f024}",
            (Self::Priority(priority), IconSet::Ascii) => match priority.get() {
                1 => "!",
                2 => "^",
                _ => "-",
            },
            (Self::Overdue, IconSet::NerdFont) => "\u{f071}",
            (Self::Overdue, IconSet::Ascii) => "<",
            (Self::Today, IconSet::NerdFont) => "\u{f017}",
            (Self::Today, IconSet::Ascii) => "*",
            (Self::Upcoming, IconSet::NerdFont) => "\u{f073}",
            (Self::Upcoming, IconSet::Ascii) => ">",
            (Self::Recurring, IconSet::NerdFont) => "\u{f021}",
            (Self::Recurring, IconSet::Ascii) => "~",
            (Self::Working, IconSet::NerdFont) => "\u{f040}",
            (Self::Working, IconSet::Ascii) => "*",
            (Self::Staged, IconSet::NerdFont) => "\u{f067}",
            (Self::Staged, IconSet::Ascii) => "+",
            (Self::Unpushed, IconSet::NerdFont) => "\u{f062}",
            (Self::Unpushed, IconSet::Ascii) => "^",
            (Self::Conflict, IconSet::NerdFont) => "\u{f00d}",
            (Self::Conflict, IconSet::Ascii) => "x",
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
    fn a_label_count_is_the_sigil_and_the_number() {
        assert_eq!(Mark::Labels(2).glyph(IconSet::Ascii), "@2");
        assert_eq!(Mark::Labels(0).glyph(IconSet::Ascii), "@0");
    }

    #[test]
    fn a_due_state_takes_its_own_mark_except_when_there_is_no_date() {
        assert_eq!(Mark::of_due(DueState::Overdue), Some(Mark::Overdue));
        assert_eq!(Mark::of_due(DueState::Today), Some(Mark::Today));
        assert_eq!(Mark::of_due(DueState::Upcoming), Some(Mark::Upcoming));
        assert_eq!(Mark::of_due(DueState::None), None);
    }

    /// Every Nerd Font glyph is one cell wide, which is what keeps the columns of the pane lined
    /// up; a glyph a font has none of is drawn as a two-cell replacement box and puts every column
    /// after it out by one.
    #[test]
    fn every_nerd_font_glyph_is_a_single_character() {
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
        ] {
            let glyph = mark.glyph(IconSet::NerdFont);
            assert_eq!(glyph.chars().count(), 1, "{mark:?} drew {glyph:?}");
        }
    }
}
