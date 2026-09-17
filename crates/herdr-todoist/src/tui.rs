//! The pane itself: a status line, the task list, and a line of key hints.

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::widgets::ListState;
use todoist::{Client, Error as TodoistError};

use crate::config::Config;
use crate::cursor::List;
use crate::list::{self, Row};
use crate::render::{Status, draw};
use crate::views::{Picker, Views};

/// The connection the pane holds between refreshes: a built client, or the reason building one
/// failed (no token source, a rejected token_command, an unreachable API).
enum Connection {
    Ready(Client),
    Failed(String),
}

pub async fn run(config: &Config, base_url: &str) -> Result<(), String> {
    let mut views = Views::new(&config.views);
    if let Some(name) = crate::pane::take_requested_view() {
        views.select_named(&name);
    }
    let mut connection = build(config, base_url).await;
    let mut list = List::new(Vec::new());
    let mut status = Status(show(&mut connection, config, base_url, &mut list, &views).await);
    let mut picker: Option<Picker> = None;
    let mut row = ListState::default();
    let mut terminal = ratatui::init();
    let outcome = loop {
        if let Err(error) =
            terminal.draw(|frame| draw(frame, &status, &views, &list, &mut row, picker.as_ref()))
        {
            break Err(error.to_string());
        }
        // A `view` action runs as its own process, so its request arrives here rather than as a key.
        if let Some(name) = crate::pane::take_requested_view()
            && views.select_named(&name)
        {
            status = Status(show(&mut connection, config, base_url, &mut list, &views).await);
        }
        let key = match next_key().map_err(|error| error.to_string()) {
            Err(error) => break Err(error),
            Ok(None) => continue,
            Ok(Some(key)) => key,
        };
        match &mut picker {
            Some(open) => match key {
                KeyCode::Char('j') | KeyCode::Down => open.move_cursor(1),
                KeyCode::Char('k') | KeyCode::Up => open.move_cursor(-1),
                KeyCode::Enter => {
                    let chosen = open.at();
                    picker = None;
                    if views.select(chosen) {
                        status = Status(
                            show(&mut connection, config, base_url, &mut list, &views).await,
                        );
                    }
                }
                KeyCode::Esc | KeyCode::Char('q') => picker = None,
                _ => {}
            },
            None => match key {
                KeyCode::Char('q') | KeyCode::Esc => break Ok(()),
                KeyCode::Char('r' | 'R') => {
                    status = Status(
                        check(
                            &mut connection,
                            config,
                            base_url,
                            &mut list,
                            views.current().filter.as_deref(),
                            Change::Refresh,
                        )
                        .await,
                    );
                }
                KeyCode::Char('j') | KeyCode::Down => list.move_cursor(1),
                KeyCode::Char('k') | KeyCode::Up => list.move_cursor(-1),
                KeyCode::Char('v') => picker = Some(Picker::open(&views)),
                KeyCode::Char(digit) => {
                    if let Some(index) = Views::by_number(digit)
                        && views.select(index)
                    {
                        status = Status(
                            show(&mut connection, config, base_url, &mut list, &views).await,
                        );
                    }
                }
                _ => {}
            },
        }
    };
    ratatui::restore();
    outcome
}

/// Resolve the token and build a client, so the pane opens either connected or showing why not.
async fn build(config: &Config, base_url: &str) -> Connection {
    match try_build(config, base_url).await {
        Ok(client) => Connection::Ready(client),
        Err(error) => Connection::Failed(error),
    }
}

async fn try_build(config: &Config, base_url: &str) -> Result<Client, String> {
    let source = config.token_source()?;
    let token = todoist::resolve(&source)
        .await
        .map_err(|error| error.to_string())?;
    Client::new(base_url, token).map_err(|error| error.to_string())
}

/// One outcome of reading the list through the held connection: the rows, a rejected token, or
/// some other failure (a rate limit, a network outage, a build failure carried over from `build`).
enum Probe {
    Loaded(Vec<Row>),
    Rejected,
    Failed(String),
}

async fn probe(connection: &Connection, filter: Option<&str>) -> Probe {
    match connection {
        Connection::Failed(error) => Probe::Failed(error.clone()),
        Connection::Ready(client) => match fetch(client, filter).await {
            Ok(rows) => Probe::Loaded(rows),
            Err(TodoistError::Unauthorized) => Probe::Rejected,
            Err(error) => Probe::Failed(error.to_string()),
        },
    }
}

/// The three lists the view is built from, read at once. `filter` is the showing view's query,
/// absent on the unfiltered list.
async fn fetch(client: &Client, filter: Option<&str>) -> Result<Vec<Row>, TodoistError> {
    let tasks = async {
        match filter {
            Some(query) => client.tasks_matching(query).await,
            None => client.tasks().await,
        }
    };
    let (tasks, projects, sections) =
        tokio::try_join!(tasks, client.projects(), client.sections())?;
    Ok(list::build(&tasks, &projects, &sections))
}

