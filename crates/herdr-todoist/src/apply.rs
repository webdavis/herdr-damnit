//! What each quick edit sends, and what the pane says once the API has answered.
//!
//! Every write is followed by a read of the showing view, so the rows on screen come from the
//! server rather than from a guess at what the write did, and a refused write leaves the rows
//! alone with the API's own message in the status line.

use todoist::{Change, Client, Destination};

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
        content: task.text.clone(),
        priority: task.priority,
        labels: task.labels.clone(),
    })
}

pub async fn write(screen: &mut Screen<'_>, edit: Edit) -> Outcome {
    let Some(task) = under_cursor(screen) else {
        return Outcome::quiet();
    };
    let id = task.id;
    let (done, sent) = match edit {
        Edit::Complete => (
            "completed".to_string(),
            screen
                .request(async |client: &Client| client.close(&id).await)
                .await,
        ),
        Edit::Reopen => (
            "reopened".to_string(),
            screen
                .request(async |client: &Client| client.reopen(&id).await)
                .await,
        ),
        Edit::Priority => {
            let next = cycled(task.priority);
            (
                format!("p{}", HIGHEST_PRIORITY + LOWEST_PRIORITY - next),
                screen
                    .request(async |client: &Client| client.update(&id, &priority(next)).await)
                    .await,
            )
        }
    };
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
        Prompt::Due { .. } | Prompt::Add { .. } => Sending::Nothing,
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
            let sent = screen
                .request(async |client: &Client| client.delete(&id).await)
                .await;
            Outcome::said(reported(screen, "deleted".to_string(), sent).await)
        }
        Sending::Due { id, due } => {
            let sent = screen
                .request(async |client: &Client| {
                    client
                        .update(
                            &id,
                            &Change {
                                due_string: Some(due.clone()),
                                ..Default::default()
                            },
                        )
                        .await
                })
                .await;
            // A refused write keeps the prompt open, so the line typed into it is not lost.
            if sent.is_ok() {
                *prompt = None;
            }
            Outcome::said(reported(screen, "due set".to_string(), sent).await)
        }
        Sending::Add(text) => {
            let added = screen
                .request(async |client: &Client| client.quick_add(&text).await)
                .await;
            match added {
                Ok(task) => {
                    *prompt = None;
                    Outcome::said(format!(
                        "added {}  {}",
                        task.content,
                        screen.refresh().await
                    ))
                }
                Err(error) => Outcome::said(error),
            }
        }
        Sending::Label {
            id,
            at,
            name,
            was_on,
            labels,
        } => {
            let sent = screen
                .request(async |client: &Client| {
                    client
                        .update(
                            &id,
                            &Change {
                                labels: Some(labels.clone()),
                                ..Default::default()
                            },
                        )
                        .await
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
        Sending::Move { id, label, to } => {
            *prompt = None;
            let sent = screen
                .request(async |client: &Client| client.move_task(&id, &to).await)
                .await;
            Outcome::said(reported(screen, format!("moved to {label}"), sent).await)
        }
    }
}

/// What the status line says about a write: the API's own words when it refused, and otherwise
/// what the write did followed by a fresh read of the showing view.
async fn reported(screen: &mut Screen<'_>, done: String, sent: Result<(), String>) -> String {
    match sent {
        Err(error) => error,
        Ok(()) => {
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
mod tests {
    use crossterm::event::KeyCode;

    use crate::edit::tests::{
        character, list_of_one, press, refusing, serve, serve_reading, typed,
    };

    #[tokio::test]
    async fn a_refused_write_reports_the_api_s_own_message_and_keeps_the_rows_on_screen() {
        let double = refusing().await;
        let mut list = list_of_one(1, &[]);

        let mut keys = vec![character('s')];
        keys.extend(typed("wenesday"));
        keys.push(KeyCode::Enter);
        let status = press(&double, &mut list, &keys).await;

        assert_eq!(
            status,
            "Invalid argument value: Unable to parse the due date"
        );
        assert_eq!(list.task_count(), 1);
        assert_eq!(list.selected_id(), Some("6X"));
    }

    #[tokio::test]
    async fn a_takes_a_whole_quick_add_line_and_reports_the_task_todoist_made_of_it() {
        let double = serve("200 OK", r#"{"id":"7","content":"Pay rent","priority":4}"#).await;
        let mut list = list_of_one(1, &[]);

        let mut keys = vec![character('a')];
        keys.extend(typed("Pay rent tomorrow 9am p1 #Finances @home"));
        keys.push(KeyCode::Enter);
        let status = press(&double, &mut list, &keys).await;

        assert_eq!(
            double.writes(),
            [
                r#"POST /tasks/quick {"text":"Pay rent tomorrow 9am p1 #Finances @home"}"#
                    .to_string()
            ]
        );
        assert_eq!(status, "added Pay rent  0 open tasks");
    }

    #[tokio::test]
    async fn l_toggles_a_label_on_and_off_from_the_picker() {
        let double = serve_reading(
            "200 OK",
            "null",
            &[(
                "/labels",
                r#"{"results":[{"name":"home"}],"next_cursor":null}"#,
            )],
        )
        .await;
        let mut list = list_of_one(1, &[]);

        // The label picker reads the account's labels, then `<CR>` writes the task's whole label
        // set with the one under the cursor added, and a second `<CR>` writes it back off.
        let status = press(
            &double,
            &mut list,
            &[character('l'), KeyCode::Enter, KeyCode::Enter],
        )
        .await;

        assert_eq!(
            double.writes(),
            [
                r#"POST /tasks/6X {"labels":["home"]}"#.to_string(),
                r#"POST /tasks/6X {"labels":[]}"#.to_string(),
            ]
        );
        assert_eq!(status, "@home off  0 open tasks");
    }

    #[tokio::test]
    async fn l_with_no_labels_anywhere_says_so_instead_of_opening_an_empty_picker() {
        let double = serve_reading(
            "200 OK",
            "null",
            &[("/labels", r#"{"results":[],"next_cursor":null}"#)],
        )
        .await;
        let mut list = list_of_one(1, &[]);

        let status = press(&double, &mut list, &[character('l')]).await;

        assert_eq!(status, "no labels");
        assert_eq!(double.writes(), Vec::<String>::new());
    }

    #[tokio::test]
    async fn m_moves_the_task_to_the_destination_picked() {
        let double = serve_reading(
            "200 OK",
            "null",
            &[(
                "/projects",
                r#"{"results":[{"id":"p1","name":"First"}],"next_cursor":null}"#,
            )],
        )
        .await;
        let mut list = list_of_one(1, &[]);

        let status = press(&double, &mut list, &[character('m'), KeyCode::Enter]).await;

        assert_eq!(
            double.writes(),
            [r#"POST /tasks/6X/move {"project_id":"p1"}"#.to_string()],
            "{:?}",
            double.requests()
        );
        assert_eq!(status, "moved to First  0 open tasks");
    }

    #[tokio::test]
    async fn p_cycles_one_step_up_in_urgency_and_wraps_at_the_urgent_end() {
        for (from, sent, said) in [(1u8, 2u8, "p3"), (3, 4, "p1"), (4, 1, "p4")] {
            let double = serve("200 OK", "null").await;
            let mut list = list_of_one(from, &[]);

            let status = press(&double, &mut list, &[character('p')]).await;

            assert_eq!(
                double.writes(),
                [format!("POST /tasks/6X {{\"priority\":{sent}}}")],
                "from {from}"
            );
            assert_eq!(status, format!("{said}  0 open tasks"));
        }
    }

    #[tokio::test]
    async fn s_sends_the_line_typed_as_the_natural_language_due_string() {
        let double = serve("200 OK", "null").await;
        let mut list = list_of_one(1, &[]);

        let mut keys = vec![character('s')];
        keys.extend(typed("every 2 weeks"));
        keys.push(KeyCode::Enter);
        let status = press(&double, &mut list, &keys).await;

        assert_eq!(
            double.writes(),
            [r#"POST /tasks/6X {"due_string":"every 2 weeks"}"#.to_string()]
        );
        assert_eq!(status, "due set  0 open tasks");
    }

    #[tokio::test]
    async fn shift_x_reopens_the_task_under_the_cursor() {
        let double = serve("200 OK", "null").await;
        let mut list = list_of_one(1, &[]);

        let status = press(&double, &mut list, &[character('X')]).await;

        assert_eq!(double.writes(), ["POST /tasks/6X/reopen".to_string()]);
        assert_eq!(status, "reopened  0 open tasks");
    }

    #[tokio::test]
    async fn x_closes_the_task_under_the_cursor() {
        let double = serve("200 OK", "null").await;
        let mut list = list_of_one(1, &[]);

        let status = press(&double, &mut list, &[character('x')]).await;

        assert_eq!(double.writes(), ["POST /tasks/6X/close".to_string()]);
        assert_eq!(status, "completed  0 open tasks");
    }
}
