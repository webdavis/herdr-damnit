use super::*;

#[test]
fn an_error_document_carries_dams_message_and_whether_the_store_failed() {
    let stderr = r#"{"error":{"kind":"refused","rule":"blocked","message":"1a2b3c4 waits on 5d6e7f8","oids":["1a2b3c4","5d6e7f8"]}}"#;
    assert_eq!(
        error_document(stderr).expect("it parsed"),
        herdr_damnit_domain::ErrorDocument {
            store: false,
            message: "1a2b3c4 waits on 5d6e7f8".to_string(),
        }
    );
}

#[test]
fn a_store_failure_is_marked_as_the_store() {
    let stderr =
        r#"{"error":{"kind":"store","rule":null,"message":"database is locked","oids":[]}}"#;
    assert!(error_document(stderr).expect("it parsed").store);
}

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
