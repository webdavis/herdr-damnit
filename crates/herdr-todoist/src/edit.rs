//! The quick edits: one key press each, and where one needs words, one line typed in the pane.
//!
//! Every write is followed by a read of the showing view, so the rows on screen come from the
//! server rather than from a guess at what the write did, and a refused write leaves the rows
//! alone and puts the API's own message in the status line.

use crossterm::event::KeyCode;

use crate::apply::{self, Ask, Edit};
use crate::prompt::Prompt;
use crate::reload::Screen;
use crate::send;
use crate::views::Views;

/// What the pane does after a key press.
#[derive(Debug, PartialEq, Eq)]
pub enum After {
    Stay,
    /// Enter the configured editor on the task under the cursor, in this pane.
    Editor,
    Quit,
    /// Leave the open list for the completed one.
    Completed,
    /// Open the detail of the task under the cursor.
    Detail,
}

/// One key press on the open list, with no prompt open.
pub async fn key(key: KeyCode, screen: &mut Screen<'_>, prompt: &mut Option<Prompt>) -> Outcome {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => Outcome::after(After::Quit),
        KeyCode::Tab | KeyCode::BackTab => Outcome::after(After::Completed),
        KeyCode::Enter => Outcome::after(After::Detail),
        KeyCode::Char('r' | 'R') => Outcome::said(screen.refresh().await),
        KeyCode::Char('j') | KeyCode::Down => {
            screen.list.move_cursor(1);
            Outcome::quiet()
        }
        KeyCode::Char('k') | KeyCode::Up => {
            screen.list.move_cursor(-1);
            Outcome::quiet()
        }
        KeyCode::Char('v') => {
            *prompt = Some(Prompt::views(screen.views));
            Outcome::quiet()
        }
        KeyCode::Char('x') => apply::write(screen, Edit::Complete).await,
        KeyCode::Char('X') => apply::write(screen, Edit::Reopen).await,
        KeyCode::Char('p') => apply::write(screen, Edit::Priority).await,
        KeyCode::Char('d') => apply::open(screen, prompt, Ask::Delete).await,
        KeyCode::Char('s') => apply::open(screen, prompt, Ask::Due).await,
        KeyCode::Char('l') => apply::open(screen, prompt, Ask::Labels).await,
        KeyCode::Char('m') => apply::open(screen, prompt, Ask::Move).await,
        KeyCode::Char('e') => match screen.selected() {
            // With an editor configured the pane hands the terminal over, which only the draw
            // loop can do; with none it opens its own box over the same words.
            Some(task) if crate::editor::argv(screen.config, &task.id).is_some() => {
                Outcome::after(After::Editor)
            }
            Some(task) => {
                *prompt = Some(Prompt::edit(&task.id, &task.content, &task.description));
                Outcome::quiet()
            }
            None => Outcome::quiet(),
        },
        KeyCode::Char('S') => send::ask(screen, prompt),
        KeyCode::Char('a') => {
            *prompt = Some(Prompt::add());
            Outcome::quiet()
        }
        KeyCode::Char(digit) => match Views::by_number(digit) {
            Some(index) if screen.views.select(index) => Outcome::said(screen.show().await),
            _ => Outcome::quiet(),
        },
        _ => Outcome::quiet(),
    }
}

/// One key press while a prompt is open. An input takes every printable key, so only `Esc` leaves
/// it; a picker also leaves on `q`, and the delete confirm leaves on anything but a second `d`.
pub async fn prompt_key(
    key: KeyCode,
    screen: &mut Screen<'_>,
    prompt: &mut Option<Prompt>,
) -> Outcome {
    let Some(open) = prompt else {
        return Outcome::quiet();
    };
    if let Some(input) = open.input_mut() {
        return match key {
            KeyCode::Esc => {
                *prompt = None;
                Outcome::quiet()
            }
            KeyCode::Backspace => {
                input.backspace();
                Outcome::quiet()
            }
            KeyCode::Char(character) => {
                input.push(character);
                Outcome::quiet()
            }
            KeyCode::Enter => apply::send(screen, prompt).await,
            _ => Outcome::quiet(),
        };
    }
    // A multi-line box takes every printable key the way an input does, and `<CR>` opens a line
    // in it rather than sending, so the send key is its own. The two boxes the list opens send
    // different ways: a note goes to an agent pane, a task's own words go to Todoist.
    let to_agent = match open {
        Prompt::Note { .. } => Some(true),
        Prompt::Edit { .. } => Some(false),
        _ => None,
    };
    if let Some(to_agent) = to_agent {
        if key == crate::prompt::SEND {
            return match to_agent {
                true => send::send(screen, prompt, &send::Host::from_env(&send::cli)).await,
                false => apply::send(screen, prompt).await,
            };
        }
        if key == KeyCode::Esc {
            *prompt = None;
            return Outcome::quiet();
        }
        if let Some(draft) = prompt.as_mut().and_then(Prompt::draft_mut) {
            match key {
                KeyCode::Enter => draft.newline(),
                KeyCode::Backspace => draft.backspace(),
                KeyCode::Char(character) => draft.push(character),
                _ => {}
            }
        }
        return Outcome::quiet();
    }
    if let Prompt::Delete { .. } = open {
        // Nothing is sent unless the second `d` arrives: every other key is a dismissal, which
        // makes an accidental `d` cost one keystroke rather than a task.
        return match key {
            KeyCode::Char('d') => apply::send(screen, prompt).await,
            _ => {
                *prompt = None;
                Outcome::quiet()
            }
        };
    }
    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            open.move_cursor(1);
            Outcome::quiet()
        }
        KeyCode::Char('k') | KeyCode::Up => {
            open.move_cursor(-1);
            Outcome::quiet()
        }
        KeyCode::Enter => apply::send(screen, prompt).await,
        KeyCode::Esc | KeyCode::Char('q') => {
            *prompt = None;
            Outcome::quiet()
        }
        _ => Outcome::quiet(),
    }
}

/// What a key press leaves behind: what the pane does next, and the status line when the press
/// had something to say. A press with nothing to say leaves the status line as it was.
pub struct Outcome {
    pub after: After,
    pub status: Option<String>,
}

impl Outcome {
    pub(crate) fn quiet() -> Self {
        Self {
            after: After::Stay,
            status: None,
        }
    }

    pub(crate) fn after(after: After) -> Self {
        Self {
            after,
            status: None,
        }
    }

    pub(crate) fn said(status: String) -> Self {
        Self {
            after: After::Stay,
            status: Some(status),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
