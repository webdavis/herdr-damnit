//! Where a date sits against today, and the two shapes the pane draws one in. The day is handed
//! in rather than read from the clock, which is what keeps a test from depending on when it runs.

pub use jiff::civil::Date;

/// Where a task's due date sits against today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DueState {
    Overdue,
    Today,
    Upcoming,
    None,
}

pub fn due_state(due: Option<Date>, today: Date) -> DueState {
    match due {
        None => DueState::None,
        Some(date) if date < today => DueState::Overdue,
        Some(date) if date == today => DueState::Today,
        Some(_) => DueState::Upcoming,
    }
}

/// A `dam` date or timestamp read down to the civil day it falls on.
pub fn parse_date(text: &str) -> Option<Date> {
    if let Ok(date) = text.parse::<Date>() {
        return Some(date);
    }
    if let Ok(stamp) = text.parse::<jiff::Timestamp>() {
        return Some(stamp.to_zoned(jiff::tz::TimeZone::UTC).date());
    }
    text.parse::<jiff::civil::DateTime>()
        .ok()
        .map(|moment| moment.date())
}

/// The `MM-DD` a row draws beside a due mark.
pub fn short(date: Date) -> String {
    format!("{:02}-{:02}", date.month(), date.day())
}

/// The `YYYY-MM-DD` the Detail screen and the Done screen draw.
pub fn long(date: Date) -> String {
    format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(text: &str) -> Date {
        parse_date(text).expect("a date")
    }

    #[test]
    fn a_due_date_sits_before_on_or_after_today() {
        let today = day("2026-09-20");
        assert_eq!(due_state(Some(day("2026-09-18")), today), DueState::Overdue);
        assert_eq!(due_state(Some(day("2026-09-20")), today), DueState::Today);
        assert_eq!(
            due_state(Some(day("2026-10-02")), today),
            DueState::Upcoming
        );
        assert_eq!(due_state(None, today), DueState::None);
    }

    #[test]
    fn a_timestamp_is_read_down_to_the_day_it_falls_on() {
        assert_eq!(parse_date("2026-09-20T14:30"), Some(day("2026-09-20")));
        assert_eq!(parse_date("2026-09-20T14:30:00Z"), Some(day("2026-09-20")));
        assert_eq!(
            parse_date("2026-09-20T14:30:00-07:00"),
            Some(day("2026-09-20"))
        );
    }

    #[test]
    fn something_that_is_not_a_date_is_no_date() {
        assert_eq!(parse_date(""), None);
        assert_eq!(parse_date("tomorrow"), None);
        assert_eq!(parse_date("2026-13-40"), None);
    }

    #[test]
    fn a_row_draws_the_month_and_the_day_and_the_detail_draws_the_year_too() {
        assert_eq!(short(day("2026-09-18")), "09-18");
        assert_eq!(long(day("2026-09-18")), "2026-09-18");
    }
}
