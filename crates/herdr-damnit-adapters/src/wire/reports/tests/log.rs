use herdr_damnit_domain::Oid;

use super::fixture;
use crate::wire::reports::completions;

#[test]
fn a_log_names_the_day_each_completion_was_committed_on() {
    let document = r#"{"commits":[
      {"id":"c1","at":"2026-09-19T08:00:00Z","message":"first","changes":[
        {"oid":"1","op":"create","fields":["subject"],"kind":"task","subject":"a",
         "done":false,"completed_at":null,"priority":4}]},
      {"id":"c2","at":"2026-09-20T09:30:00Z","message":"second","changes":[
        {"oid":"1","op":"update","fields":["done"],"kind":"task","subject":"a",
         "done":true,"completed_at":"2026-09-20T09:29:11Z","priority":4}]}
    ]}"#;
    let completed = completions(document).expect("it parsed");

    assert_eq!(
        completed.get(&herdr_damnit_domain::Oid::new("1")),
        herdr_damnit_domain::parse_date("2026-09-20").as_ref()
    );
}

#[test]
fn a_completion_with_no_instant_of_its_own_takes_the_day_it_was_committed() {
    let document = r#"{"commits":[
      {"id":"c1","at":"2026-09-18T23:30:00Z","message":"only","changes":[
        {"oid":"1","op":"update","fields":["done"],"kind":"task","subject":"a","done":true}]}
    ]}"#;
    assert_eq!(
        completions(document).expect("it parsed")[&herdr_damnit_domain::Oid::new("1")],
        herdr_damnit_domain::parse_date("2026-09-18").expect("a date")
    );
}

#[test]
fn a_later_edit_of_a_completed_task_is_not_a_second_completion() {
    let document = r#"{"commits":[
      {"id":"c1","at":"2026-09-20T09:00:00Z","message":"done","changes":[
        {"oid":"1","op":"update","fields":["done"],"kind":"task","subject":"a",
         "done":true,"completed_at":"2026-09-20T09:00:00Z"}]},
      {"id":"c2","at":"2026-09-25T09:00:00Z","message":"retitled","changes":[
        {"oid":"1","op":"update","fields":["subject"],"kind":"task","subject":"b",
         "done":true,"completed_at":"2026-09-20T09:00:00Z"}]}
    ]}"#;
    assert_eq!(
        completions(document).expect("it parsed")[&herdr_damnit_domain::Oid::new("1")],
        herdr_damnit_domain::parse_date("2026-09-20").expect("a date")
    );
}

#[test]
fn an_edit_after_a_completion_does_not_move_the_completion_day() {
    let document = r#"{"commits":[
      {"id":"c1","at":"2026-09-20T09:00:00Z","message":"done","changes":[
        {"oid":"1","op":"update","fields":["done"],"kind":"task","subject":"a","done":true}]},
      {"id":"c2","at":"2026-09-25T09:00:00Z","message":"retitled","changes":[
        {"oid":"1","op":"update","fields":["subject"],"kind":"task","subject":"b","done":true}]}
    ]}"#;
    assert_eq!(
        completions(document).expect("it parsed")[&Oid::new("1")],
        herdr_damnit_domain::parse_date("2026-09-20").expect("a date")
    );
}

#[test]
fn a_reopen_names_done_and_is_not_a_completion() {
    let document = r#"{"commits":[
      {"id":"c1","at":"2026-09-22T10:00:00Z","message":"reopened","changes":[
        {"oid":"1","op":"update","fields":["done"],"kind":"task","subject":"a",
         "done":false,"completed_at":null}]}
    ]}"#;
    assert!(
        completions(document).expect("it parsed").is_empty(),
        "a reopen was read as a completion"
    );
}

#[test]
fn a_task_that_was_never_completed_has_no_date() {
    assert!(
        completions(&fixture("log.json"))
            .expect("it parsed")
            .is_empty()
    );
}

#[test]
fn the_captured_log_of_a_real_completion_names_exactly_one_completed_object() {
    let completed = completions(&fixture("log-completion.json")).expect("it parsed");
    assert_eq!(completed.len(), 1, "{completed:?}");
}
