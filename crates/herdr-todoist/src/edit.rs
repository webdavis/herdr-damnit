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
    if let Prompt::Note { .. } = open {
        // The note box is the comment box's widget, so it takes the same keys: `<CR>` opens a
        // line and SEND hands the brief over.
        if key == crate::prompt::SEND {
            return send::send(screen, prompt, &send::Host::from_env(&send::cli)).await;
        }
        let Some(draft) = open.draft_mut() else {
            return Outcome::quiet();
        };
        match key {
            KeyCode::Esc => *prompt = None,
            KeyCode::Enter => draft.newline(),
            KeyCode::Backspace => draft.backspace(),
            KeyCode::Char(character) => draft.push(character),
            _ => {}
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

    fn after(after: After) -> Self {
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
pub(crate) mod tests {
    use super::*;
    use crate::connection::Connection;
    use crate::cursor::List;
    use crate::list;
    use crate::list::tests::task;
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// Every request a double was sent, as its request line plus body.
    pub(crate) type Seen = Arc<Mutex<Vec<String>>>;

    pub(crate) struct Double {
        pub(crate) base_url: String,
        seen: Seen,
    }

    impl Double {
        pub(crate) fn requests(&self) -> Vec<String> {
            self.seen.lock().expect("lock").clone()
        }

        /// The requests that are not the list reads a refresh makes.
        pub(crate) fn writes(&self) -> Vec<String> {
            self.requests()
                .into_iter()
                .filter(|request| !request.starts_with("GET"))
                .collect()
        }
    }

    /// A loopback double answering every write with `status` and `body`, every read named in
    /// `reads` with the body it is paired with, and every other read with an empty page.
    pub(crate) async fn serve(status: &'static str, body: &'static str) -> Double {
        serve_reading(status, body, &[]).await
    }

    pub(crate) async fn serve_reading(
        status: &'static str,
        body: &'static str,
        reads: &'static [(&'static str, &'static str)],
    ) -> Double {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let base_url = format!("http://{}", listener.local_addr().expect("addr"));
        let seen: Seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut buffer = [0u8; 2048];
                let read = socket.read(&mut buffer).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                // The method and the target, without the HTTP version, so a case reads as the
                // request it is about.
                let mut words = request.split_whitespace();
                let line = format!(
                    "{} {}",
                    words.next().unwrap_or_default(),
                    words.next().unwrap_or_default()
                );
                let sent_body = request.split_once("\r\n\r\n").map(|(_, body)| body);
                recorded.lock().expect("lock").push(
                    format!("{line} {}", sent_body.unwrap_or_default())
                        .trim_end()
                        .to_string(),
                );
                // A list read is answered with an empty page whatever the write case is, so a
                // refresh after a write never turns a write's case into a parse failure.
                let answer = if line.starts_with("GET") {
                    let read = reads
                        .iter()
                        .find(|(needle, _)| line.contains(needle))
                        .map(|(_, body)| *body);
                    (
                        "200 OK",
                        read.unwrap_or(r#"{"results":[],"next_cursor":null}"#),
                    )
                } else {
                    (status, body)
                };
                let response = format!(
                    "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    answer.0,
                    answer.1.len(),
                    answer.1
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });
        Double { base_url, seen }
    }

    /// A double that refuses every write the way the API refuses one, with its own message.
    pub(crate) async fn refusing() -> Double {
        serve(
            "400 Bad Request",
            r#"{"error":"Invalid argument value",
                 "error_extra":{"explanation":"Unable to parse the due date"}}"#,
        )
        .await
    }

    /// One open task, carrying the priority and labels a quick edit reads off its row.
    pub(crate) fn list_of_one(priority: u8, labels: &[&str]) -> List {
        let project = serde_json::from_str(r#"{"id":"p1","name":"First"}"#).expect("project");
        let labels = labels
            .iter()
            .map(|label| format!("\"{label}\""))
            .collect::<Vec<_>>()
            .join(",");
        List::new(list::build(
            &[task(&format!(
                r#"{{"id":"6X","content":"water the plants","project_id":"p1",
                     "priority":{priority},"labels":[{labels}]}}"#
            ))],
            &[project],
            &[],
        ))
    }

    /// Press `keys` on the open list of one task, against `double`, and report the status line
    /// the last press left behind.
    pub(crate) async fn press(double: &Double, list: &mut List, keys: &[KeyCode]) -> String {
        let config = crate::reload::tests::config_with_token_command("printf test-token");
        let mut connection = Connection::build(&config, &double.base_url).await;
        let mut views = Views::new(&[]);
        let mut prompt: Option<Prompt> = None;
        let mut status = String::new();
        for pressed in keys {
            let mut screen = Screen {
                connection: &mut connection,
                config: &config,
                base_url: &double.base_url,
                list,
                views: &mut views,
            };
            let outcome = if prompt.is_some() {
                prompt_key(*pressed, &mut screen, &mut prompt).await
            } else {
                key(*pressed, &mut screen, &mut prompt).await
            };
            if let Some(said) = outcome.status {
                status = said;
            }
        }
        status
    }

    pub(crate) fn character(key: char) -> KeyCode {
        KeyCode::Char(key)
    }

    pub(crate) fn typed(line: &str) -> Vec<KeyCode> {
        line.chars().map(KeyCode::Char).collect()
    }

    #[tokio::test]
    async fn a_dismissed_confirm_sends_nothing_at_all() {
        let double = serve("200 OK", "null").await;
        let mut list = list_of_one(1, &[]);

        let status = press(&double, &mut list, &[character('d'), KeyCode::Esc]).await;

        assert!(
            double.requests().is_empty(),
            "a dismissed confirm still sent: {:?}",
            double.requests()
        );
        assert_eq!(status, "");
        assert_eq!(list.task_count(), 1, "the row left on a dismissal");
    }

    #[tokio::test]
    async fn an_input_left_empty_sends_nothing() {
        let double = serve("200 OK", "null").await;
        let mut list = list_of_one(1, &[]);

        press(&double, &mut list, &[character('a'), KeyCode::Enter]).await;

        assert!(
            double.requests().is_empty(),
            "an empty line still sent: {:?}",
            double.requests()
        );
    }

    #[tokio::test]
    async fn a_key_with_no_task_under_the_cursor_sends_nothing() {
        let double = serve("200 OK", "null").await;
        let mut list = List::new(Vec::new());

        press(
            &double,
            &mut list,
            &[character('x'), character('p'), character('d')],
        )
        .await;

        assert!(
            double.requests().is_empty(),
            "a key on an empty list sent: {:?}",
            double.requests()
        );
    }
    #[tokio::test]
    async fn dd_deletes_the_task_after_the_second_d() {
        let double = serve("200 OK", "null").await;
        let mut list = list_of_one(1, &[]);

        let status = press(&double, &mut list, &[character('d')]).await;
        assert!(
            double.requests().is_empty(),
            "the first d sent something: {:?}",
            double.requests()
        );

        assert_eq!(status, "", "the first d said something");

        let double = serve("200 OK", "null").await;
        let status = press(&double, &mut list, &[character('d'), character('d')]).await;

        assert_eq!(double.writes(), ["DELETE /tasks/6X".to_string()]);
        assert_eq!(status, "deleted  0 open tasks");
    }
}