/// Read the list through the held connection and report the status line. A failure leaves the
/// rows already on screen alone and says so in the status line.
///
/// A refresh reuses the same client and connection pool; only a rejected token re-runs
/// `token_command` and rebuilds one, so a `token_command` that prompts for input is not spawned
/// on every keypress.
async fn check(
    connection: &mut Connection,
    config: &Config,
    base_url: &str,
    list: &mut List,
    filter: Option<&str>,
    change: Change,
) -> String {
    let outcome = match probe(connection, filter).await {
        Probe::Rejected => {
            *connection = build(config, base_url).await;
            probe(connection, filter).await
        }
        outcome => outcome,
    };
    match outcome {
        Probe::Loaded(rows) => {
            match change {
                Change::Refresh => list.refresh(rows),
                Change::Switch => list.switch(rows),
            }
            format!("{} open tasks", list.task_count())
        }
        Probe::Rejected => TodoistError::Unauthorized.to_string(),
        Probe::Failed(error) => error,
    }
}

/// Which view the rows belong to, which is what decides where the cursor lands: a refresh of the
/// showing view keeps the cursor near its task, a switch to another view only keeps the task
/// itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Change {
    Refresh,
    Switch,
}

/// Read the showing view and report the status line.
async fn show(
    connection: &mut Connection,
    config: &Config,
    base_url: &str,
    list: &mut List,
    views: &Views,
) -> String {
    check(
        connection,
        config,
        base_url,
        list,
        views.current().filter.as_deref(),
        Change::Switch,
    )
    .await
}

/// A key press, or `None` when the poll window passed with nothing pressed, which keeps the draw
/// loop responsive to a resize.
fn next_key() -> Result<Option<KeyCode>, std::io::Error> {
    if !event::poll(Duration::from_millis(200))? {
        return Ok(None);
    }
    match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => Ok(Some(key.code)),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;
    use crate::config::Config;
    use crate::list::tests::task;

    /// One empty page, which every list endpoint parses.
    const EMPTY_PAGE: &str = r#"{"results":[],"next_cursor":null}"#;

    /// The document Todoist answers a malformed filter query with.
    const REFUSED_FILTER: &str = r#"{"error_tag":"INVALID_ARGUMENT_VALUE","error_code":20,
        "error":"Invalid argument value","http_code":400,
        "error_extra":{"argument":"filter","explanation":"Unable to parse the filter query"}}"#;

    fn config_with_token_command(script: &str) -> Config {
        Config::parse(&format!(r#"token_command = ["sh", "-c", "{script}"]"#)).expect("parses")
    }

    fn list_of_one() -> List {
        let project = serde_json::from_str(r#"{"id":"p1","name":"First"}"#).expect("project");
        List::new(list::build(
            &[task(r#"{"id":"1","content":"keep me","project_id":"p1"}"#)],
            &[project],
            &[],
        ))
    }

    /// A refresh of the unfiltered list, which is what most of these cases are.
    async fn refresh(
        connection: &mut Connection,
        config: &Config,
        base_url: &str,
        list: &mut List,
    ) -> String {
        check(connection, config, base_url, list, None, Change::Refresh).await
    }

    /// A loopback double answering every request the same way, until the listener is dropped.
    async fn serve_forever(status: &'static str, body: &'static str) -> String {
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

    #[tokio::test]
    async fn a_failed_connection_reports_its_error_without_probing_the_network() {
        let mut connection = Connection::Failed("no token source: neither key is set".to_string());
        let config = Config::default();
        let mut list = List::new(Vec::new());
        let message = refresh(&mut connection, &config, "http://127.0.0.1:1", &mut list).await;
        assert_eq!(message, "no token source: neither key is set");
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
        let mut connection = build(&config, &base_url).await;
        let mut list = List::new(Vec::new());

        let first = refresh(&mut connection, &config, &base_url, &mut list).await;
        let second = refresh(&mut connection, &config, &base_url, &mut list).await;

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
        let mut connection = build(&config, &base_url).await;
        let mut list = List::new(Vec::new());

        let message = refresh(&mut connection, &config, &base_url, &mut list).await;

        assert_eq!(message, "unauthorized: the token was rejected");
    }

    #[tokio::test]
    async fn a_rate_limited_refresh_keeps_the_last_good_list_on_screen() {
        let base_url = serve_forever("429 Too Many Requests", "{}").await;
        let config = config_with_token_command("printf test-token");
        let mut connection = build(&config, &base_url).await;
        let mut list = list_of_one();

        let message = refresh(&mut connection, &config, &base_url, &mut list).await;

        assert_eq!(message, "rate limited, retry shortly");
        assert_eq!(list.task_count(), 1);
        assert_eq!(list.selected_id(), Some("1"));
    }

    #[tokio::test]
    async fn a_refused_filter_reports_the_api_s_own_message_and_keeps_the_rows_on_screen() {
        let base_url = serve_forever("400 Bad Request", REFUSED_FILTER).await;
        let config = config_with_token_command("printf test-token");
        let mut connection = build(&config, &base_url).await;
        let mut list = list_of_one();

        let message = check(
            &mut connection,
            &config,
            &base_url,
            &mut list,
            Some("#Work &"),
            Change::Switch,
        )
        .await;

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
        let mut connection = build(&config, &base_url).await;
        let mut list = list_of_one();

        let message = check(
            &mut connection,
            &config,
            &base_url,
            &mut list,
            Some("today & @nobody"),
            Change::Switch,
        )
        .await;

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
        let mut connection = build(&config, &base_url).await;
        let mut list = list_of_one();

        let message = refresh(&mut connection, &config, &base_url, &mut list).await;

        assert!(message.starts_with("network error:"), "{message}");
        assert_eq!(list.task_count(), 1);
    }
}
