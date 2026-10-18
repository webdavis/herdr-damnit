use herdr_damnit_domain::{IconSet, Oid, Op, Stage, status_rows};

use super::fixture;
use crate::wire::reports::stage;

#[test]
fn a_clean_status_is_a_clean_stage() {
    assert_eq!(
        status_rows(
            &stage(&fixture("status-clean.json")).expect("it parsed"),
            IconSet::Ascii
        ),
        status_rows(&Stage::default(), IconSet::Ascii)
    );
}

#[test]
fn a_full_status_fills_every_array_it_carries() {
    let stage = stage(&fixture("status-full.json")).expect("it parsed");
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
    assert_eq!(
        stage.notices[0].message,
        "example: pull failed: the service is unreachable"
    );
}

#[test]
fn an_unpushed_remote_that_names_its_objects_carries_them() {
    let document = r#"{"staged":[],"unstaged":[],"conflicts":[],"notices":[],
      "unpushed":[{"remote":"example","commits":2,"oids":["1a2b3c4","5d6e7f8"]}]}"#;
    let stage = stage(document).expect("it parsed");
    assert!(stage.is_unpushed(&herdr_damnit_domain::Oid::new("1a2b3c4")));
    assert!(stage.is_unpushed(&herdr_damnit_domain::Oid::new("5d6e7f8")));
}

#[test]
fn an_unpushed_row_reads_the_objects_and_ignores_the_commit_ids_beside_them() {
    let document = r#"{"staged":[],"unstaged":[],"conflicts":[],"notices":[],
      "unpushed":[{"remote":"example","commits":2,
                   "oids":["1a2b3c4","5d6e7f8"],
                   "commit_ids":["c9c9c9c","d8d8d8d"]}]}"#;
    let stage = stage(document).expect("it parsed");

    assert_eq!(stage.unpushed[0].commits, 2);
    assert_eq!(
        stage.unpushed[0].oids,
        vec![Oid::new("1a2b3c4"), Oid::new("5d6e7f8")]
    );
    assert!(stage.is_unpushed(&Oid::new("1a2b3c4")));
    assert!(
        !stage.is_unpushed(&Oid::new("c9c9c9c")),
        "a commit id was read as an object"
    );
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

#[test]
fn a_notice_kind_this_pane_has_not_heard_of_still_reaches_the_screen() {
    let document = r#"{"staged":[],"unstaged":[],"conflicts":[],"unpushed":[],
      "notices":[{"kind":"invented_tomorrow","remote":"example","why":"something happened"}]}"#;
    let notice = stage(document).expect("it parsed").notices.remove(0);
    assert_eq!(notice.message, "invented_tomorrow: something happened");
}
