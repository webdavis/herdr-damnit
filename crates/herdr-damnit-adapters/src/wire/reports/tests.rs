//! The status document, the log walk and the two sync summaries.

use super::*;
use herdr_damnit_domain::{Oid, Op};

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn a_clean_status_is_a_clean_stage() {
    assert!(
        stage(&fixture("status-clean.json"))
            .expect("it parsed")
            .is_clean()
    );
}

#[test]
fn a_full_status_fills_every_array_it_carries() {
    let stage = stage(&fixture("status-full.json")).expect("it parsed");
    assert!(!stage.is_clean());
    assert!(
        !stage.unstaged.is_empty(),
        "the captured store had working changes"
    );
    assert!(
        stage
            .unstaged
            .iter()
            .all(|change| !change.subject.is_empty()),
        "a change lost its subject"
    );
    assert!(
        stage
            .unstaged
            .iter()
            .all(|change| !change.fields.is_empty()),
        "a change lost the field names it touches"
    );
}

#[test]
fn an_operation_word_becomes_its_own_variant() {
    let document = r#"{"staged":[
      {"oid":"1","op":"create","before":null,"after":{"oid":"1","kind":"task","subject":"a"}},
      {"oid":"2","op":"update","before":{"oid":"2","kind":"task","subject":"b"},
       "after":{"oid":"2","kind":"task","subject":"b"}},
      {"oid":"3","op":"delete","before":{"oid":"3","kind":"task","subject":"c"},"after":null}
    ],"unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#;
    let ops: Vec<Op> = stage(document)
        .expect("it parsed")
        .staged
        .into_iter()
        .map(|change| change.op)
        .collect();
    assert_eq!(ops, vec![Op::Create, Op::Update, Op::Delete]);
}

/// Without `--full` a change names its own subject; with it the subject is on the embedded object
/// instead, and the Status screen needs a line either way.
#[test]
fn a_change_from_a_full_status_takes_its_subject_off_the_object_it_embeds() {
    let document = r#"{"staged":[{"oid":"1","op":"create","before":null,
      "after":{"oid":"1","kind":"task","subject":"the embedded one"}}],
      "unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#;
    assert_eq!(
        stage(document).expect("it parsed").staged[0].subject,
        "the embedded one"
    );
}

#[test]
fn a_conflict_and_a_notice_carry_the_text_the_status_screen_draws() {
    let document = r#"{"staged":[],"unstaged":[],"unpushed":[{"remote":"example","commits":2}],
      "conflicts":[{"oid":"3d4e5f6","remote":"example",
        "ours":{"oid":"3d4e5f6","kind":"task","subject":"mine"},
        "theirs":{"oid":"3d4e5f6","kind":"task","subject":"theirs"}}],
      "notices":[{"kind":"pull_failed","remote":"example","why":"the service is unreachable"}]}"#;
    let stage = stage(document).expect("it parsed");

    assert_eq!(stage.conflicts[0].ours, "mine");
    assert_eq!(stage.conflicts[0].theirs, "theirs");
    assert_eq!(stage.unpushed[0].commits, 2);
    assert!(
        stage.unpushed[0].oids.is_empty(),
        "a dam that sends no oids leaves the set empty"
    );
    assert_eq!(stage.notices[0].kind, "pull_failed");
    assert_eq!(
        stage.notices[0].message,
        "example: pull failed: the service is unreachable"
    );
}

/// A `dam` that publishes the objects behind its unpushed commits gives every one of them its
/// unpushed mark.
#[test]
fn an_unpushed_remote_that_names_its_objects_carries_them() {
    let document = r#"{"staged":[],"unstaged":[],"conflicts":[],"notices":[],
      "unpushed":[{"remote":"example","commits":2,"oids":["1a2b3c4","5d6e7f8"]}]}"#;
    let stage = stage(document).expect("it parsed");
    assert!(stage.is_unpushed(&herdr_damnit_domain::Oid::new("1a2b3c4")));
    assert!(stage.is_unpushed(&herdr_damnit_domain::Oid::new("5d6e7f8")));
}

#[test]
fn a_removed_upstream_notice_reads_as_a_sentence_rather_than_a_kind() {
    let document = r#"{"staged":[],"unstaged":[],"conflicts":[],"unpushed":[],
      "notices":[{"kind":"removed_upstream","remote":"example","oid":"7a8b9c0",
                  "subject":"old task"}]}"#;
    assert_eq!(
        stage(document).expect("it parsed").notices[0].message,
        "removed on example: \"old task\" is kept here"
    );
}

