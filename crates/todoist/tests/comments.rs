//! The comment endpoints, against a loopback double. No token and no live call.

mod support;

use todoist::{Client, TokenSource, resolve_with};

async fn client(base_url: &str) -> Client {
    let token = resolve_with(&TokenSource::Env("T".to_string()), |_| {
        Some("test-token".to_string())
    })
    .await
    .expect("resolves");
    Client::new(base_url, token).expect("client")
}

/// The document the vendor schema describes: a paginated list of notes, whose poster is
/// `posted_uid` and whose attachment is `file_attachment`, both nullable.
const THREAD: &str = r#"{"results":[
    {"id":"c1","content":"first","posted_at":"2026-09-16T08:00:00Z","posted_uid":"7",
     "file_attachment":null,"uids_to_notify":null,"is_deleted":false,"reactions":null},
    {"id":"c2","content":"with a file","posted_at":"2026-09-17T08:00:00Z","posted_uid":"7",
     "file_attachment":{"resource_type":"file","file_name":"receipt.pdf",
       "file_type":"application/pdf","file_url":"https://example.invalid/receipt.pdf"},
     "uids_to_notify":null,"is_deleted":false,"reactions":null}],
    "next_cursor":null}"#;

#[tokio::test]
async fn the_thread_is_read_off_the_task_and_parses_every_field_the_pane_draws() {
    let double = support::serve_once("200 OK", &["Content-Type: application/json"], THREAD).await;
    let client = client(&double.base_url).await;

    let comments = client.comments("6X").await.expect("the thread");

    let request = double.request.lock().expect("lock").clone();
    assert!(request.starts_with("GET /comments?"), "{request}");
    assert!(request.contains("task_id=6X"), "{request}");
    assert_eq!(comments.len(), 2);
    assert_eq!(comments[0].content, "first");
    assert_eq!(
        comments[0].posted_at.as_deref(),
        Some("2026-09-16T08:00:00Z")
    );
    assert_eq!(comments[0].posted_uid.as_deref(), Some("7"));
    assert_eq!(comments[0].attachment(), None, "a null file_attachment");
    assert_eq!(comments[1].attachment().as_deref(), Some("receipt.pdf"));
}

#[tokio::test]
async fn a_comment_with_no_name_on_its_attachment_is_still_reported_as_carrying_one() {
    let double = support::serve_once(
        "200 OK",
        &["Content-Type: application/json"],
        r#"{"results":[{"id":"c1","content":"","posted_at":null,"posted_uid":null,
             "file_attachment":{"resource_type":"file"},"uids_to_notify":null,
             "is_deleted":false,"reactions":null}],"next_cursor":null}"#,
    )
    .await;
    let client = client(&double.base_url).await;

    let comments = client.comments("6X").await.expect("the thread");

    assert_eq!(comments[0].attachment().as_deref(), Some("file"));
    assert_eq!(comments[0].posted_at, None);
}

#[tokio::test]
async fn adding_a_comment_sends_the_task_and_the_text_and_nothing_else() {
    let double = support::serve_once(
        "200 OK",
        &["Content-Type: application/json"],
        r#"{"id":"c9","content":"typed","posted_at":null,"posted_uid":null,
             "file_attachment":null,"uids_to_notify":null,"is_deleted":false,"reactions":null}"#,
    )
    .await;
    let client = client(&double.base_url).await;

    client
        .add_comment("6X", "first line\nsecond line")
        .await
        .expect("accepted");

    let request = double.request.lock().expect("lock").clone();
    assert!(request.starts_with("POST /comments "), "{request}");
    assert!(
        request.ends_with(r#"{"task_id":"6X","content":"first line\nsecond line"}"#),
        "{request}"
    );
}

#[tokio::test]
async fn a_refused_comment_carries_the_api_s_own_message() {
    let double = support::serve_once(
        "400 Bad Request",
        &[],
        r#"{"error":"Invalid argument value",
             "error_extra":{"explanation":"Content is too long"}}"#,
    )
    .await;
    let client = client(&double.base_url).await;

    let error = client.add_comment("6X", "x").await.expect_err("refused");

    assert_eq!(
        error.to_string(),
        "Invalid argument value: Content is too long"
    );
}
