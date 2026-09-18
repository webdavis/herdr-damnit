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

/// Two pages of tasks, the first pointing at the second.
const TASK_PAGES: &[(&str, &str)] = &[
    (
        "cursor=second-page",
        r#"{"results":[{"id":"2","content":"child","project_id":"p1","section_id":"s1",
             "parent_id":"1","priority":4,"labels":["home"],"due":{"date":"2026-09-18"},
             "child_order":2}],"next_cursor":null}"#,
    ),
    (
        "/tasks",
        r#"{"results":[{"id":"1","content":"parent","project_id":"p1","child_order":1}],
             "next_cursor":"second-page"}"#,
    ),
];

#[tokio::test]
async fn tasks_are_read_to_the_last_page_and_keep_the_fields_the_list_shows() {
    let double = support::serve_routes(TASK_PAGES).await;
    let client = Client::new(&double.base_url, token().await).expect("client");

    let tasks = client.tasks().await.expect("request succeeds");

    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0].id, "1");
    let child = &tasks[1];
    assert_eq!(child.parent_id.as_deref(), Some("1"));
    assert_eq!(child.section_id.as_deref(), Some("s1"));
    assert_eq!(child.priority, 4);
    assert_eq!(child.labels, vec!["home".to_string()]);
    assert_eq!(
        child.due.as_ref().map(|due| due.date.as_str()),
        Some("2026-09-18")
    );
    let targets = double.targets.lock().expect("lock").clone();
    assert_eq!(targets.len(), 2, "{targets:?}");
    assert!(targets[0].contains("limit=200"), "{targets:?}");
    assert!(targets[1].contains("cursor=second-page"), "{targets:?}");
}

#[tokio::test]
async fn a_task_with_only_the_required_fields_parses() {
    let double = support::serve_routes(&[(
        "/tasks",
        r#"{"results":[{"id":"1","content":"bare"}],"next_cursor":null}"#,
    )])
    .await;
    let client = Client::new(&double.base_url, token().await).expect("client");

    let tasks = client.tasks().await.expect("request succeeds");

    assert_eq!(tasks[0].priority, 1);
    assert!(tasks[0].labels.is_empty());
    assert!(tasks[0].project_id.is_none());
}

#[tokio::test]
async fn projects_and_sections_are_read_with_their_names_and_order() {
    let double = support::serve_routes(&[
        (
            "/projects",
            r#"{"results":[{"id":"p1","name":"First","child_order":3}],"next_cursor":null}"#,
        ),
        (
            "/sections",
            r#"{"results":[{"id":"s1","name":"Doing","project_id":"p1","section_order":2}],
                 "next_cursor":null}"#,
        ),
    ])
    .await;
    let client = Client::new(&double.base_url, token().await).expect("client");

    let projects = client.projects().await.expect("projects");
    let sections = client.sections().await.expect("sections");

    assert_eq!((projects[0].name.as_str(), projects[0].order), ("First", 3));
    assert_eq!(
        (
            sections[0].name.as_str(),
            sections[0].project_id.as_str(),
            sections[0].order
        ),
        ("Doing", "p1", 2)
    );
}

#[tokio::test]
async fn a_repeated_cursor_ends_the_walk_instead_of_looping() {
    let double = support::serve_routes(&[(
        "/tasks",
        r#"{"results":[{"id":"1","content":"one"}],"next_cursor":"same"}"#,
    )])
    .await;
    let client = Client::new(&double.base_url, token().await).expect("client");

    let tasks = client.tasks().await.expect("request succeeds");

    assert_eq!(
        tasks.len(),
        2,
        "the repeated cursor is followed exactly once"
    );
}

/// Two pages of one filter query's results, the first pointing at the second.
const FILTER_PAGES: &[(&str, &str)] = &[
    (
        "cursor=second-page",
        r#"{"results":[{"id":"2","content":"overdue one","project_id":"p1"}],"next_cursor":null}"#,
    ),
    (
        "/tasks/filter",
        r#"{"results":[{"id":"1","content":"today one","project_id":"p1"}],
             "next_cursor":"second-page"}"#,
    ),
];

#[tokio::test]
async fn a_filter_query_goes_to_the_filter_endpoint_and_is_read_to_the_last_page() {
    let double = support::serve_routes(FILTER_PAGES).await;
    let client = Client::new(&double.base_url, token().await).expect("client");

    let tasks = client
        .tasks_matching("today | overdue")
        .await
        .expect("request succeeds");

    assert_eq!(tasks.len(), 2);
    let targets = double.targets.lock().expect("lock").clone();
    assert!(targets[0].starts_with("/tasks/filter?"), "{targets:?}");
    assert!(
        targets[0].contains("query=today+%7C+overdue"),
        "{targets:?}"
    );
    assert!(targets[0].contains("limit=200"), "{targets:?}");
    assert!(targets[1].contains("cursor=second-page"), "{targets:?}");
}

