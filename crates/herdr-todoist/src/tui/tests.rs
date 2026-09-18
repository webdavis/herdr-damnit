use super::*;

#[tokio::test]
async fn a_refresh_interval_of_zero_still_reads_once_when_the_pane_opens() {
    let base_url =
        crate::reload::tests::serve_forever("200 OK", crate::reload::tests::EMPTY_PAGE).await;
    let config = Config::parse(
        "refresh_seconds = 0\ntoken_command = [\"sh\", \"-c\", \"printf test-token\"]",
    )
    .expect("parses");
    let mut list = List::new(Vec::new());
    let mut views = Views::new(&[]);
    let (mut cache, mut queue) = crate::reload::tests::stores("tui-open-zero-interval");

    let (_, schedule, status) = open(
        &config, &base_url, &mut list, &mut views, &mut cache, &mut queue,
    )
    .await;

    assert_eq!(
        status, "0 open tasks",
        "refresh_seconds = 0 must not skip the opening read"
    );
    assert!(
        !schedule.due(crate::cache::now() + 1_000_000, false),
        "the interval itself stays off"
    );
}

#[test]
fn ctrl_d_folds_to_the_send_key_the_comment_box_listens_for() {
    assert_eq!(
        fold(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL)),
        crate::prompt::SEND
    );
}

#[test]
fn a_plain_letter_and_a_control_key_with_no_letter_pass_through_unfolded() {
    assert_eq!(
        fold(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE)),
        KeyCode::Char('d')
    );
    assert_eq!(
        fold(KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL)),
        KeyCode::Enter
    );
}

#[test]
fn a_numbered_action_s_request_outranks_the_configured_opening_view() {
    let config = Config::parse(
        "default_view = \"today\"\n\
         [[views]]\nname = \"today\"\nfilter = \"today\"\n\
         [[views]]\nname = \"work\"\nfilter = \"#Work\"\n",
    )
    .expect("parses");
    let request =
        std::env::temp_dir().join(format!("herdr-todoist-opening-{}", std::process::id()));
    let _ = std::fs::remove_file(&request);

    assert_eq!(
        opening_view(&config, &request),
        Some("today".to_string()),
        "with nothing asked for, the pane opens on the configured view"
    );

    crate::state::request_view(&request, "work");
    assert_eq!(
        opening_view(&config, &request),
        Some("work".to_string()),
        "a view action's request wins over the configured view"
    );
    assert_eq!(
        opening_view(&config, &request),
        Some("today".to_string()),
        "the request is spent once it has been honoured"
    );

    assert_eq!(
        opening_view(&Config::default(), &request),
        None,
        "with no configured view the pane opens on the unfiltered list"
    );
}

#[tokio::test]
async fn the_detail_is_opened_for_the_task_under_the_cursor_and_leaves_that_cursor_alone() {
    let base_url =
        crate::reload::tests::serve_forever("200 OK", crate::reload::tests::EMPTY_PAGE).await;
    let config = crate::reload::tests::config_with_token_command("printf test-token");
    let mut connection = Connection::build(&config, &base_url).await;
    let project: todoist::Project =
        serde_json::from_str(r#"{"id":"p1","name":"First"}"#).expect("project");
    let mut list = List::new(crate::list::build(
        &[
            crate::list::tests::task(
                r#"{"id":"1","content":"first","project_id":"p1","child_order":1,
                     "description":"the first one"}"#,
            ),
            crate::list::tests::task(
                r#"{"id":"2","content":"second","project_id":"p1","child_order":2,
                     "description":"the second one"}"#,
            ),
        ],
        &[project],
        &[],
        &crate::list::tests::marks(),
    ));
    list.move_cursor(1);
    let at = list.selected();

    let mut detail = open_detail(&mut connection, &config, &base_url, &list)
        .await
        .expect("a task under the cursor");

    // The detail is about the second task, description and all, read off the row the list
    // already fetched rather than by a second read of the task.
    let drawn: Vec<String> = detail
        .lines()
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect()
        })
        .collect();
    assert!(drawn.contains(&"second".to_string()), "{drawn:?}");
    assert!(drawn.contains(&"the second one".to_string()), "{drawn:?}");

    // Scrolling, opening the box and coming back leave the list exactly as it was: the detail
    // screen never touches it.
    for key in [
        KeyCode::Char('j'),
        KeyCode::Char('c'),
        KeyCode::Char('x'),
        KeyCode::Esc,
    ] {
        detail.key(key, &mut connection, &config, &base_url).await;
    }
    assert_eq!(
        detail
            .key(KeyCode::Esc, &mut connection, &config, &base_url)
            .await,
        DetailAfter::Back
    );

    assert_eq!(list.selected(), at);
    assert_eq!(list.selected_id(), Some("2"));
    assert_eq!(list.task_count(), 2);
}

#[test]
fn the_cursor_on_a_heading_has_no_task_to_open() {
    let headings = List::new(vec![crate::list::Row::Header("First".to_string())]);

    assert!(headings.selected_task().is_none());
}
