//! Opening from the cache, the stale mark, and the writes made while the network was down.

use super::*;

/// A base URL nothing is listening on, which is what the pane sees when the network is down.
pub(crate) async fn unreachable() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    format!("http://{}", listener.local_addr().expect("addr"))
}

/// One task, as the list endpoint sends it.
const ONE_TASK: &str =
    r#"{"results":[{"id":"1","content":"keep me","project_id":"p1"}],"next_cursor":null}"#;

fn warm(dir: &std::path::Path, saved_at: u64) -> Cache {
    let mut cache = Cache::new(dir);
    let task = crate::list::tests::task(r#"{"id":"1","content":"keep me","project_id":"p1"}"#);
    let project = serde_json::from_str(r#"{"id":"p1","name":"First"}"#).expect("project");
    cache.save("all", vec![task], vec![project], Vec::new(), saved_at);
    Cache::new(dir)
}

#[test]
fn the_pane_opens_on_the_view_it_last_read_and_says_how_old_that_is() {
    let (cold, queue) = stores("opening-warm");
    let mut cache = warm(cold.dir(), 1_000);

    // `opening` takes no connection at all: the first draw cannot wait on a request.
    let (rows, status) = opening(&mut cache, &queue, "all", &Config::default(), 1_300);

    assert_eq!(rows.len(), 2, "a project heading and its task: {rows:?}");
    assert_eq!(rows[1].text().trim(), "keep me");
    assert_eq!(status, "stale 5m");
}

#[test]
fn a_pane_with_no_cache_at_all_opens_empty_and_says_nothing_until_the_first_read() {
    let (mut cache, queue) = stores("opening-cold");

    let (rows, status) = opening(&mut cache, &queue, "all", &Config::default(), 1_300);

    assert!(rows.is_empty());
    assert_eq!(status, "");
}

#[test]
fn a_corrupt_cache_file_opens_the_pane_empty_rather_than_keeping_it_shut() {
    let (cold, queue) = stores("opening-corrupt");
    let mut cache = warm(cold.dir(), 1_000);
    let file = std::fs::read_dir(cold.dir())
        .expect("the cache directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .expect("the view's file");
    std::fs::write(&file, "{\"version\":1,\"saved_at\":").expect("truncate");

    let (rows, status) = opening(&mut cache, &queue, "all", &Config::default(), 1_300);

    assert!(rows.is_empty());
    assert_eq!(status, "");
}

#[tokio::test]
async fn a_successful_read_keeps_the_view_for_the_next_time_the_pane_opens() {
    let double = crate::edit::tests::serve_reading("200 OK", "", &[("/tasks", ONE_TASK)]).await;
    let base_url = double.base_url.clone();
    let config = config_with_token_command("printf test-token");
    let mut connection = Connection::build(&config, &base_url).await;
    let mut list = List::new(Vec::new());
    let mut views = Views::new(&[]);
    let (mut cache, mut queue) = stores("saved");
    let dir = cache.dir().to_path_buf();

    let status = screen(
        &mut connection,
        &config,
        &base_url,
        &mut list,
        &mut views,
        &mut cache,
        &mut queue,
    )
    .show()
    .await;

    assert_eq!(status, "1 open tasks");
    let kept = Cache::new(&dir).load("all").expect("the read was kept");
    assert_eq!(kept.tasks.len(), 1);
    assert_eq!(kept.tasks[0].content, "keep me");
}

#[tokio::test]
async fn a_read_that_cannot_reach_the_api_marks_the_pane_stale_with_the_age_of_what_it_holds() {
    let base_url = unreachable().await;
    let config = config_with_token_command("printf test-token");
    let mut connection = Connection::build(&config, &base_url).await;
    let (cold, _) = stores("stale");
    let mut cache = warm(cold.dir(), crate::cache::now().saturating_sub(3_600));
    let (_, mut queue) = stores("stale-queue");
    let (rows, opened) = opening(&mut cache, &queue, "all", &config, crate::cache::now());
    let mut list = List::new(rows);
    let mut views = Views::new(&[]);

    let status = screen(
        &mut connection,
        &config,
        &base_url,
        &mut list,
        &mut views,
        &mut cache,
        &mut queue,
    )
    .refresh()
    .await;

    assert_eq!(opened, "stale 1h");
    assert_eq!(status, "stale 1h", "the outage message replaced the age");
    assert_eq!(list.task_count(), 1, "the cached rows left the screen");
}

#[tokio::test]
async fn a_read_that_cannot_reach_the_api_with_nothing_cached_says_the_network_is_down() {
    let base_url = unreachable().await;
    let config = config_with_token_command("printf test-token");
    let mut connection = Connection::build(&config, &base_url).await;
    let mut list = List::new(Vec::new());
    let mut views = Views::new(&[]);
    let (mut cache, mut queue) = stores("stale-cold");

    let status = screen(
        &mut connection,
        &config,
        &base_url,
        &mut list,
        &mut views,
        &mut cache,
        &mut queue,
    )
    .refresh()
    .await;

    assert!(status.starts_with("network error:"), "{status}");
}

#[tokio::test]
async fn a_write_made_offline_is_queued_marked_on_its_row_and_still_there_after_a_restart() {
    let base_url = unreachable().await;
    let config = config_with_token_command("printf test-token");
    let mut connection = Connection::build(&config, &base_url).await;
    let mut list = list_of_one();
    let mut views = Views::new(&[]);
    let (mut cache, mut queue) = stores("offline-write");
    let path = queue.path().to_path_buf();

    let status = screen(
        &mut connection,
        &config,
        &base_url,
        &mut list,
        &mut views,
        &mut cache,
        &mut queue,
    )
    .write(Write::Close("1".to_string()))
    .await;

    assert_eq!(status, Ok(crate::queue::Sent::Queued));
    assert_eq!(
        crate::queue::Queue::open(&path).task_ids(),
        vec!["1"],
        "the write did not survive the pane exiting"
    );
    let marked = list.rows()[1].text();
    assert!(marked.starts_with("  + "), "{marked:?}");
    assert!(
        marked.contains("keep me"),
        "the task left the screen: {marked:?}"
    );
}

#[tokio::test]
async fn the_queue_is_sent_in_order_by_the_next_read_that_reaches_the_api() {
    let double = crate::edit::tests::serve("204 No Content", "").await;
    let config = config_with_token_command("printf test-token");
    let mut connection = Connection::build(&config, &double.base_url).await;
    let mut list = list_of_one();
    let mut views = Views::new(&[]);
    let (mut cache, mut queue) = stores("replay");
    queue.push(Write::Close("1".to_string()));
    queue.push(Write::Delete("2".to_string()));
    queue.push(Write::Reopen("3".to_string()));

    let status = screen(
        &mut connection,
        &config,
        &double.base_url,
        &mut list,
        &mut views,
        &mut cache,
        &mut queue,
    )
    .refresh()
    .await;

    assert_eq!(status, "sent 3  0 open tasks");
    assert_eq!(
        double.writes(),
        vec![
            "POST /tasks/1/close".to_string(),
            "DELETE /tasks/2".to_string(),
            "POST /tasks/3/reopen".to_string()
        ]
    );
    assert!(queue.is_empty());
}

#[tokio::test]
async fn x_offline_says_the_completion_is_waiting_rather_than_claiming_it_was_made() {
    let base_url = unreachable().await;
    let config = config_with_token_command("printf test-token");
    let mut connection = Connection::build(&config, &base_url).await;
    let mut list = list_of_one();
    let mut views = Views::new(&[]);
    let (mut cache, mut queue) = stores("offline-key");
    let mut prompt = None;

    let outcome = crate::edit::key(
        crossterm::event::KeyCode::Char('x'),
        &mut screen(
            &mut connection,
            &config,
            &base_url,
            &mut list,
            &mut views,
            &mut cache,
            &mut queue,
        ),
        &mut prompt,
    )
    .await;

    assert_eq!(outcome.status, Some("queued: completed +1".to_string()));
    assert_eq!(list.task_count(), 1, "the task left the screen unsent");
}

#[tokio::test]
async fn switching_view_offline_draws_that_view_s_own_copy_rather_than_the_one_left_on_screen() {
    let base_url = unreachable().await;
    let config = config_with_token_command("printf test-token");
    let mut connection = Connection::build(&config, &base_url).await;
    let (cold, mut queue) = stores("switch");
    let mut cache = Cache::new(cold.dir());
    let project: todoist::Project =
        serde_json::from_str(r#"{"id":"p1","name":"First"}"#).expect("project");
    cache.save(
        "work",
        vec![crate::list::tests::task(
            r#"{"id":"9","content":"in work only","project_id":"p1"}"#,
        )],
        vec![project],
        Vec::new(),
        crate::cache::now().saturating_sub(120),
    );
    let mut list = list_of_one();
    let mut views = Views::new(&[crate::config::View {
        name: "work".to_string(),
        filter: "#Work".to_string(),
    }]);
    views.select(1);

    let status = screen(
        &mut connection,
        &config,
        &base_url,
        &mut list,
        &mut views,
        &mut cache,
        &mut queue,
    )
    .show()
    .await;

    assert_eq!(status, "stale 2m");
    assert_eq!(list.selected_id(), Some("9"), "the old view stayed drawn");
}
