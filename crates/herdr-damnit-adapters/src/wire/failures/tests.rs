//! The error document `dam` prints on standard error under `--json`.

use super::*;

#[test]
fn an_error_document_carries_its_kind_its_rule_and_the_objects_it_names() {
    let stderr = r#"{"error":{"kind":"refused","rule":"blocked","message":"1a2b3c4 waits on 5d6e7f8","oids":["1a2b3c4","5d6e7f8"]}}"#;
    let document = error_document(stderr).expect("it parsed");

    assert_eq!(document.kind, herdr_damnit_domain::ErrorKind::Refused);
    assert_eq!(document.rule, Some(herdr_damnit_domain::Rule::Blocked));
    assert_eq!(document.message, "1a2b3c4 waits on 5d6e7f8");
    assert_eq!(
        document.oids,
        vec![
            herdr_damnit_domain::Oid::new("1a2b3c4"),
            herdr_damnit_domain::Oid::new("5d6e7f8"),
        ]
    );
}

/// A rule word this pane has not heard of still reaches the status line, because the message is
/// `dam`'s own either way.
#[test]
fn a_rule_this_pane_has_not_heard_of_keeps_its_word() {
    let stderr =
        r#"{"error":{"kind":"refused","rule":"invented_tomorrow","message":"no","oids":[]}}"#;
    assert_eq!(
        error_document(stderr).expect("it parsed").rule,
        Some(herdr_damnit_domain::Rule::Unknown(
            "invented_tomorrow".to_string()
        ))
    );
}

/// Every kind but a refusal carries a null rule, which is `dam`'s own contract.
#[test]
fn a_failure_that_is_not_a_refusal_names_no_rule() {
    let stderr =
        r#"{"error":{"kind":"store","rule":null,"message":"database is locked","oids":[]}}"#;
    let document = error_document(stderr).expect("it parsed");
    assert_eq!(document.kind, herdr_damnit_domain::ErrorKind::Store);
    assert_eq!(document.rule, None);
}

/// clap answers a bad command line before `dam` runs, so there is no document to read; Task 14's
/// `classify` falls back to the first line of standard error in that case.
#[test]
fn standard_error_that_is_not_a_document_is_no_document_rather_than_an_error() {
    assert!(error_document("error: unexpected argument '--nope'").is_none());
    assert!(error_document("dam: no object matches \"zzzzzzz\"").is_none());
    assert!(error_document("").is_none());
    assert!(
        error_document(r#"{"objects":[]}"#).is_none(),
        "a report is not an error document"
    );
}
