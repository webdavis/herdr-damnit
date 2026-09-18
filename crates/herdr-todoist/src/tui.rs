//! The pane itself: a status line, the task list, and a line of key hints.

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::widgets::ListState;

use crate::cache::{self, Cache};
use crate::completed::Completed;
use crate::config::Config;
use crate::connection::Connection;
use crate::cursor::List;
use crate::detail::{After as DetailAfter, Detail};
use crate::edit::{self, After};
use crate::editor;
use crate::prompt::Prompt;
use crate::queue::Queue;
use crate::refresh::Schedule;
use crate::reload::{self, Screen};
use crate::render::{Chrome, draw, draw_detail};
use crate::views::Views;

/// Everything a key press borrows to act: the held connection, the rows on screen, the views, the
/// local copy of each view and the writes still waiting for the network. One struct so a call
/// that needs several of them takes one `&mut` instead of one argument per field.
struct Pane {
    connection: Connection,
    list: List,
    views: Views,
    cache: Cache,
    queue: Queue,
}

pub async fn run(config: &Config, base_url: &str) -> Result<(), String> {
    let mut views = Views::new(&config.views);
    // A `view` action's request outranks the configured opening view: it is the later word, and
    // the one a key was just pressed for.
    if let Some(name) = opening_view(config, &crate::state::view_request_path()) {
        views.select_named(&name);
    }
    let view = views.current().name.clone();
    let (mut pane, mut schedule, mut status) = open(
        config,
        base_url,
        &view,
        views,
        Cache::in_state_dir(),
        Queue::in_state_dir(),
    )
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
        let (drawn_status, drawn_list) = drawn.unwrap_or((status.as_str(), &pane.list));
        let drew = terminal.draw(|frame| match &showing {
            Showing::Detail(detail) => draw_detail(
                frame,
                &Chrome {
                    views: &pane.views,
                    palette: &palette,
                },
                detail,
            ),
            Showing::Open | Showing::Completed => draw(
                frame,
                drawn_status,
                &Chrome {
                    views: &pane.views,
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
            && pane.views.select_named(&name)
        {
            showing = Showing::Open;
            prompt = None;
            status = screen(&mut pane, config, base_url).show().await;
        }
        // The interval refresh. It is held back while a prompt is open or another screen is on,
        // so a half-typed line is never redrawn away, and it is driven from this loop alone,
        // which is why it cannot outlive the pane.
        let busy = prompt.is_some() || !matches!(showing, Showing::Open);
        if schedule.due(cache::now(), busy) {
            status = screen(&mut pane, config, base_url).refresh().await;
            schedule.mark(cache::now());
            continue;
        }
        let key = match next_key().map_err(|error| error.to_string()) {
            Err(error) => break Err(error),
            Ok(None) => continue,
            Ok(Some(key)) => key,
        };
        if let Showing::Detail(detail) = &mut showing {
            // Leaving the detail draws the list again with its own cursor untouched: nothing here
            // reads or writes the list, so the row that was under the cursor still is.
            match detail
                .key(key, &mut pane.connection, config, base_url)
                .await
            {
                DetailAfter::Quit => break Ok(()),
                DetailAfter::Back => showing = Showing::Open,
                DetailAfter::Stay => {}
            }
            continue;
        }
        if let Showing::Completed = showing {
            let was = pane.views.showing();
            let history = completed.as_mut().expect("the completed list is built");
            match completed_key(
                key,
                history,
                &mut pane.connection,
                config,
                base_url,
                &mut pane.views,
            )
            .await
            {
                After::Quit => break Ok(()),
                After::Completed => showing = Showing::Open,
                // The completed list has no edits, so neither of these is reachable from it.
                After::Detail | After::Editor | After::Stay => {}
            }
            // A number key left the completed list for another view, which has to be read; a Tab
            // back to the view already on screen has nothing to read.
            if matches!(showing, Showing::Open) && pane.views.showing() != was {
                status = screen(&mut pane, config, base_url).show().await;
            }
            continue;
        }
        let mut acting = screen(&mut pane, config, base_url);
        let acted = if prompt.is_some() {
            edit::prompt_key(key, &mut acting, &mut prompt).await
        } else {
            edit::key(key, &mut acting, &mut prompt).await
        };
        if let Some(said) = acted.status {
            status = said;
            // A key press that read the API puts the interval back to a full interval away.
            schedule.mark(cache::now());
        }
        match acted.after {
            After::Quit => break Ok(()),
            After::Completed => {
                if completed.is_none() {
                    completed = Some(Completed::open(&mut pane.connection, config, base_url).await);
                }
                showing = Showing::Completed;
            }
            After::Editor => {
                // `enter_editor` only leaves the alternate screen when it has an editor to run,
                // so the terminal is only re-entered on that same path.
                if let Some(said) = enter_editor(&mut pane, config, base_url).await {
                    status = said;
                    terminal = ratatui::init();
                }
            }
            After::Detail => {
                if let Some(opened) =
                    open_detail(&mut pane.connection, config, base_url, &pane.list).await
                {
                    showing = Showing::Detail(opened);
                }
            }
            After::Stay => {}
        }
    };
    ratatui::restore();
    outcome
}

/// Build the pane's held state and read once before the first draw, whatever `refresh_seconds`
/// is: it turns off the interval that follows, never the read that proves the cache right or
/// wrong the moment the pane opens.
async fn open(
    config: &Config,
    base_url: &str,
    view: &str,
    views: Views,
    mut cache: Cache,
    queue: Queue,
) -> (Pane, Schedule, String) {
    let (rows, _) = reload::opening(&mut cache, &queue, view, config, cache::now());
    let connection = Connection::build(config, base_url).await;
    let mut pane = Pane {
        connection,
        list: List::new(rows),
        views,
        cache,
        queue,
    };
    let status = screen(&mut pane, config, base_url).refresh().await;
    let mut schedule = Schedule::new(config.refresh_seconds(), cache::now());
    schedule.mark(cache::now());
    (pane, schedule, status)
}

/// Hand the pane's terminal to the editor and take it back.
///
/// The alternate screen, raw mode and the mouse belong to whoever is drawing, so the pane leaves
/// them before the child starts and the caller enters them again afterwards, on every path: a
/// child that could not be started and one that exited badly both come back here. Reports the
/// status line, or `None` when there was nothing under the cursor to edit.
async fn enter_editor(pane: &mut Pane, config: &Config, base_url: &str) -> Option<String> {
    let argv = editor::argv(config, &pane.list.selected_task()?.id)?;
    ratatui::restore();
    let ran = editor::run(&argv);
    let mut acting = screen(pane, config, base_url);
    Some(editor::after(ran, &mut acting).await)
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

fn screen<'a>(pane: &'a mut Pane, config: &'a Config, base_url: &'a str) -> Screen<'a> {
    Screen {
        connection: &mut pane.connection,
        config,
        base_url,
        list: &mut pane.list,
        views: &mut pane.views,
        cache: &mut pane.cache,
        queue: &mut pane.queue,
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
mod tests;
