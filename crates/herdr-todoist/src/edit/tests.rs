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
    pressed(
        &crate::reload::tests::config_with_token_command("printf test-token"),
        double,
        list,
        keys,
    )
    .await
    .0
}

/// The same, under a config of the caller's own, reporting what the last press left behind:
/// the status line, what the pane does next, and the prompt left open.
pub(crate) async fn pressed(
    config: &crate::config::Config,
    double: &Double,
    list: &mut List,
    keys: &[KeyCode],
) -> (String, After, Option<Prompt>) {
    let mut connection = Connection::build(config, &double.base_url).await;
    let mut views = Views::new(&[]);
    let mut prompt: Option<Prompt> = None;
    let mut status = String::new();
    let mut after = After::Stay;
    for key_press in keys {
        let mut screen = Screen {
            connection: &mut connection,
            config,
            base_url: &double.base_url,
            list,
            views: &mut views,
        };
        let outcome = if prompt.is_some() {
            prompt_key(*key_press, &mut screen, &mut prompt).await
        } else {
            key(*key_press, &mut screen, &mut prompt).await
        };
        if let Some(said) = outcome.status {
            status = said;
        }
        after = outcome.after;
    }
    (status, after, prompt)
}

/// The pane's config with a token source and an `editor` key of the caller's own.
fn config_with_editor(editor: &str) -> crate::config::Config {
    crate::config::Config::parse(&format!(
        "token_command = [\"sh\", \"-c\", \"printf test-token\"]\neditor = {editor}"
    ))
    .expect("parses")
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
async fn e_hands_the_pane_to_the_editor_rather_than_opening_a_box() {
    let double = serve("200 OK", "null").await;
    let mut list = list_of_one(1, &[]);

    let (_, after, prompt) = pressed(
        &crate::reload::tests::config_with_token_command("printf test-token"),
        &double,
        &mut list,
        &[character('e')],
    )
    .await;

    assert_eq!(after, After::Editor);
    assert!(prompt.is_none(), "a box opened with an editor configured");
}

#[tokio::test]
async fn e_with_no_editor_configured_opens_the_box_on_the_task_s_own_words() {
    let double = serve("200 OK", "null").await;
    let mut list = list_of_one(1, &[]);

    let (_, after, prompt) = pressed(
        &config_with_editor("[]"),
        &double,
        &mut list,
        &[character('e')],
    )
    .await;

    assert_eq!(after, After::Stay);
    let box_open = prompt.expect("the pane's own box");
    assert_eq!(box_open.title(), "edit");
    let (lines, _) = box_open.entries(&Views::new(&[])).expect("the lines typed");
    assert_eq!(lines, vec![" water the plants_"]);
}

#[tokio::test]
async fn e_on_a_row_with_no_task_under_the_cursor_does_nothing_at_all() {
    let double = serve("200 OK", "null").await;
    let mut list = List::new(vec![crate::list::Row::Header("First".to_string())]);

    let (status, after, prompt) = pressed(
        &config_with_editor("[]"),
        &double,
        &mut list,
        &[character('e')],
    )
    .await;

    assert_eq!(after, After::Stay);
    assert!(prompt.is_none(), "a box opened over no task");
    assert_eq!(status, "");
    assert!(
        double.requests().is_empty(),
        "a key on a heading sent: {:?}",
        double.requests()
    );
}

#[tokio::test]
async fn the_box_saves_the_first_line_as_the_content_and_the_rest_as_the_description() {
    let double = serve("200 OK", "null").await;
    let mut list = list_of_one(1, &[]);
    let mut keys = vec![character('e')];
    keys.extend(typed("!"));
    keys.push(KeyCode::Enter);
    keys.extend(typed("the ones on the sill"));
    keys.push(crate::prompt::SEND);

    let (status, _, prompt) = pressed(&config_with_editor("[]"), &double, &mut list, &keys).await;

    assert_eq!(
        double.writes(),
        [concat!(
            r#"POST /tasks/6X {"content":"water the plants!","#,
            r#""description":"the ones on the sill"}"#
        )
        .to_string()]
    );
    assert!(status.starts_with("saved"), "{status}");
    assert!(prompt.is_none(), "the box stayed open after it saved");
}

#[tokio::test]
async fn a_refused_save_keeps_the_box_open_with_the_lines_still_in_it() {
    let double = refusing().await;
    let mut list = list_of_one(1, &[]);
    let mut keys = vec![character('e')];
    keys.extend(typed("!"));
    keys.push(KeyCode::Enter);
    keys.extend(typed("the ones on the sill"));
    keys.push(crate::prompt::SEND);

    let (status, _, prompt) = pressed(&config_with_editor("[]"), &double, &mut list, &keys).await;

    assert_eq!(
        status,
        "Invalid argument value: Unable to parse the due date"
    );
    let box_open = prompt.expect("the box closed on a refused save");
    let (lines, _) = box_open.entries(&Views::new(&[])).expect("the lines typed");
    assert_eq!(lines, vec![" water the plants!", " the ones on the sill_"]);
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
