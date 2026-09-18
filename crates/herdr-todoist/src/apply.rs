//! What each quick edit sends, and what the pane says once the API has answered.
//!
//! Every write is followed by a read of the showing view, so the rows on screen come from the
//! server rather than from a guess at what the write did, and a refused write leaves the rows
//! alone with the API's own message in the status line.

use todoist::{Change, Client, Destination};

use crate::queue::{Sent, Write as Queued};

use crate::edit::Outcome;
use crate::list::{HIGHEST_PRIORITY, LOWEST_PRIORITY};
use crate::prompt::{self, Prompt};
use crate::reload::Screen;

/// A write that needs no words: the task under the cursor is all of the request.
pub enum Edit {
    Complete,
    Reopen,
    Priority,
}

/// A prompt that has to read the API before it can be drawn, or has nothing to read at all.
pub enum Ask {
    Delete,
    Due,
    Labels,
    Move,
}

/// The task under the cursor, as much of it as a quick edit needs.
struct Under {
    id: String,
    content: String,
    priority: u8,
    labels: Vec<String>,
}

fn under_cursor(screen: &Screen<'_>) -> Option<Under> {
    screen.selected().map(|task| Under {
        id: task.id.clone(),
        content: task.content.clone(),
        priority: task.priority,
        labels: task.labels.clone(),
    })
}

pub async fn write(screen: &mut Screen<'_>, edit: Edit) -> Outcome {
    let Some(task) = under_cursor(screen) else {
        return Outcome::quiet();
    };
    let id = task.id;
    let (done, queued) = match edit {
        Edit::Complete => ("completed".to_string(), Queued::Close(id)),
        Edit::Reopen => ("reopened".to_string(), Queued::Reopen(id)),
        Edit::Priority => {
            let next = cycled(task.priority);
            (
                format!("p{}", HIGHEST_PRIORITY + LOWEST_PRIORITY - next),
                Queued::Update {
                    id,
                    change: priority(next),
                },
            )
        }
    };
    let sent = screen.write(queued).await;
    Outcome::said(reported(screen, done, sent).await)
}

fn priority(priority: u8) -> Change {
    Change {
        priority: Some(priority),
        ..Default::default()
    }
}

/// Open a prompt about the task under the cursor. The two pickers read what they offer first, so
/// a failed read, or one with nothing to offer, says so in the status line and leaves the pane as
/// it was rather than drawing an empty picker.
pub async fn open(screen: &mut Screen<'_>, prompt: &mut Option<Prompt>, ask: Ask) -> Outcome {
    let Some(task) = under_cursor(screen) else {
        return Outcome::quiet();
    };
    match ask {
        Ask::Delete => *prompt = Some(Prompt::delete(&task.id, &task.content)),
        Ask::Due => *prompt = Some(Prompt::due(&task.id)),
        Ask::Labels => {
            match screen
                .request(async |client: &Client| client.labels().await)
                .await
            {
                Ok(known) if known.is_empty() && task.labels.is_empty() => {
                    return Outcome::said("no labels".to_string());
                }
                Ok(known) => *prompt = Some(Prompt::labels(&task.id, &task.labels, &known)),
                Err(error) => return Outcome::said(error),
            }
        }
        Ask::Move => {
            match screen
                .request(async |client: &Client| {
                    tokio::try_join!(client.projects(), client.sections())
                })
                .await
            {
                Ok((projects, sections)) => {
                    *prompt = Some(Prompt::move_to(&task.id, &projects, &sections));
                }
                Err(error) => return Outcome::said(error),
            }
        }
    }
    Outcome::quiet()
}

/// What an open prompt asks the API for once it is sent, read off the prompt before the request
/// so the prompt itself can be closed or kept as the answer decides.
enum Sending {
    View(usize),
    Delete(String),
    Due {
        id: String,
        due: String,
    },
    Add(String),
    Label {
        id: String,
        at: usize,
        name: String,
        was_on: bool,
        labels: Vec<String>,
    },
    Move {
        id: String,
        label: String,
        to: Destination,
    },
    Edit {
        id: String,
        content: String,
        description: String,
    },
    Nothing,
}

fn sending(prompt: &Prompt) -> Sending {
    match prompt {
        Prompt::Views(picker) => Sending::View(picker.at()),
        Prompt::Delete { id, .. } => Sending::Delete(id.clone()),
        Prompt::Due { id, input } if !input.is_blank() => Sending::Due {
            id: id.clone(),
            due: input.text().to_string(),
        },
        Prompt::Add { input } if !input.is_blank() => Sending::Add(input.text().to_string()),
        Prompt::Labels {
            id,
            choices,
            picker,
        } => match choices.get(picker.at()) {
            Some(choice) => Sending::Label {
                id: id.clone(),
                at: picker.at(),
                name: choice.name.clone(),
                was_on: choice.on,
                labels: prompt::toggled(choices, picker.at()),
            },
            None => Sending::Nothing,
        },
        Prompt::Move {
            id,
            choices,
            picker,
        } => match choices.get(picker.at()) {
            Some(choice) => Sending::Move {
                id: id.clone(),
                label: choice.label.trim().to_string(),
                to: choice.to.clone(),
            },
            None => Sending::Nothing,
        },
        Prompt::Edit { id, draft } if !draft.is_blank() => {
            let (content, description) = split(&draft.text());
            Sending::Edit {
                id: id.clone(),
                content,
                description,
            }
        }
        // A comment is typed on the detail screen, which posts it itself: the quick edits act on
        // the task under the list's cursor and the detail screen has no list.
        Prompt::Comment { .. }
        | Prompt::Note { .. }
        | Prompt::Due { .. }
        | Prompt::Add { .. }
        | Prompt::Edit { .. } => Sending::Nothing,
    }
}

