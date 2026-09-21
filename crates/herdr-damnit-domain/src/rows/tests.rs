use super::*;
use crate::{Kind, Object, Priority, TaskFields, parse_date};

struct NoMarks;

impl StagingMarks for NoMarks {
    fn mark_of(&self, _oid: &Oid) -> Option<Mark> {
        None
    }
}

struct OneMark(Oid, Mark);

impl StagingMarks for OneMark {
    fn mark_of(&self, oid: &Oid) -> Option<Mark> {
        (oid == &self.0).then_some(self.1)
    }
}

fn task(oid: &str, path: &str, subject: &str) -> Object {
    Object {
        oid: Oid::new(oid),
        kind: Kind::Task,
        subject: subject.to_string(),
        body: String::new(),
        path: path.to_string(),
        labels: Vec::new(),
        depends: Vec::new(),
        recurrence: None,
        task: Some(TaskFields {
            done: false,
            priority: Priority::default(),
            due: None,
            deadline: None,
            attached: None,
        }),
        event: None,
    }
}

fn style() -> RowStyle {
    RowStyle {
        icons: IconSet::Ascii,
        today: parse_date("2026-09-20").expect("a date"),
    }
}

fn drawn(rows: &[Row]) -> Vec<String> {
    rows.iter().map(Row::text).collect()
}

#[test]
fn every_path_gets_one_heading_and_its_objects_sit_under_it() {
    let objects = vec![
        task("2", "proj/home", "water the plants"),
        task("1", "proj/dotfiles", "ship the pin bump"),
        task("3", "proj/dotfiles", "refresh the roster row"),
        task("4", "proj/dotfiles/", "bump the roster pin"),
    ];

    assert_eq!(
        drawn(&rows(&objects, &NoMarks, style())),
        vec![
            "proj/dotfiles".to_string(),
            "  bump the roster pin".to_string(),
            "  refresh the roster row".to_string(),
            "  ship the pin bump".to_string(),
            "proj/home".to_string(),
            "  water the plants".to_string(),
        ]
    );
}

#[test]
fn an_object_with_no_path_is_grouped_rather_than_dropped() {
    let rows = rows(&[task("1", "", "file taxes")], &NoMarks, style());
    assert_eq!(
        drawn(&rows),
        vec!["(no path)".to_string(), "  file taxes".to_string()]
    );
}

#[test]
fn a_path_that_is_nothing_but_separators_is_no_path_either() {
    let rows = rows(&[task("1", "/", "file taxes")], &NoMarks, style());
    assert_eq!(rows[0].text(), "(no path)");
}

#[test]
fn the_marks_lead_the_line_in_one_order() {
    let mut object = task("1", "home", "pay the rent");
    object.labels = vec!["home".to_string(), "slow".to_string()];
    object.recurrence = Some("every month".to_string());
    if let Some(fields) = object.task.as_mut() {
        fields.priority = Priority::new(1).expect("a priority");
        fields.due = parse_date("2026-09-18");
    }

    let rows = rows(&[object], &OneMark(Oid::new("1"), Mark::Staged), style());

    assert_eq!(rows[1].text(), "  + ! < 09-18 ~ pay the rent @2");
}

/// The spec's List mock draws an object whose commits have not reached the remote with the up
/// arrow leading, which is the staging column `mark_of` fills.
#[test]
fn an_unpushed_object_draws_the_up_arrow_in_the_staging_column() {
    let object = task("1", "home", "refresh the roster row");

    let rows = rows(&[object], &OneMark(Oid::new("1"), Mark::Unpushed), style());

    assert_eq!(rows[1].text(), "  ^ refresh the roster row");
}

#[test]
fn a_date_due_today_carries_its_mark_and_no_date_beside_it() {
    let mut object = task("1", "home", "call the bank");
    if let Some(fields) = object.task.as_mut() {
        fields.due = parse_date("2026-09-20");
    }

    let rows = rows(&[object], &NoMarks, style());
    assert_eq!(rows[1].text(), "  * call the bank");
}

#[test]
fn a_mark_is_painted_by_what_it_means_and_the_subject_is_plain_text() {
    let mut object = task("1", "home", "call the bank");
    if let Some(fields) = object.task.as_mut() {
        fields.priority = Priority::new(1).expect("a priority");
    }

    let rows = rows(&[object], &NoMarks, style());
    let Row::Object(row) = &rows[1] else {
        panic!("expected an object row");
    };
    assert_eq!(row.segments[1].slot, Slot::Red);
    assert_eq!(row.segments.last().expect("a segment").slot, Slot::Text);
}

#[test]
fn a_heading_names_no_oid_and_an_object_row_does() {
    let rows = rows(&[task("1a2b3c4", "home", "x")], &NoMarks, style());
    assert_eq!(rows[0].oid(), None);
    assert_eq!(rows[1].oid(), Some(&Oid::new("1a2b3c4")));
}
