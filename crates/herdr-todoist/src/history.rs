//! The completed-task walk: which request reads the next page, what the pane has collected so
//! far, and where the walk stops. Pure state over the pages handed to it, so the whole paging
//! boundary is testable without a terminal or a network.

use todoist::{CompletedPage, CompletedTask};

use crate::list::{Row, Segment, TaskRow};
use crate::theme::Slot;

/// The API reads completed tasks in a window of at most three months, so the walk steps back one
/// window at a time. Ninety days is inside that cap for any three consecutive months.
const WINDOW_DAYS: i64 = 90;

/// How deep the walk goes. The endpoint has no floor of its own: it answers one window at a time,
/// and the account's own retention decides how much of a window holds anything. So the pane reads
/// a stated depth and says where it stopped, rather than asking for windows without end.
const WINDOWS: i64 = 12;

const SECONDS_PER_DAY: u64 = 86_400;

/// One request the walk wants: a window, and a page within it.
#[derive(Debug, PartialEq, Eq)]
pub struct Request {
    pub since: String,
    pub until: String,
    pub cursor: Option<String>,
}

/// The completed history read so far, newest first, and the position of the walk that read it.
/// Days are day numbers since the Unix epoch; `until` is exclusive, the way the endpoint reads it.
pub struct History {
    collected: Vec<CompletedTask>,
    until: i64,
    since: i64,
    floor: i64,
    cursor: Option<String>,
    spent: bool,
}

impl History {
    /// A walk starting at the newest window, `today` being a day number since the Unix epoch.
    pub fn new(today: i64) -> Self {
        let until = today + 1;
        Self {
            collected: Vec::new(),
            until,
            since: until - WINDOW_DAYS,
            floor: until - WINDOW_DAYS * WINDOWS,
            cursor: None,
            spent: false,
        }
    }

    pub fn today() -> Self {
        Self::new(today())
    }

    /// The request that reads the next page, or `None` once the walk has reached its floor.
    pub fn request(&self) -> Option<Request> {
        (!self.spent).then(|| Request {
            since: timestamp(self.since),
            until: timestamp(self.until),
            cursor: self.cursor.clone(),
        })
    }

    /// Take a page: keep its tasks, and move the walk to whatever reads the page after it.
    pub fn accept(&mut self, page: CompletedPage) {
        self.collected.extend(page.tasks);
        // The whole collection is ordered rather than appended to: a later window is older than
        // the one before it, but the endpoint promises no order inside one.
        self.collected.sort_by(|left, right| {
            right
                .completed_at
                .as_deref()
                .cmp(&left.completed_at.as_deref())
                .then(right.id.cmp(&left.id))
        });
        match page.next_cursor {
            Some(cursor) if Some(&cursor) != self.cursor.as_ref() => self.cursor = Some(cursor),
            _ => self.step_back(),
        }
    }

    /// The window before the one just read, or the end of the walk.
    fn step_back(&mut self) {
        self.cursor = None;
        self.until = self.since;
        self.since = (self.until - WINDOW_DAYS).max(self.floor);
        self.spent = self.until <= self.floor;
    }

    /// Whether the walk has read as deep as it goes.
    pub fn spent(&self) -> bool {
        self.spent
    }

    pub fn len(&self) -> usize {
        self.collected.len()
    }

    /// The oldest day the walk reads, which is what the bottom of the list reports.
    pub fn floor(&self) -> String {
        date(self.floor)
    }

    /// Drop a task from the history, which is what a reopened task does: it is no longer
    /// completed, so it leaves the completed list at once.
    pub fn forget(&mut self, id: &str) {
        self.collected.retain(|task| task.id != id);
    }

    /// One row per completed task, newest first. The list is flat: a completed task is read by
    /// when it was finished rather than by where it was filed.
    pub fn rows(&self) -> Vec<Row> {
        self.collected
            .iter()
            .map(|task| {
                Row::Task(TaskRow {
                    id: task.id.clone(),
                    text: line(task),
                    content: task.content.clone(),
                    // The completed endpoint sends none of these, so the completed screen's keys
                    // are the ones that need none of them.
                    description: String::new(),
                    priority: crate::list::LOWEST_PRIORITY,
                    labels: Vec::new(),
                    due: None,
                    waiting: false,
                })
            })
            .collect()
    }
}

/// A task's line: the completion date first, so the dates line up down the pane.
fn line(task: &CompletedTask) -> Vec<Segment> {
    let completed_at = task.completed_at.as_deref().unwrap_or("");
    let day = completed_at.get(..10).unwrap_or(completed_at);
    vec![
        Segment::new(format!("{day:10}  "), Slot::Dim1),
        Segment::new(task.content.clone(), Slot::Text),
    ]
}

/// Today as a day number since the Unix epoch. A clock before the epoch reads as the epoch, which
/// only a misconfigured machine can produce and which still leaves the walk a valid window.
fn today() -> i64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(elapsed) => (elapsed.as_secs() / SECONDS_PER_DAY) as i64,
        Err(_) => 0,
    }
}

/// A day number as the ISO 8601 timestamp the endpoint reads, at the start of that day in UTC.
fn timestamp(day: i64) -> String {
    format!("{}T00:00:00Z", date(day))
}

