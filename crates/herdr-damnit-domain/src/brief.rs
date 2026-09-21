//! The text `S` hands to the workspace's agent pane. Plain text, because an agent pane is a shell:
//! it is pasted into that pane's input and the operator presses return themselves.

use crate::{Kind, Object, long};

pub fn brief(object: &Object, note: &str) -> String {
    let kind = match object.kind {
        Kind::Task => "task",
        Kind::Event => "event",
    };
    let mut lines = vec![
        format!("dam {kind}: {}", object.subject),
        format!("oid: {}", object.oid),
    ];
    if !object.path.is_empty() {
        lines.push(format!("path: {}", object.path));
    }
    if let Some(due) = object.due() {
        lines.push(format!("due: {}", long(due)));
    }
    if let Some(deadline) = object.task.as_ref().and_then(|task| task.deadline) {
        lines.push(format!("deadline: {}", long(deadline)));
    }
    if !object.priority().is_lowest() {
        lines.push(format!("priority: p{}", object.priority().get()));
    }
    if !object.labels.is_empty() {
        lines.push(format!("labels: {}", object.labels.join(", ")));
    }
    let mut text = lines.join("\n");
    if !object.body.trim().is_empty() {
        text.push_str("\n\n");
        text.push_str(object.body.trim_end());
    }
    if !note.trim().is_empty() {
        text.push_str("\n\nnote: ");
        text.push_str(note.trim());
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Kind, Oid, Priority, TaskFields, parse_date};

    fn task() -> Object {
        Object {
            oid: Oid::new("1a2b3c4d"),
            kind: Kind::Task,
            subject: "file taxes".to_string(),
            body: "receipts are in the drawer".to_string(),
            path: "home/admin/".to_string(),
            labels: vec!["home".to_string(), "slow".to_string()],
            depends: Vec::new(),
            recurrence: None,
            task: Some(TaskFields {
                done: false,
                priority: Priority::new(1).expect("a priority"),
                due: parse_date("2026-09-20"),
                deadline: None,
                attached: None,
            }),
            event: None,
        }
    }

    #[test]
    fn the_brief_names_the_task_by_its_oid_and_carries_the_note_last() {
        assert_eq!(
            brief(&task(), "start with the receipts"),
            "dam task: file taxes\n\
             oid: 1a2b3c4d\n\
             path: home/admin/\n\
             due: 2026-09-20\n\
             priority: p1\n\
             labels: home, slow\n\
             \n\
             receipts are in the drawer\n\
             \n\
             note: start with the receipts"
        );
    }

    #[test]
    fn a_field_the_object_has_nothing_for_is_left_out_rather_than_written_empty() {
        let bare = Object {
            body: String::new(),
            path: String::new(),
            labels: Vec::new(),
            task: Some(TaskFields {
                done: false,
                priority: Priority::default(),
                due: None,
                deadline: None,
                attached: None,
            }),
            ..task()
        };

        assert_eq!(brief(&bare, ""), "dam task: file taxes\noid: 1a2b3c4d");
    }

    #[test]
    fn an_event_says_so_in_its_first_line() {
        let event = Object {
            kind: Kind::Event,
            task: None,
            body: String::new(),
            path: String::new(),
            labels: Vec::new(),
            ..task()
        };

        assert!(brief(&event, "").starts_with("dam event: file taxes\n"));
    }

    #[test]
    fn a_deadline_sits_under_the_due_date_when_the_task_carries_one() {
        let dated = Object {
            task: Some(TaskFields {
                deadline: parse_date("2026-10-01"),
                ..task().task.expect("a task")
            }),
            ..task()
        };

        assert!(
            brief(&dated, "").contains("due: 2026-09-20\ndeadline: 2026-10-01\n"),
            "{}",
            brief(&dated, "")
        );
    }

    #[test]
    fn a_blank_note_leaves_no_note_line_behind() {
        assert!(!brief(&task(), "   ").contains("note:"));
    }
}
