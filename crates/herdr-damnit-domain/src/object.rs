//! One object as `dam` describes it. The field names and the optionality are `dam`'s own wire
//! shape; what the pane adds is the three questions every row asks whatever the kind.

use crate::{Date, Oid, Priority};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Task,
    Event,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Object {
    pub oid: Oid,
    pub kind: Kind,
    pub subject: String,
    pub body: String,
    pub path: String,
    pub labels: Vec<String>,
    pub depends: Vec<Oid>,
    pub recurrence: Option<String>,
    pub task: Option<TaskFields>,
    pub event: Option<EventFields>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskFields {
    pub done: bool,
    pub priority: Priority,
    pub due: Option<Date>,
    pub deadline: Option<Date>,
    /// The oid of the event this task is attached to.
    pub attached: Option<Oid>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventFields {
    pub start: String,
    pub end: String,
    pub timezone: Option<String>,
    pub location: Option<String>,
    pub status: String,
    pub transparency: String,
    pub attendees: Vec<Attendee>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attendee {
    pub email: String,
    pub response: String,
}

impl Object {
    pub fn is_done(&self) -> bool {
        self.task.as_ref().is_some_and(|task| task.done)
    }

    pub fn priority(&self) -> Priority {
        self.task
            .as_ref()
            .map_or_else(Priority::default, |task| task.priority)
    }

    pub fn due(&self) -> Option<Date> {
        self.task.as_ref().and_then(|task| task.due)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_date;

    fn task() -> Object {
        Object {
            oid: Oid::new("1a2b3c4"),
            kind: Kind::Task,
            subject: "ship the pin bump".to_string(),
            body: String::new(),
            path: "proj/dotfiles".to_string(),
            labels: vec!["slow".to_string()],
            depends: Vec::new(),
            recurrence: None,
            task: Some(TaskFields {
                done: false,
                priority: Priority::new(1).expect("a priority"),
                due: parse_date("2026-09-18"),
                deadline: None,
                attached: None,
            }),
            event: None,
        }
    }

    #[test]
    fn a_task_answers_for_its_own_fields() {
        let object = task();
        assert!(!object.is_done());
        assert_eq!(object.priority().get(), 1);
        assert_eq!(object.due(), parse_date("2026-09-18"));
    }

    #[test]
    fn an_event_answers_the_defaults_rather_than_forcing_a_match_on_kind() {
        let object = Object {
            kind: Kind::Event,
            task: None,
            event: Some(EventFields {
                start: "2026-09-20T09:00".to_string(),
                end: "2026-09-20T10:00".to_string(),
                timezone: None,
                location: None,
                status: "confirmed".to_string(),
                transparency: "busy".to_string(),
                attendees: Vec::new(),
            }),
            ..task()
        };
        assert!(!object.is_done());
        assert_eq!(object.priority(), Priority::default());
        assert_eq!(object.due(), None);
    }
}