/// Each of the five kinds `dam` 0.2.0 publishes reads as a sentence of its own, including the one
/// that carries no `why` at all.
#[test]
fn every_notice_kind_dam_publishes_reads_as_a_sentence() {
    let document = r#"{"staged":[],"unstaged":[],"conflicts":[],"unpushed":[],"notices":[
      {"kind":"push_failed","remote":"example","oid":"1","why":"rate limited"},
      {"kind":"pull_failed","remote":"example","why":"the helper closed its output"},
      {"kind":"kind_changed","oid":"2","ours":"task","theirs":"event"},
      {"kind":"event_cancelled","oid":"3","subject":"quarterly review","attached":1},
      {"kind":"removed_upstream","remote":"example","oid":"4","subject":"from upstream"}
    ]}"#;
    let messages: Vec<String> = stage(document)
        .expect("it parsed")
        .notices
        .into_iter()
        .map(|notice| notice.message)
        .collect();

    assert_eq!(
        messages,
        vec![
            "example: push failed: rate limited",
            "example: pull failed: the helper closed its output",
            "kind changed: a task here and an event upstream",
            "event cancelled: \"quarterly review\"",
            "removed on example: \"from upstream\" is kept here",
        ]
    );
}

/// A notice kind `dam` adds later still reaches the screen, named rather than dropped.
#[test]
fn a_notice_kind_this_pane_has_not_heard_of_still_reaches_the_screen() {
    let document = r#"{"staged":[],"unstaged":[],"conflicts":[],"unpushed":[],
      "notices":[{"kind":"invented_tomorrow","remote":"example","why":"something happened"}]}"#;
    let notice = stage(document).expect("it parsed").notices.remove(0);
    assert_eq!(notice.kind, "invented_tomorrow");
    assert!(notice.message.contains("something happened"), "{notice:?}");
}

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

/// A `dam` that names no completion instant leaves the commit's own day as the answer.
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

/// A change that touched something other than `done` is not a completion, even on a task that was
/// already complete.
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

/// Only the commit that flipped `done` is the completion, so a later edit of a task that is
/// already complete leaves the original day standing.
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

/// `dam edit --undone` commits a change that names `done` and leaves the task open, which is the
/// opposite of a completion and must not date one.
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

/// The captured log of a real completion, which is the only proof the walk reads the shape `dam`
/// 0.2.0 actually prints rather than one this pane invented.
#[test]
fn the_captured_log_of_a_completion_names_exactly_one_completed_object() {
    let completed = completions(&fixture("log-completion.json")).expect("it parsed");
    assert_eq!(completed.len(), 1, "{completed:?}");
}

#[test]
fn the_sync_summaries_are_rebuilt_from_json_rather_than_read_from_human_output() {
    assert_eq!(
        push_summary(&fixture("push-ok.json")).expect("it parsed"),
        "example: 3 sent, 3 ok, 0 failed, 0 skipped"
    );
    assert_eq!(
        push_summary(&fixture("push-partial-failure.json")).expect("it parsed"),
        "example: 3 sent, 2 ok, 1 failed, 0 skipped"
    );
    assert_eq!(
        pull_summary(&fixture("pull-ok.json")).expect("it parsed"),
        "example: 2 new, 1 updated, 40 unchanged, 0 conflict(s), 0 removed upstream"
    );
    assert_eq!(
        pull_summary(&fixture("pull-conflict.json")).expect("it parsed"),
        "example: 2 new, 1 updated, 40 unchanged, 1 conflict(s), 0 removed upstream"
    );
}

/// A push to every remote answers per remote, and the status line is one line, so the summaries
/// are joined rather than stacked.
#[test]
fn two_remotes_are_two_summaries_on_one_line() {
    let document = r#"{"remotes":[
      {"remote":"one","sent":1,"succeeded":1,"skipped":0,"failed":[]},
      {"remote":"two","sent":2,"succeeded":0,"skipped":2,"failed":[]}]}"#;
    assert_eq!(
        push_summary(document).expect("it parsed"),
        "one: 1 sent, 1 ok, 0 failed, 0 skipped; two: 2 sent, 0 ok, 0 failed, 2 skipped"
    );
}

#[test]
fn something_that_is_not_json_is_an_error_rather_than_an_empty_model() {
    assert!(stage("not json").is_err());
    assert!(completions("not json").is_err());
    assert!(push_summary("not json").is_err());
    assert!(pull_summary("not json").is_err());
}