#[tokio::test]
async fn a_refused_filter_reports_the_message_the_api_sent() {
    let double = support::serve_once(
        "400 Bad Request",
        &["Content-Type: application/json"],
        r#"{"error_tag":"INVALID_ARGUMENT_VALUE","error_code":20,"error":"Invalid argument value",
            "http_code":400,"error_extra":{"argument":"filter",
            "explanation":"Unable to parse the filter query"}}"#,
    )
    .await;
    let client = Client::new(&double.base_url, token().await).expect("client");

    let error = client.tasks_matching("#Work &").await.unwrap_err();

    assert_eq!(
        error,
        Error::Refused("Invalid argument value: Unable to parse the filter query".to_string())
    );
}

#[tokio::test]
async fn a_filter_matching_nothing_is_an_empty_list_rather_than_an_error() {
    let double = support::serve_once(
        "200 OK",
        &["Content-Type: application/json"],
        r#"{"results":[],"next_cursor":null}"#,
    )
    .await;
    let client = Client::new(&double.base_url, token().await).expect("client");

    let tasks = client
        .tasks_matching("today & @nobody")
        .await
        .expect("request succeeds");

    assert!(tasks.is_empty());
}

#[tokio::test]
async fn a_completed_page_is_one_request_that_hands_back_its_cursor() {
    let double = support::serve_routes(&[(
        "/tasks/completed/by_completion_date",
        r#"{"items":[{"id":"1","content":"paid rent","completed_at":"2026-09-16T18:04:00Z"}],
             "next_cursor":"second-page"}"#,
    )])
    .await;
    let client = Client::new(&double.base_url, token().await).expect("client");

    let page = client
        .completed("2026-06-18T00:00:00Z", "2026-09-19T00:00:00Z", None)
        .await
        .expect("request succeeds");

    assert_eq!(page.tasks.len(), 1);
    assert_eq!(page.tasks[0].content, "paid rent");
    assert_eq!(
        page.tasks[0].completed_at.as_deref(),
        Some("2026-09-16T18:04:00Z")
    );
    assert_eq!(page.next_cursor.as_deref(), Some("second-page"));
    let targets = double.targets.lock().expect("lock").clone();
    assert_eq!(
        targets.len(),
        1,
        "one page has to be one request, not a walk: {targets:?}"
    );
    assert!(
        targets[0].contains("since=2026-06-18T00%3A00%3A00Z"),
        "{targets:?}"
    );
    assert!(
        targets[0].contains("until=2026-09-19T00%3A00%3A00Z"),
        "{targets:?}"
    );
    assert!(targets[0].contains("limit=50"), "{targets:?}");
}

#[tokio::test]
async fn the_cursor_of_a_completed_page_is_sent_with_the_same_window() {
    let double = support::serve_routes(&[(
        "/tasks/completed/by_completion_date",
        r#"{"items":[],"next_cursor":null}"#,
    )])
    .await;
    let client = Client::new(&double.base_url, token().await).expect("client");

    client
        .completed(
            "2026-06-18T00:00:00Z",
            "2026-09-19T00:00:00Z",
            Some("second-page"),
        )
        .await
        .expect("request succeeds");

    let targets = double.targets.lock().expect("lock").clone();
    assert!(targets[0].contains("cursor=second-page"), "{targets:?}");
    assert!(
        targets[0].contains("since=2026-06-18T00%3A00%3A00Z"),
        "{targets:?}"
    );
}

#[tokio::test]
async fn a_completed_task_with_a_null_completion_time_still_parses() {
    let double = support::serve_routes(&[(
        "/tasks/completed",
        r#"{"items":[{"id":"1","content":"bare","completed_at":null}],"next_cursor":null}"#,
    )])
    .await;
    let client = Client::new(&double.base_url, token().await).expect("client");

    let page = client
        .completed("2026-06-18T00:00:00Z", "2026-09-19T00:00:00Z", None)
        .await
        .expect("request succeeds");

    assert_eq!(page.tasks[0].completed_at, None);
}

#[tokio::test]
async fn reopening_a_task_posts_to_its_reopen_path() {
    let double = support::serve_once("200 OK", &[], "{}").await;
    let client = Client::new(&double.base_url, token().await).expect("client");

    client.reopen("6X").await.expect("reopen succeeds");

    let request = double.request.lock().expect("lock").clone();
    assert!(request.starts_with("POST /tasks/6X/reopen "), "{request}");
    assert!(
        request.contains("authorization: Bearer test-token"),
        "{request}"
    );
}

#[tokio::test]
async fn a_refused_reopen_carries_the_api_s_own_message() {
    let double = support::serve_once(
        "400 Bad Request",
        &[],
        r#"{"error":"Invalid argument value",
             "error_extra":{"explanation":"Task is not completed"}}"#,
    )
    .await;
    let client = Client::new(&double.base_url, token().await).expect("client");

    let error = client.reopen("6X").await.unwrap_err();

    assert_eq!(
        error.to_string(),
        "Invalid argument value: Task is not completed"
    );
}