/// The task's words as the box holds them: the first line is the content, everything under it is
/// the description, which is the shape the box was opened in.
fn split(text: &str) -> (String, String) {
    match text.split_once('\n') {
        Some((content, description)) => (content.trim().to_string(), description.to_string()),
        None => (text.trim().to_string(), String::new()),
    }
}

/// Send what the open prompt has been given.
pub async fn send(screen: &mut Screen<'_>, prompt: &mut Option<Prompt>) -> Outcome {
    let Some(open) = prompt.as_ref() else {
        return Outcome::quiet();
    };
    match sending(open) {
        // An input left empty asks for nothing, so it closes without a request.
        Sending::Nothing => {
            *prompt = None;
            Outcome::quiet()
        }
        Sending::View(at) => {
            *prompt = None;
            if screen.views.select(at) {
                return Outcome::said(screen.show().await);
            }
            Outcome::quiet()
        }
        Sending::Delete(id) => {
            *prompt = None;
            let sent = screen.write(Queued::Delete(id)).await;
            Outcome::said(reported(screen, "deleted".to_string(), sent).await)
        }
        Sending::Due { id, due } => {
            let sent = screen
                .write(Queued::Update {
                    id,
                    change: Change {
                        due_string: Some(due),
                        ..Default::default()
                    },
                })
                .await;
            // A refused write keeps the prompt open, so the line typed into it is not lost.
            if sent.is_ok() {
                *prompt = None;
            }
            Outcome::said(reported(screen, "due set".to_string(), sent).await)
        }
        Sending::Add(text) => match screen.add(&text).await {
            Ok(Some(task)) => {
                *prompt = None;
                Outcome::said(format!(
                    "added {}  {}",
                    task.content,
                    screen.refresh().await
                ))
            }
            Ok(None) => {
                *prompt = None;
                Outcome::said("queued: add".to_string())
            }
            Err(error) => Outcome::said(error),
        },
        Sending::Label {
            id,
            at,
            name,
            was_on,
            labels,
        } => {
            let sent = screen
                .write(Queued::Update {
                    id,
                    change: Change {
                        labels: Some(labels),
                        ..Default::default()
                    },
                })
                .await;
            if sent.is_err() {
                return Outcome::said(reported(screen, String::new(), sent).await);
            }
            // The picker stays open, marked the way the write left the task, so several labels
            // can be toggled without reopening it.
            if let Some(Prompt::Labels { choices, .. }) = prompt.as_mut()
                && let Some(choice) = choices.get_mut(at)
            {
                choice.on = !was_on;
            }
            let done = format!("@{name} {}", if was_on { "off" } else { "on" });
            Outcome::said(reported(screen, done, sent).await)
        }
        Sending::Edit {
            id,
            content,
            description,
        } => {
            let sent = screen
                .write(Queued::Update {
                    id,
                    change: Change {
                        content: Some(content),
                        description: Some(description),
                        ..Default::default()
                    },
                })
                .await;
            // A refused write keeps the box open, so the lines just typed into it are not lost.
            if sent.is_ok() {
                *prompt = None;
            }
            Outcome::said(reported(screen, "saved".to_string(), sent).await)
        }
        Sending::Move { id, label, to } => {
            *prompt = None;
            let sent = screen.write(Queued::Move { id, to }).await;
            Outcome::said(reported(screen, format!("moved to {label}"), sent).await)
        }
    }
}

/// What the status line says about a write: the API's own words when it refused, what is waiting
/// when the network is down, and otherwise what the write did followed by a fresh read of the
/// showing view.
async fn reported(screen: &mut Screen<'_>, done: String, sent: Result<Sent, String>) -> String {
    match sent {
        Err(error) => error,
        // Offline there is nothing to read back, so the status line names the write that is
        // waiting and its row keeps the waiting mark until the queue is sent.
        Ok(Sent::Queued) => match done.is_empty() {
            true => format!("queued{}", screen.queue.waiting_tail()),
            false => format!("queued: {done}{}", screen.queue.waiting_tail()),
        },
        Ok(Sent::Done) => {
            let read = screen.refresh().await;
            if done.is_empty() {
                read
            } else {
                format!("{done}  {read}")
            }
        }
    }
}

/// One step up in urgency, wrapping from the most urgent back to no priority at all: the API's
/// 1, 2, 3, 4 is the app's p4, p3, p2, p1, so a cycle reads as p4 to p1 and round again.
fn cycled(priority: u8) -> u8 {
    if priority >= HIGHEST_PRIORITY {
        LOWEST_PRIORITY
    } else {
        priority + 1
    }
}

#[cfg(test)]
mod tests;
