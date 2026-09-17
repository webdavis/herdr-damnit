//! The pane itself: a status line and, until the list view arrives, a placeholder body.

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use todoist::{Client, Error as TodoistError};

use crate::config::Config;

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
    let mut status = Status(check(&mut connection, config, base_url).await);
    let mut terminal = ratatui::init();
    let outcome = loop {
        if let Err(error) = terminal.draw(|frame| draw(frame, &status)) {
            break Err(error.to_string());
        }
        match next_key().map_err(|error| error.to_string()) {
            Err(error) => break Err(error),
            Ok(Some(KeyCode::Char('q') | KeyCode::Esc)) => break Ok(()),
            Ok(Some(KeyCode::Char('r'))) => {
                status = Status(check(&mut connection, config, base_url).await);
            }
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

/// One outcome of probing the held connection: connected, the token was rejected, or some other
/// failure (a rate limit, a network outage, a build failure carried over from `build`).
enum Probe {
    Connected,
    Rejected,
    Failed(String),
}

async fn probe(connection: &Connection) -> Probe {
    match connection {
        Connection::Failed(error) => Probe::Failed(error.clone()),
        Connection::Ready(client) => match client.user().await {
            Ok(_) => Probe::Connected,
            Err(TodoistError::Unauthorized) => Probe::Rejected,
            Err(error) => Probe::Failed(error.to_string()),
        },
    }
}

/// Check the held connection and report its status. A refresh reuses the same client and
/// connection pool; only a rejected token re-runs `token_command` and rebuilds one, so a
/// `token_command` that prompts for input is not spawned on every keypress.
async fn check(connection: &mut Connection, config: &Config, base_url: &str) -> String {
    let outcome = match probe(connection).await {
        Probe::Rejected => {
            *connection = build(config, base_url).await;
            probe(connection).await
        }
        outcome => outcome,
    };
    match outcome {
        Probe::Connected => "connected".to_string(),
        Probe::Rejected => TodoistError::Unauthorized.to_string(),
        Probe::Failed(error) => error,
    }
}

fn draw(frame: &mut ratatui::Frame<'_>, status: &Status) {
    let [status_area, body_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(frame.area());
    frame.render_widget(Paragraph::new(status.line()), status_area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from("The task list arrives in the next task."),
            Line::from(""),
            Line::from("r  refresh"),
            Line::from("q  close the pane"),
        ])
        .block(Block::default().borders(Borders::ALL)),
        body_area,
    );
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

    fn config_with_token_command(script: &str) -> Config {
        Config::parse(&format!(r#"token_command = ["sh", "-c", "{script}"]"#)).expect("parses")
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
        let message = check(&mut connection, &config, "http://127.0.0.1:1").await;
        assert_eq!(message, "no token source: neither key is set");
    }

    #[tokio::test]
    async fn a_refresh_reuses_the_client_instead_of_rerunning_token_command() {
        let counter = std::env::temp_dir().join(format!(
            "herdr-todoist-token-command-runs-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&counter);
        let base_url = serve_forever("200 OK", r#"{"id":"1","email":null,"full_name":null}"#).await;
        let config = config_with_token_command(&format!(
            "printf x >> {} && printf test-token",
            counter.display()
        ));
        let mut connection = build(&config, &base_url).await;

        let first = check(&mut connection, &config, &base_url).await;
        let second = check(&mut connection, &config, &base_url).await;

        assert_eq!(first, "connected");
        assert_eq!(second, "connected");
        let runs = std::fs::read_to_string(&counter).expect("counter file");
        let _ = std::fs::remove_file(&counter);
        assert_eq!(runs, "x", "token_command ran more than once: {runs:?}");
    }

    #[tokio::test]
    async fn a_rejected_token_is_reported_after_rebuilding_once() {
        let base_url = serve_forever("401 Unauthorized", "{}").await;
        let config = config_with_token_command("printf test-token");
        let mut connection = build(&config, &base_url).await;

        let message = check(&mut connection, &config, &base_url).await;

        assert_eq!(message, "unauthorized: the token was rejected");
    }
}
