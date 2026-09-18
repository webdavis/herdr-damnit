use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::*;
use crate::list::tests::task;

mod offline;

/// One empty page, which every list endpoint parses.
pub(crate) const EMPTY_PAGE: &str = r#"{"results":[],"next_cursor":null}"#;

/// The document Todoist answers a malformed filter query with.
const REFUSED_FILTER: &str = r#"{"error_tag":"INVALID_ARGUMENT_VALUE","error_code":20,
    "error":"Invalid argument value","http_code":400,
    "error_extra":{"argument":"filter","explanation":"Unable to parse the filter query"}}"#;

pub(crate) fn config_with_token_command(script: &str) -> Config {
    Config::parse(&format!(r#"token_command = ["sh", "-c", "{script}"]"#)).expect("parses")
}

pub(crate) fn list_of_one() -> List {
    let project = serde_json::from_str(r#"{"id":"p1","name":"First"}"#).expect("project");
    List::new(list::build(
        &[task(r#"{"id":"1","content":"keep me","project_id":"p1"}"#)],
        &[project],
        &[],
        &crate::list::tests::marks(),
    ))
}

/// A loopback double answering every request the same way, until the listener is dropped.
pub(crate) async fn serve_forever(status: &'static str, body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let base_url = format!("http://{}", listener.local_addr().expect("addr"));
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut buffer = [0u8; 1024];
            let _ = socket.read(&mut buffer).await;
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        }
    });
    base_url
}

/// A cache directory and a queue file of this test's own, both empty, so no test reads what
/// another wrote and none of them reaches the operator's own state directory.
pub(crate) fn stores(name: &str) -> (Cache, Queue) {
    let dir = std::env::temp_dir().join(format!(
        "herdr-todoist-store-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    crate::cache::clear(&dir);
    let queue = dir.join("queue.json");
    crate::queue::clear(&queue);
    (Cache::new(dir), Queue::open(queue))
}

/// A screen over the given rows, reading a double at `base_url`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn screen<'a>(
    connection: &'a mut Connection,
    config: &'a Config,
    base_url: &'a str,
    list: &'a mut List,
    views: &'a mut Views,
    cache: &'a mut Cache,
    queue: &'a mut Queue,
) -> Screen<'a> {
    Screen {
        connection,
        config,
        base_url,
        list,
        views,
        cache,
        queue,
    }
}

#[tokio::test]
async fn a_failed_connection_reports_its_error_without_probing_the_network() {
    let mut connection = Connection::Failed("no token source: neither key is set".to_string());
    let config = Config::default();
    let mut list = List::new(Vec::new());
    let mut views = Views::new(&[]);
    let (mut cache, mut queue) = stores("screen");
    let mut screen = screen(
        &mut connection,
        &config,
        "http://127.0.0.1:1",
        &mut list,
        &mut views,
        &mut cache,
        &mut queue,
    );

    assert_eq!(
        screen.refresh().await,
        "no token source: neither key is set"
    );
}

#[tokio::test]
async fn a_refresh_reuses_the_client_instead_of_rerunning_token_command() {
    let counter = std::env::temp_dir().join(format!(
        "herdr-todoist-token-command-runs-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&counter);
    let base_url = serve_forever("200 OK", EMPTY_PAGE).await;
    let config = config_with_token_command(&format!(
        "printf x >> {} && printf test-token",
        counter.display()
    ));
    let mut connection = Connection::build(&config, &base_url).await;
    let mut list = List::new(Vec::new());
    let mut views = Views::new(&[]);
    let (mut cache, mut queue) = stores("screen");
    let mut screen = screen(
        &mut connection,
        &config,
        &base_url,
        &mut list,
        &mut views,
        &mut cache,
        &mut queue,
    );

    let first = screen.refresh().await;
    let second = screen.refresh().await;

    assert_eq!(first, "0 open tasks");
    assert_eq!(second, "0 open tasks");
    let runs = std::fs::read_to_string(&counter).expect("counter file");
    let _ = std::fs::remove_file(&counter);
    assert_eq!(runs, "x", "token_command ran more than once: {runs:?}");
}

#[tokio::test]
async fn a_rejected_token_is_reported_after_rebuilding_once() {
    let base_url = serve_forever("401 Unauthorized", "{}").await;
    let config = config_with_token_command("printf test-token");
    let mut connection = Connection::build(&config, &base_url).await;
    let mut list = List::new(Vec::new());
    let mut views = Views::new(&[]);
    let (mut cache, mut queue) = stores("screen");
    let mut screen = screen(
        &mut connection,
        &config,
        &base_url,
        &mut list,
        &mut views,
        &mut cache,
        &mut queue,
    );

    assert_eq!(
        screen.refresh().await,
        "unauthorized: the token was rejected"
    );
}

#[tokio::test]
async fn a_rate_limited_refresh_keeps_the_last_good_list_on_screen() {
    let base_url = serve_forever("429 Too Many Requests", "{}").await;
    let config = config_with_token_command("printf test-token");
    let mut connection = Connection::build(&config, &base_url).await;
    let mut list = list_of_one();
    let mut views = Views::new(&[]);
    let (mut cache, mut queue) = stores("screen");
    let mut screen = screen(
        &mut connection,
        &config,
        &base_url,
        &mut list,
        &mut views,
        &mut cache,
        &mut queue,
    );

    let message = screen.refresh().await;

    assert_eq!(message, "rate limited, retry shortly");
    assert_eq!(list.task_count(), 1);
    assert_eq!(list.selected_id(), Some("1"));
}

#[tokio::test]
async fn a_refused_filter_reports_the_api_s_own_message_and_keeps_the_rows_on_screen() {
    let base_url = serve_forever("400 Bad Request", REFUSED_FILTER).await;
    let config = config_with_token_command("printf test-token");
    let mut connection = Connection::build(&config, &base_url).await;
    let mut list = list_of_one();
    let mut views = Views::new(&[crate::config::View {
        name: "work".to_string(),
        filter: "#Work &".to_string(),
    }]);
    let (mut cache, mut queue) = stores("screen");
    views.select(1);
    let mut screen = screen(
        &mut connection,
        &config,
        &base_url,
        &mut list,
        &mut views,
        &mut cache,
        &mut queue,
    );

    let message = screen.show().await;

    assert_eq!(
        message,
        "Invalid argument value: Unable to parse the filter query"
    );
    assert_eq!(list.task_count(), 1);
    assert_eq!(list.selected_id(), Some("1"));
}

#[tokio::test]
async fn a_filter_matching_nothing_empties_the_list_instead_of_reporting_a_failure() {
    let base_url = serve_forever("200 OK", EMPTY_PAGE).await;
    let config = config_with_token_command("printf test-token");
    let mut connection = Connection::build(&config, &base_url).await;
    let mut list = list_of_one();
    let mut views = Views::new(&[crate::config::View {
        name: "nobody".to_string(),
        filter: "today & @nobody".to_string(),
    }]);
    let (mut cache, mut queue) = stores("screen");
    views.select(1);
    let mut screen = screen(
        &mut connection,
        &config,
        &base_url,
        &mut list,
        &mut views,
        &mut cache,
        &mut queue,
    );

    let message = screen.show().await;

    assert_eq!(message, "0 open tasks");
    assert_eq!(list.task_count(), 0);
}

#[tokio::test]
async fn a_network_failure_keeps_the_last_good_list_on_screen() {
    let config = config_with_token_command("printf test-token");
    let base_url = {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("addr");
        format!("http://{address}")
    };
    let mut connection = Connection::build(&config, &base_url).await;
    let mut list = list_of_one();
    let mut views = Views::new(&[]);
    let (mut cache, mut queue) = stores("screen");
    let mut screen = screen(
        &mut connection,
        &config,
        &base_url,
        &mut list,
        &mut views,
        &mut cache,
        &mut queue,
    );

    let message = screen.refresh().await;

    assert!(message.starts_with("network error:"), "{message}");
    assert_eq!(list.task_count(), 1);
}
