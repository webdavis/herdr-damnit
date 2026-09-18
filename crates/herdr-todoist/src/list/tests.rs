use super::*;
use crate::icons::IconSet;
use crate::theme::Slot;

/// The day every one of these tests reads its due states against, so none of them depends on when
/// it runs.
pub(crate) const TODAY: &str = "2026-09-18";

/// The Nerd Font marks, against a fixed day.
pub(crate) fn marks() -> Marks {
    Marks::new(IconSet::NerdFont, TODAY)
}

/// The plain marks, against the same day.
pub(crate) fn plain_marks() -> Marks {
    Marks::new(IconSet::Ascii, TODAY)
}

/// Tasks, projects and sections are built from API-shaped JSON, so these tests pin the field
/// names the list reads as well as the layout it produces.
pub(crate) fn task(json: &str) -> Task {
    serde_json::from_str(json).expect("task")
}

fn project(json: &str) -> Project {
    serde_json::from_str(json).expect("project")
}

fn section(json: &str) -> Section {
    serde_json::from_str(json).expect("section")
}

fn texts(rows: &[Row]) -> Vec<String> {
    rows.iter().map(Row::text).collect()
}

/// The rows of one task, built with the marks handed in.
fn line_of(json: &str, marks: &Marks) -> String {
    let rows = build(
        &[task(json)],
        &[project(r#"{"id":"p1","name":"First"}"#)],
        &[],
        marks,
    );
    texts(&rows).remove(1)
}

#[test]
fn tasks_are_grouped_under_their_project_and_section() {
    let rows = build(
        &[
            task(r#"{"id":"1","content":"loose","project_id":"p1","child_order":1}"#),
            task(
                r#"{"id":"2","content":"filed","project_id":"p1","section_id":"s1","child_order":2}"#,
            ),
            task(r#"{"id":"3","content":"later","project_id":"p2","child_order":1}"#),
        ],
        &[
            project(r#"{"id":"p2","name":"Second","child_order":2}"#),
            project(r#"{"id":"p1","name":"First","child_order":1}"#),
        ],
        &[section(
            r#"{"id":"s1","name":"Doing","project_id":"p1","section_order":1}"#,
        )],
        &marks(),
    );

    assert_eq!(
        texts(&rows),
        vec![
            "First",
            "  loose",
            "  Doing",
            "    filed",
            "Second",
            "  later",
        ]
    );
}

#[test]
fn a_project_with_no_open_task_gets_no_heading() {
    let rows = build(
        &[],
        &[project(r#"{"id":"p1","name":"Empty"}"#)],
        &[section(r#"{"id":"s1","name":"Doing","project_id":"p1"}"#)],
        &marks(),
    );

    assert!(rows.is_empty(), "{rows:?}");
}

#[test]
fn subtasks_are_folded_under_their_parent_and_counted_on_it() {
    let rows = build(
        &[
            task(r#"{"id":"1","content":"parent","project_id":"p1","child_order":1}"#),
            task(
                r#"{"id":"3","content":"second child","project_id":"p1","parent_id":"1","child_order":2}"#,
            ),
            task(
                r#"{"id":"2","content":"first child","project_id":"p1","parent_id":"1","child_order":1}"#,
            ),
            task(r#"{"id":"4","content":"grandchild","project_id":"p1","parent_id":"2"}"#),
        ],
        &[project(r#"{"id":"p1","name":"First"}"#)],
        &[],
        &marks(),
    );

    assert_eq!(
        texts(&rows),
        vec![
            "First",
            "  parent  (2)",
            "    first child  (1)",
            "      grandchild",
            "    second child",
        ]
    );
}

#[test]
fn a_subtask_whose_parent_is_not_open_stands_on_its_own() {
    let rows = build(
        &[task(
            r#"{"id":"2","content":"orphan","project_id":"p1","parent_id":"gone"}"#,
        )],
        &[project(r#"{"id":"p1","name":"First"}"#)],
        &[],
        &marks(),
    );

    assert_eq!(texts(&rows), vec!["First", "  orphan"]);
}

/// The marks of a task line, with the color slot each is painted in.
fn marks_of(json: &str, marks: &Marks) -> Vec<(String, Slot)> {
    let rows = build(
        &[task(json)],
        &[project(r#"{"id":"p1","name":"First"}"#)],
        &[],
        marks,
    );
    let Row::Task(row) = rows.into_iter().nth(1).expect("a task row") else {
        panic!("the second row is the task");
    };
    row.text
        .into_iter()
        .map(|part| (part.text, part.slot))
        .filter(|(text, _)| !text.trim().is_empty())
        .collect()
}

#[test]
fn a_line_leads_with_its_marks_and_ends_with_its_title() {
    let line = line_of(
        r#"{"id":"1","content":"file taxes","project_id":"p1","priority":4,
             "labels":["home","slow"],"due":{"date":"2026-09-18T09:00:00Z"}}"#,
        &plain_marks(),
    );

    assert_eq!(line, "  ! * @2 file taxes");
}

#[test]
fn each_priority_takes_its_own_mark_and_the_apps_own_color() {
    let of = |priority: u8| {
        marks_of(
            &format!(r#"{{"id":"1","content":"x","project_id":"p1","priority":{priority}}}"#),
            &plain_marks(),
        )
        .first()
        .cloned()
    };

    assert_eq!(of(4), Some(("!".to_string(), Slot::Red)));
    assert_eq!(of(3), Some(("^".to_string(), Slot::Orange)));
    assert_eq!(of(2), Some(("-".to_string(), Slot::Blue)));
    // The lowest priority is the app's "no priority": the only mark left is the title.
    assert_eq!(of(1), Some(("x".to_string(), Slot::Text)));
}

#[test]
fn each_due_state_marks_the_line_in_its_own_color() {
    let of = |due: &str| {
        let json =
            format!(r#"{{"id":"1","content":"x","project_id":"p1","due":{{"date":"{due}"}}}}"#);
        marks_of(&json, &plain_marks()).first().cloned()
    };

    assert_eq!(of("2026-09-17"), Some(("<09-17".to_string(), Slot::Red)));
    assert_eq!(of("2026-09-18"), Some(("*".to_string(), Slot::Yellow)));
    assert_eq!(of("2026-09-25"), Some((">09-25".to_string(), Slot::Blue)));
}

#[test]
fn a_task_with_no_due_date_carries_no_due_mark() {
    let marks = marks_of(
        r#"{"id":"1","content":"someday","project_id":"p1"}"#,
        &plain_marks(),
    );

    assert_eq!(marks, vec![("someday".to_string(), Slot::Text)]);
}

#[test]
fn a_due_date_in_another_year_is_shown_whole() {
    let line = line_of(
        r#"{"id":"1","content":"x","project_id":"p1","due":{"date":"2025-12-31"}}"#,
        &plain_marks(),
    );

    assert_eq!(line, "  <2025-12-31 x");
}

#[test]
fn a_recurring_task_carries_the_repeat_mark_and_a_plain_one_does_not() {
    let recurring = line_of(
        r#"{"id":"1","content":"x","project_id":"p1",
             "due":{"date":"2026-09-18","is_recurring":true}}"#,
        &plain_marks(),
    );
    let plain = line_of(
        r#"{"id":"1","content":"x","project_id":"p1","due":{"date":"2026-09-18"}}"#,
        &plain_marks(),
    );

    assert_eq!(recurring, "  * ~ x");
    assert_eq!(plain, "  * x");
}

#[test]
fn labels_are_counted_rather_than_named() {
    let line = line_of(
        r#"{"id":"1","content":"x","project_id":"p1","labels":["home","slow","errand"]}"#,
        &plain_marks(),
    );

    assert_eq!(line, "  @3 x");
}

#[test]
fn the_nerd_font_set_draws_the_same_line_in_glyphs() {
    let json = r#"{"id":"1","content":"x","project_id":"p1","priority":4,"labels":["home"],
                    "due":{"date":"2026-09-17","is_recurring":true}}"#;

    assert_eq!(
        line_of(json, &marks()),
        "  \u{f024} \u{f071}09-17 \u{f021} \u{f02c}1 x"
    );
    assert_eq!(line_of(json, &plain_marks()), "  ! <09-17 ~ @1 x");
}

#[test]
fn the_lowest_priority_is_left_off_the_line() {
    let rows = build(
        &[task(
            r#"{"id":"1","content":"someday","project_id":"p1","priority":1}"#,
        )],
        &[project(r#"{"id":"p1","name":"First"}"#)],
        &[],
        &marks(),
    );

    assert_eq!(texts(&rows), vec!["First", "  someday"]);
}

#[test]
fn a_task_whose_section_is_unknown_is_treated_as_unfiled() {
    let rows = build(
        &[task(
            r#"{"id":"1","content":"hidden","project_id":"p1","section_id":"s9"}"#,
        )],
        &[project(r#"{"id":"p1","name":"First"}"#)],
        &[],
        &marks(),
    );

    assert_eq!(texts(&rows), vec!["First", "  hidden"]);
}

#[test]
fn a_task_with_no_project_id_is_grouped_under_a_named_heading() {
    let rows = build(
        &[task(r#"{"id":"1","content":"bare"}"#)],
        &[],
        &[],
        &marks(),
    );

    assert_eq!(texts(&rows), vec!["(no project)", "  bare"]);
}

#[test]
fn a_task_whose_project_is_unknown_is_grouped_under_its_project_id() {
    let rows = build(
        &[task(r#"{"id":"1","content":"stray","project_id":"p9"}"#)],
        &[],
        &[],
        &marks(),
    );

    assert_eq!(texts(&rows), vec!["p9", "  stray"]);
}

#[test]
fn a_task_whose_title_starts_with_a_plus_is_still_marked_waiting() {
    let mut rows = build(
        &[task(
            r#"{"id":"1","content":"+1 follow up","project_id":"p1"}"#,
        )],
        &[project(r#"{"id":"p1","name":"First"}"#)],
        &[],
        &marks(),
    );

    mark_waiting(&mut rows, &["1"]);

    assert_eq!(texts(&rows), vec!["First", "  + +1 follow up"]);
}

#[test]
fn marking_the_same_row_twice_does_not_double_the_mark() {
    let mut rows = build(
        &[task(r#"{"id":"1","content":"keep me","project_id":"p1"}"#)],
        &[project(r#"{"id":"p1","name":"First"}"#)],
        &[],
        &marks(),
    );

    mark_waiting(&mut rows, &["1"]);
    mark_waiting(&mut rows, &["1"]);

    assert_eq!(texts(&rows), vec!["First", "  + keep me"]);
}
