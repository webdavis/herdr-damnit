//! The pane itself: a status line, the task list, and a line of key hints.

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::widgets::ListState;

use crate::completed::Completed;
use crate::config::Config;
use crate::connection::Connection;
use crate::cursor::List;
use crate::detail::{After as DetailAfter, Detail};
use crate::edit::{self, After};
use crate::editor;
use crate::prompt::Prompt;
use crate::reload::Screen;
use crate::render::{Chrome, draw, draw_detail};
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
    // The completed list is built on its first use and kept, so leaving and returning does not
    // re-read the API. The detail is not: it is about one task and is read when that task is
    // opened.
    let mut completed: Option<Completed> = None;
    let mut showing = Showing::Open;
    let mut row = ListState::default();
    let palette = crate::theme::resolve(config.theme.as_deref());
    let mut terminal = ratatui::init();
    let outcome = loop {
        let drawn = match (&showing, completed.as_ref()) {
            (Showing::Completed, Some(history)) => Some((history.status(), history.list())),
            _ => None,
        };
        let (drawn_status, drawn_list) = drawn.unwrap_or((status.as_str(), &list));
        let drew = terminal.draw(|frame| match &showing {
            Showing::Detail(detail) => draw_detail(
                frame,
                &Chrome {
                    views: &views,
                    palette: &palette,
                },
                detail,
            ),
            Showing::Open | Showing::Completed => draw(
                frame,
                drawn_status,
                &Chrome {
                    views: &views,
                    palette: &palette,
                },
                drawn_list,
                &mut row,
                prompt.as_ref(),
                matches!(showing, Showing::Completed),
            ),
        });
        if let Err(error) = drew {
            break Err(error.to_string());
        }
        // A `view` action runs as its own process, so its request arrives here rather than as a key.
        if let Some(name) = crate::state::take_requested_view(&crate::state::view_request_path())
            && views.select_named(&name)
        {
            showing = Showing::Open;
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
        if let Showing::Detail(detail) = &mut showing {
            // Leaving the detail draws the list again with its own cursor untouched: nothing here
            // reads or writes the list, so the row that was under the cursor still is.
            match detail.key(key, &mut connection, config, base_url).await {
                DetailAfter::Quit => break Ok(()),
                DetailAfter::Back => showing = Showing::Open,
                DetailAfter::Stay => {}
            }
            continue;
        }
        if let Showing::Completed = showing {
            let was = views.showing();
            let history = completed.as_mut().expect("the completed list is built");
            match completed_key(key, history, &mut connection, config, base_url, &mut views).await {
                After::Quit => break Ok(()),
                After::Completed => showing = Showing::Open,
                // The completed list has no edits, so neither of these is reachable from it.
                After::Detail | After::Editor | After::Stay => {}
            }
            // A number key left the completed list for another view, which has to be read; a Tab
            // back to the view already on screen has nothing to read.
            if matches!(showing, Showing::Open) && views.showing() != was {
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
                showing = Showing::Completed;
            }
            After::Editor => {
                // `enter_editor` only leaves the alternate screen when it has an editor to run,
                // so the terminal is only re-entered on that same path.
                if let Some(said) =
                    enter_editor(&mut connection, config, base_url, &mut list, &mut views).await
                {
                    status = said;
                    terminal = ratatui::init();
                }
            }
            After::Detail => {
                if let Some(opened) = open_detail(&mut connection, config, base_url, &list).await {
                    showing = Showing::Detail(opened);
                }
            }
            After::Stay => {}
        }
    };
    ratatui::restore();
    outcome
}

/// Hand the pane's terminal to the editor and take it back.
///
/// The alternate screen, raw mode and the mouse belong to whoever is drawing, so the pane leaves
/// them before the child starts and the caller enters them again afterwards, on every path: a
/// child that could not be started and one that exited badly both come back here. Reports the
/// status line, or `None` when there was nothing under the cursor to edit.
async fn enter_editor(
    connection: &mut Connection,
    config: &Config,
    base_url: &str,
    list: &mut List,
    views: &mut Views,
) -> Option<String> {
    let argv = editor::argv(config, &list.selected_task()?.id)?;
    ratatui::restore();
    let ran = editor::run(&argv);
    let mut pane = screen(connection, config, base_url, list, views);
    Some(editor::after(ran, &mut pane).await)
}

/// Which of the pane's three screens is on. The completed list and the detail are screens rather
/// than views, the way task 107 settled it: the numbered `view:1` to `view:9` actions and
/// `default_view` name the operator's own filter queries, and neither of these has a filter.
enum Showing {
    Open,
    Completed,
    Detail(Detail),
}

/// Open the detail of the task under the cursor. The cursor on a heading, or on nothing at all,
/// has no task to open, so the list stays on screen.
async fn open_detail(
    connection: &mut Connection,
    config: &Config,
    base_url: &str,
    list: &List,
) -> Option<Detail> {
    let task = list.selected_task()?;
    Some(
        Detail::open(
            connection,
            config,
            base_url,
            &task.id,
            &task.content,
            &task.description,
        )
        .await,
    )
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
        Event::Key(key) if key.kind == KeyEventKind::Press => Ok(Some(fold(key))),
        _ => Ok(None),
    }
}

/// Fold a Ctrl-held letter into the control character a terminal itself would send, since
/// crossterm hands back the letter and the modifier separately. This is what turns Ctrl-D into
/// [`crate::prompt::SEND`]: everywhere past this point the pane's only currency is a bare
/// `KeyCode`.
fn fold(key: KeyEvent) -> KeyCode {
    match key.code {
        KeyCode::Char(character) if key.modifiers.contains(KeyModifiers::CONTROL) => {
            match character.to_ascii_lowercase() {
                letter @ 'a'..='z' => KeyCode::Char((letter as u8 - b'a' + 1) as char),
                _ => key.code,
            }
        }
        _ => key.code,
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
}
