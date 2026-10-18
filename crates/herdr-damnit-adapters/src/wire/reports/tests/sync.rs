use super::fixture;
use crate::wire::reports::{pull_summary, push_summary};

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