fn date(day: i64) -> String {
    let (year, month, of_month) = civil(day);
    format!("{year:04}-{month:02}-{of_month:02}")
}

/// A day number since the Unix epoch as a civil year, month and day, by Howard Hinnant's
/// `civil_from_days`, which is exact over the whole proleptic Gregorian calendar.
fn civil(day: i64) -> (i64, i64, i64) {
    let shifted = day + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let of_era = shifted - era * 146_097;
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * of_year + 2) / 153;
    let of_month = of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    (if month <= 2 { year + 1 } else { year }, month, of_month)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-09-18, the day these windows are written against.
    const TODAY: i64 = 20_714;

    fn completed(json: &str) -> CompletedTask {
        serde_json::from_str(json).expect("completed task")
    }

    fn page(tasks: Vec<CompletedTask>, next_cursor: Option<&str>) -> CompletedPage {
        CompletedPage {
            tasks,
            next_cursor: next_cursor.map(str::to_string),
        }
    }

    fn texts(history: &History) -> Vec<String> {
        history
            .rows()
            .iter()
            .map(|row| row.text().to_string())
            .collect()
    }

    #[test]
    fn a_day_number_becomes_the_date_it_names() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(TODAY), "2026-09-18");
        assert_eq!(date(11_016), "2000-02-29", "a leap day");
        assert_eq!(timestamp(TODAY), "2026-09-18T00:00:00Z");
    }

    #[test]
    fn the_first_window_ends_after_today_and_reaches_back_three_months() {
        let history = History::new(TODAY);

        assert_eq!(
            history.request(),
            Some(Request {
                since: "2026-06-21T00:00:00Z".to_string(),
                until: "2026-09-19T00:00:00Z".to_string(),
                cursor: None,
            }),
            "today's completions have to be inside the first window"
        );
    }

    #[test]
    fn a_page_with_a_cursor_asks_for_the_same_window_again() {
        let mut history = History::new(TODAY);

        history.accept(page(Vec::new(), Some("second-page")));

        let request = history.request().expect("another page");
        assert_eq!(request.cursor.as_deref(), Some("second-page"));
        assert_eq!(request.since, "2026-06-21T00:00:00Z");
    }

    #[test]
    fn the_last_page_of_a_window_steps_back_to_the_window_before_it() {
        let mut history = History::new(TODAY);

        history.accept(page(Vec::new(), None));

        assert_eq!(
            history.request(),
            Some(Request {
                since: "2026-03-23T00:00:00Z".to_string(),
                until: "2026-06-21T00:00:00Z".to_string(),
                cursor: None,
            })
        );
    }

    #[test]
    fn a_repeated_cursor_steps_back_instead_of_reading_one_window_forever() {
        let mut history = History::new(TODAY);
        history.accept(page(Vec::new(), Some("stuck")));

        history.accept(page(Vec::new(), Some("stuck")));

        assert_eq!(
            history.request().expect("a window").until,
            "2026-06-21T00:00:00Z"
        );
    }

    #[test]
    fn the_walk_stops_at_its_floor_and_names_the_day_it_read_back_to() {
        let mut history = History::new(TODAY);

        for _ in 0..WINDOWS {
            assert!(!history.spent(), "the walk ended early");
            history.accept(page(Vec::new(), None));
        }

        assert!(history.spent());
        assert_eq!(history.request(), None);
        assert_eq!(history.floor(), "2023-10-05");
    }

    #[test]
    fn tasks_are_newest_first_across_pages_and_within_one_day() {
        let mut history = History::new(TODAY);

        history.accept(page(
            vec![
                completed(r#"{"id":"1","content":"older","completed_at":"2026-09-10T08:00:00Z"}"#),
                completed(r#"{"id":"2","content":"newest","completed_at":"2026-09-17T21:30:00Z"}"#),
            ],
            Some("second-page"),
        ));
        history.accept(page(
            vec![completed(
                r#"{"id":"3","content":"same day, earlier","completed_at":"2026-09-17T06:05:00Z"}"#,
            )],
            None,
        ));

        assert_eq!(
            texts(&history),
            vec![
                "2026-09-17  newest",
                "2026-09-17  same day, earlier",
                "2026-09-10  older",
            ]
        );
    }

    #[test]
    fn a_task_with_no_completion_time_keeps_the_dates_in_their_column() {
        let mut history = History::new(TODAY);

        history.accept(page(
            vec![
                completed(r#"{"id":"1","content":"dated","completed_at":"2026-09-17T06:05:00Z"}"#),
                completed(r#"{"id":"2","content":"undated","completed_at":null}"#),
            ],
            None,
        ));

        assert_eq!(
            texts(&history),
            vec!["2026-09-17  dated", "            undated"]
        );
    }

    #[test]
    fn a_reopened_task_leaves_the_history_at_once() {
        let mut history = History::new(TODAY);
        history.accept(page(
            vec![
                completed(r#"{"id":"1","content":"first","completed_at":"2026-09-17T06:00:00Z"}"#),
                completed(r#"{"id":"2","content":"second","completed_at":"2026-09-16T06:00:00Z"}"#),
            ],
            None,
        ));

        history.forget("1");

        assert_eq!(texts(&history), vec!["2026-09-16  second"]);
        assert_eq!(history.len(), 1);
    }
}
