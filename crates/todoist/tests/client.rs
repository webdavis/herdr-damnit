mod support;

use todoist::{Client, Error, TokenSource, resolve_with};

async fn token() -> todoist::Token {
    resolve_with(&TokenSource::Env("T".to_string()), |_| {
        Some("test-token".to_string())
    })
    .await
    .expect("resolves")
}

#[tokio::test]
async fn a_successful_request_sends_the_bearer_token_and_parses_the_user() {
    let double = support::serve_once(
        "200 OK",
        &["Content-Type: application/json"],
        r#"{"id":"42","email":"someone@example.com","full_name":"Someone"}"#,
    )
    .await;
    let client = Client::new(&double.base_url, token().await).expect("client");

    let user = client.user().await.expect("request succeeds");

    assert_eq!(user.id, "42");
    let request = double.request.lock().expect("lock").clone();
    assert!(request.starts_with("GET /user "), "{request}");
    assert!(
        request.contains("authorization: Bearer test-token"),
        "{request}"
    );
}

#[tokio::test]
async fn a_rejected_token_maps_to_unauthorized() {
    let double = support::serve_once("401 Unauthorized", &[], "{}").await;
    let client = Client::new(&double.base_url, token().await).expect("client");

    assert_eq!(client.user().await.unwrap_err(), Error::Unauthorized);
}

#[tokio::test]
async fn a_forbidden_response_maps_to_forbidden() {
    let double = support::serve_once("403 Forbidden", &[], "{}").await;
    let client = Client::new(&double.base_url, token().await).expect("client");

    assert_eq!(client.user().await.unwrap_err(), Error::Forbidden);
}

#[tokio::test]
async fn a_rate_limit_keeps_the_retry_after_delay() {
    let double = support::serve_once("429 Too Many Requests", &["Retry-After: 12"], "{}").await;
    let client = Client::new(&double.base_url, token().await).expect("client");

    let error = client.user().await.unwrap_err();

    assert_eq!(error, Error::RateLimitedAfter(12));
    assert_eq!(error.to_string(), "rate limited, retry in 12s");
}

#[tokio::test]
async fn an_unreachable_api_reports_a_network_error_without_the_url() {
    let base_url = support::dead_base_url().await;
    let client = Client::new(&base_url, token().await).expect("client");

    let error = client.user().await.unwrap_err();

    assert!(matches!(error, Error::Network(_)), "{error}");
    assert!(!error.to_string().contains("127.0.0.1"), "{error}");
}

#[tokio::test]
async fn a_body_that_is_not_the_expected_shape_reports_an_unreadable_response() {
    let double = support::serve_once("200 OK", &[], "not json").await;
    let client = Client::new(&double.base_url, token().await).expect("client");

    assert!(matches!(
        client.user().await.unwrap_err(),
        Error::Malformed(_)
    ));
}
