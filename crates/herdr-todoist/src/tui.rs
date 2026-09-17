//! The pane itself: a status line and, until the list view arrives, a placeholder body.

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use todoist::{Client, DEFAULT_BASE_URL};

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

pub async fn run(config: &Config) -> Result<(), String> {
    let mut status = Status(connect(config).await);
    let mut terminal = ratatui::init();
    let outcome = loop {
        if let Err(error) = terminal.draw(|frame| draw(frame, &status)) {
            break Err(error.to_string());
        }
        match next_key().map_err(|error| error.to_string()) {
            Err(error) => break Err(error),
            Ok(Some(KeyCode::Char('q') | KeyCode::Esc)) => break Ok(()),
            Ok(Some(KeyCode::Char('r'))) => status = Status(connect(config).await),
            Ok(_) => {}
        }
    };
    ratatui::restore();
    outcome
}

/// Resolve the token and make one request, so the pane opens either connected or showing why not.
async fn connect(config: &Config) -> String {
    match try_connect(config).await {
        Ok(()) => "connected".to_string(),
        Err(error) => error,
    }
}

async fn try_connect(config: &Config) -> Result<(), String> {
    let source = config.token_source()?;
    let token = todoist::resolve(&source)
        .await
        .map_err(|error| error.to_string())?;
    let client = Client::new(DEFAULT_BASE_URL, token).map_err(|error| error.to_string())?;
    client.user().await.map_err(|error| error.to_string())?;
    Ok(())
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
