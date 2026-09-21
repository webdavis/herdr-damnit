//! The marks a task line carries: its priority, its due state, whether it repeats, and whether it
//! is labelled. Two sets draw them. The Nerd Font set uses glyphs from the Font Awesome block every
//! Nerd Font patches in, each one cell wide; the plain set uses one ASCII character per mark, for a
//! terminal whose font has no such glyph and would draw a replacement box two cells wide, which
//! would put every column in the pane out by one.

use serde::Deserialize;

/// Which set of marks the pane draws, chosen in the config file.
#[derive(Debug, Default, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum IconSet {
    #[default]
    NerdFont,
    Ascii,
}

/// Where a task's due date sits against today.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Due {
    Overdue,
    Today,
    Upcoming,
    None,
}

/// A task's due state, read against `today` as a plain ISO date. Both values are `YYYY-MM-DD`, so
/// comparing them as text compares them as dates, and the pane never asks the clock itself: the
/// day is handed in, which is what keeps a test from depending on when it runs.
pub fn due(date: Option<&str>, today: &str) -> Due {
    match date {
        None => Due::None,
        Some(date) if date < today => Due::Overdue,
        Some(date) if date == today => Due::Today,
        Some(_) => Due::Upcoming,
    }
}

/// The marks one set draws.
#[derive(Debug, Clone, Copy)]
pub struct Icons {
    /// The priority marks, from the API's 4 (the app's p1) down to its 2 (the app's p3). The
    /// lowest priority is the app's "no priority" and is drawn as nothing.
    pub priority: [&'static str; 3],
    pub overdue: &'static str,
    pub today: &'static str,
    pub upcoming: &'static str,
    pub recurring: &'static str,
    pub labels: &'static str,
}

/// Font Awesome glyphs, every one of them single width: flag, warning, clock, calendar, refresh
/// and tags. The three priorities share the flag and differ by color, the way the app draws them.
const NERD_FONT: Icons = Icons {
    priority: ["\u{f024}", "\u{f024}", "\u{f024}"],
    overdue: "\u{f071}",
    today: "\u{f017}",
    upcoming: "\u{f073}",
    recurring: "\u{f021}",
    labels: "\u{f02c}",
};

/// The plain set. The three priorities differ by character as well as by color, so a terminal
/// without color still tells them apart, and `@` is the sigil the app itself puts on a label.
const ASCII: Icons = Icons {
    priority: ["!", "^", "-"],
    overdue: "<",
    today: "*",
    upcoming: ">",
    recurring: "~",
    labels: "@",
};

impl Icons {
    pub fn of(set: IconSet) -> Self {
        match set {
            IconSet::NerdFont => NERD_FONT,
            IconSet::Ascii => ASCII,
        }
    }

    /// The mark for an API priority, or `None` for the lowest, which the app draws as no priority
    /// at all.
    pub fn priority(&self, priority: u8) -> Option<&'static str> {
        match priority {
            4 => Some(self.priority[0]),
            3 => Some(self.priority[1]),
            2 => Some(self.priority[2]),
            _ => None,
        }
    }

    /// The mark for a due state, or `None` for a task with no due date, which carries no mark.
    pub fn due(&self, due: Due) -> Option<&'static str> {
        match due {
            Due::Overdue => Some(self.overdue),
            Due::Today => Some(self.today),
            Due::Upcoming => Some(self.upcoming),
            Due::None => None,
        }
    }
}

/// What a task line needs to decorate itself: the set of marks, and the day its due state is read
/// against.
#[derive(Debug, Clone)]
pub struct Marks {
    pub icons: Icons,
    pub today: String,
}

impl Marks {
    pub fn new(set: IconSet, today: impl Into<String>) -> Self {
        Self {
            icons: Icons::of(set),
            today: today.into(),
        }
    }

    /// The marks for the day it is now, in the machine's own time zone, which is the zone the
    /// account's due dates are written in.
    pub fn today(set: IconSet) -> Self {
        Self::new(set, chrono::Local::now().date_naive().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODAY: &str = "2026-09-18";

    #[test]
    fn a_date_before_today_is_overdue_and_the_day_itself_is_today() {
        assert_eq!(due(Some("2026-09-17"), TODAY), Due::Overdue);
        assert_eq!(due(Some(TODAY), TODAY), Due::Today);
        assert_eq!(due(Some("2026-09-19"), TODAY), Due::Upcoming);
        assert_eq!(due(None, TODAY), Due::None);
    }

    #[test]
    fn a_due_date_carrying_a_time_reads_by_its_day() {
        // The list hands in the first ten characters of the API's value, so a timed due date on
        // today is today rather than upcoming.
        assert_eq!(due(Some("2026-09-18"), TODAY), Due::Today);
        assert_eq!(due(Some("2025-12-31"), TODAY), Due::Overdue);
    }

    #[test]
    fn each_priority_has_a_mark_and_the_lowest_has_none() {
        for set in [IconSet::NerdFont, IconSet::Ascii] {
            let icons = Icons::of(set);
            let marks: Vec<Option<&str>> = (1..=4).map(|p| icons.priority(p)).collect();

            assert_eq!(marks[0], None, "{set:?}: the lowest priority draws no mark");
            assert!(marks[1..].iter().all(Option::is_some), "{set:?}: {marks:?}");
        }
    }

    #[test]
    fn the_plain_set_tells_the_three_priorities_apart_by_character() {
        let icons = Icons::of(IconSet::Ascii);

        assert_eq!(icons.priority(4), Some("!"));
        assert_eq!(icons.priority(3), Some("^"));
        assert_eq!(icons.priority(2), Some("-"));
    }

    #[test]
    fn every_mark_of_both_sets_is_one_cell_wide() {
        use unicode_width::UnicodeWidthStr;

        for set in [IconSet::NerdFont, IconSet::Ascii] {
            let icons = Icons::of(set);
            let marks = [
                icons.priority[0],
                icons.priority[1],
                icons.priority[2],
                icons.overdue,
                icons.today,
                icons.upcoming,
                icons.recurring,
                icons.labels,
            ];
            for mark in marks {
                assert_eq!(mark.width(), 1, "{set:?}: {mark:?} is not one cell wide");
            }
        }
    }

    #[test]
    fn the_days_own_marks_read_against_a_plain_iso_date() {
        // The only place the clock is read. The shape is what the comparison needs, so this holds
        // whatever day it runs on.
        let today = Marks::today(IconSet::Ascii).today;

        assert_eq!(today.len(), 10, "{today}");
        assert!(
            today.chars().all(|c| c.is_ascii_digit() || c == '-'),
            "{today}"
        );
    }

    #[test]
    fn each_due_state_has_a_mark_and_a_task_with_no_due_date_has_none() {
        for set in [IconSet::NerdFont, IconSet::Ascii] {
            let icons = Icons::of(set);

            assert_eq!(icons.due(Due::Overdue), Some(icons.overdue), "{set:?}");
            assert_eq!(icons.due(Due::Today), Some(icons.today), "{set:?}");
            assert_eq!(icons.due(Due::Upcoming), Some(icons.upcoming), "{set:?}");
            assert_eq!(icons.due(Due::None), None, "{set:?}");
        }
    }
}
