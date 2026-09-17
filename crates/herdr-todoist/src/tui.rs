//! The pane itself: a status line, the task list, and a line of key hints.

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{ListItem, ListState, Paragraph};
use todoist::{Client, Error as TodoistError};

use crate::config::Config;
use crate::cursor::List;
use crate::list::{self, Row};

const HINTS: &str = "j/k move   R refresh   q close";

/// What the status line says. Every failure the client can report (a rejected token, a rate limit
/// with its retry delay, a network outage) arrives here as its own message.
struct Status(String);

impl Status {
    fn line(&self) -> Line<'_> {
        Line::from(vec![
            Span::styled("todoist", Style::new().add_modifier(Modifier::BOLD)),
            Span::raw(format!("  {}", self.0)),
        ])
    }
}

/// The connection the pane holds between refreshes: a built client, or the reason building one
/// failed (no token source, a rejected token_command, an unreachable API).
enum Connection {
    Ready(Client),
    Failed(String),
}

pub async fn run(config: &Config, base_url: &str) -> Result<(), String> {
    let mut connection = build(config, base_url).await;
    let mut list = List::new(Vec::new());
    let mut status = Status(check(&mut connection, config, base_url, &mut list).await);
    let mut view = ListState::default();
    let mut terminal = ratatui::init();
    let outcome = loop {
        if let Err(error) = terminal.draw(|frame| draw(frame, &status, &list, &mut view)) {
            break Err(error.to_string());
        }
        match next_key().map_err(|error| error.to_string()) {
            Err(error) => break Err(error),
            Ok(Some(KeyCode::Char('q') | KeyCode::Esc)) => break Ok(()),
            Ok(Some(KeyCode::Char('r' | 'R'))) => {
                status = Status(check(&mut connection, config, base_url, &mut list).await);
            }
            Ok(Some(KeyCode::Char('j') | KeyCode::Down)) => list.move_cursor(1),
            Ok(Some(KeyCode::Char('k') | KeyCode::Up)) => list.move_cursor(-1),
            Ok(_) => {}
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

async fn probe(connection: &Connection) -> Probe {
    match connection {
        Connection::Failed(error) => Probe::Failed(error.clone()),
        Connection::Ready(client) => match fetch(client).await {
            Ok(rows) => Probe::Loaded(rows),
            Err(TodoistError::Unauthorized) => Probe::Rejected,
            Err(error) => Probe::Failed(error.to_string()),
        },
    }
}

/// The three lists the view is built from, read at once.
async fn fetch(client: &Client) -> Result<Vec<Row>, TodoistError> {
    let (tasks, projects, sections) =
        tokio::try_join!(client.tasks(), client.projects(), client.sections())?;
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
) -> String {
    let outcome = match probe(connection).await {
        Probe::Rejected => {
            *connection = build(config, base_url).await;
            probe(connection).await
        }
        outcome => outcome,
    };
    match outcome {
        Probe::Loaded(rows) => {
            list.refresh(rows);
            format!("{} open tasks", list.task_count())
        }
        Probe::Rejected => TodoistError::Unauthorized.to_string(),
        Probe::Failed(error) => error,
    }
}

fn draw(frame: &mut ratatui::Frame<'_>, status: &Status, list: &List, view: &mut ListState) {
    let [status_area, body_area, hint_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    frame.render_widget(Paragraph::new(status.line()), status_area);
    view.select(list.selected_id().map(|_| list.selected()));
    frame.render_stateful_widget(
        ratatui::widgets::List::new(list.rows().iter().map(item))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED)),
        body_area,
        view,
    );
    frame.render_widget(Paragraph::new(Line::from(HINTS)), hint_area);
}

/// A heading stands out in bold; a task line is drawn as it was built.
fn item(row: &Row) -> ListItem<'_> {
    let style = match row {
        Row::Header(_) => Style::new().add_modifier(Modifier::BOLD),
        Row::Task(_) => Style::new(),
    };
    ListItem::new(Line::styled(row.text(), style))
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

    fn list_of_two() -> List {
        let project = serde_json::from_str(r#"{"id":"p1","name":"First"}"#).expect("project");
        List::new(list::build(
            &[
                task(r#"{"id":"1","content":"first","project_id":"p1","child_order":1}"#),
                task(r#"{"id":"2","content":"second","project_id":"p1","child_order":2}"#),
            ],
            &[project],
            &[],
        ))
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

    /// Draw into an off-screen terminal and report which line came out highlighted.
    fn highlighted_line(list: &List) -> String {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(40, 6)).expect("terminal");
        let mut view = ListState::default();
        terminal
            .draw(|frame| draw(frame, &Status("2 open tasks".to_string()), list, &mut view))
            .expect("draw");
        let buffer = terminal.backend().buffer().clone();
        let mut line = String::new();
        for row in 0..buffer.area.height {
            for column in 0..buffer.area.width {
                let cell = &buffer[(column, row)];
                if cell.modifier.contains(Modifier::REVERSED) {
                    line.push_str(cell.symbol());
                }
            }
        }
        line.trim().to_string()
    }

    #[test]
    fn the_highlight_is_drawn_on_the_selected_task() {
        let mut list = list_of_two();

        assert_eq!(highlighted_line(&list), "first");
        list.move_cursor(1);
        assert_eq!(highlighted_line(&list), "second");
    }

    #[test]
    fn an_empty_list_draws_no_highlight() {
        assert_eq!(highlighted_line(&List::new(Vec::new())), "");
    }

    #[tokio::test]
    async fn a_failed_connection_reports_its_error_without_probing_the_network() {
        let mut connection = Connection::Failed("no token source: neither key is set".to_string());
        let config = Config::default();
        let mut list = List::new(Vec::new());
        let message = check(&mut connection, &config, "http://127.0.0.1:1", &mut list).await;
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

        let first = check(&mut connection, &config, &base_url, &mut list).await;
        let second = check(&mut connection, &config, &base_url, &mut list).await;

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

        let message = check(&mut connection, &config, &base_url, &mut list).await;

        assert_eq!(message, "unauthorized: the token was rejected");
    }

    #[tokio::test]
    async fn a_rate_limited_refresh_keeps_the_last_good_list_on_screen() {
        let base_url = serve_forever("429 Too Many Requests", "{}").await;
        let config = config_with_token_command("printf test-token");
        let mut connection = build(&config, &base_url).await;
        let mut list = list_of_one();

        let message = check(&mut connection, &config, &base_url, &mut list).await;

        assert_eq!(message, "rate limited, retry shortly");
        assert_eq!(list.task_count(), 1);
        assert_eq!(list.selected_id(), Some("1"));
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

        let message = check(&mut connection, &config, &base_url, &mut list).await;

        assert!(message.starts_with("network error:"), "{message}");
        assert_eq!(list.task_count(), 1);
    }
}
