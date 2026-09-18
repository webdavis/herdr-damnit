//! The pane itself: a status line, the task list, and a line of key hints.

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::widgets::ListState;

use crate::completed::Completed;
use crate::config::Config;
use crate::connection::Connection;
use crate::cursor::List;
use crate::edit::{self, After};
use crate::prompt::Prompt;
use crate::reload::Screen;
use crate::render::draw;
use crate::views::Views;

pub async fn run(config: &Config, base_url: &str) -> Result<(), String> {
    let mut views = Views::new(&config.views);
    // A `view` action's request outranks the configured opening view: it is the later word, and
    // the one a key was just pressed for.
    if let Some(name) = opening_view(config, &crate::state::view_request_path()) {
        views.select_named(&name);
    }
    let mut connection = Connection::build(config, base_url).await;
    let mut list = List::new(Vec::new());
    let mut status = screen(&mut connection, config, base_url, &mut list, &mut views)
        .show()
        .await;
    let mut prompt: Option<Prompt> = None;
    // The completed list is a second screen rather than a tenth view: the numbered views are the
    // operator's own filter queries, and this one has no filter, no grouping and keys of its own.
    // It is built on its first use and kept, so leaving and returning does not re-read the API.
    let mut completed: Option<Completed> = None;
    let mut showing_completed = false;
    let mut row = ListState::default();
    let mut terminal = ratatui::init();
    let outcome = loop {
        let showing = completed
            .as_ref()
            .filter(|_| showing_completed)
            .map(|history| (history.status(), history.list()));
        let (drawn_status, drawn_list) = showing.unwrap_or((status.as_str(), &list));
        if let Err(error) = terminal.draw(|frame| {
            draw(
                frame,
                drawn_status,
                &views,
                drawn_list,
                &mut row,
                prompt.as_ref(),
                showing_completed,
            )
        }) {
            break Err(error.to_string());
        }
        // A `view` action runs as its own process, so its request arrives here rather than as a key.
        if let Some(name) = crate::state::take_requested_view(&crate::state::view_request_path())
            && views.select_named(&name)
        {
            showing_completed = false;
            prompt = None;
            status = screen(&mut connection, config, base_url, &mut list, &mut views)
                .show()
                .await;
        }
        let key = match next_key().map_err(|error| error.to_string()) {
            Err(error) => break Err(error),
            Ok(None) => continue,
            Ok(Some(key)) => key,
        };
        if showing_completed {
            let showing = views.showing();
            let history = completed.as_mut().expect("the completed list is built");
            match completed_key(key, history, &mut connection, config, base_url, &mut views).await {
                After::Quit => break Ok(()),
                After::Completed => showing_completed = false,
                After::Stay => {}
            }
            // A number key left the completed list for another view, which has to be read; a Tab
            // back to the view already on screen has nothing to read.
            if !showing_completed && views.showing() != showing {
                status = screen(&mut connection, config, base_url, &mut list, &mut views)
                    .show()
                    .await;
            }
            continue;
        }
        let mut pane = screen(&mut connection, config, base_url, &mut list, &mut views);
        let acted = if prompt.is_some() {
            edit::prompt_key(key, &mut pane, &mut prompt).await
        } else {
            edit::key(key, &mut pane, &mut prompt).await
        };
        if let Some(said) = acted.status {
            status = said;
        }
        match acted.after {
            After::Quit => break Ok(()),
            After::Completed => {
                if completed.is_none() {
                    completed = Some(Completed::open(&mut connection, config, base_url).await);
                }
                showing_completed = true;
            }
            After::Stay => {}
        }
    };
    ratatui::restore();
    outcome
}

fn screen<'a>(
    connection: &'a mut Connection,
    config: &'a Config,
    base_url: &'a str,
    list: &'a mut List,
    views: &'a mut Views,
) -> Screen<'a> {
    Screen {
        connection,
        config,
        base_url,
        list,
        views,
    }
}

/// One key press on the completed list. `u` and `X` both reopen, since `X` reopens wherever the
/// cursor is, and a number key leaves for that view the way it does on the open list.
async fn completed_key(
    key: KeyCode,
    completed: &mut Completed,
    connection: &mut Connection,
    config: &Config,
    base_url: &str,
    views: &mut Views,
) -> After {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => After::Quit,
        KeyCode::Tab | KeyCode::BackTab => After::Completed,
        KeyCode::Char('j') | KeyCode::Down => {
            completed.down(connection, config, base_url).await;
            After::Stay
        }
        KeyCode::Char('k') | KeyCode::Up => {
            completed.up();
            After::Stay
        }
        KeyCode::Char('u' | 'X') => {
            completed.reopen(connection, config, base_url).await;
            After::Stay
        }
        KeyCode::Char('r' | 'R') => {
            *completed = Completed::open(connection, config, base_url).await;
            After::Stay
        }
        // A number key names an open-task view, so it leaves the completed list for that view,
        // and a number with no view behind it does nothing here either.
        KeyCode::Char(digit) => match Views::by_number(digit).filter(|at| *at < views.len()) {
            Some(index) => {
                views.select(index);
                After::Completed
            }
            None => After::Stay,
        },
        _ => After::Stay,
    }
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

/// The view the pane opens on: the one a `view` action asked for, else the configured opening
/// view, else the unfiltered list.
fn opening_view(config: &Config, request: &std::path::Path) -> Option<String> {
    crate::state::take_requested_view(request).or_else(|| config.default_view.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
