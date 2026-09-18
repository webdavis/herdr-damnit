//! The detail screen's cases, against a loopback double. No token and no live call.

use super::*;
use crossterm::event::KeyCode;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A loopback double: every GET is answered with the thread body that is current, and every
/// write with `write_status` and `write_body`. Posting swaps the thread for `after`, which is
/// how a refetch is proven to be what put the new comment on screen.
struct Double {
    base_url: String,
    requests: Arc<Mutex<Vec<String>>>,
}

impl Double {
    fn requests(&self) -> Vec<String> {
        self.requests.lock().expect("lock").clone()
    }
}

async fn serve(
    thread: &'static str,
    write_status: &'static str,
    write_body: &'static str,
    after: Option<&'static str>,
) -> Double {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let base_url = format!("http://{}", listener.local_addr().expect("addr"));
    let requests = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&requests);
    let showing = Arc::new(Mutex::new(thread));
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut buffer = [0u8; 4096];
            let read = socket.read(&mut buffer).await.unwrap_or(0);
            let request = String::from_utf8_lossy(&buffer[..read]).to_string();
            let mut words = request.split_whitespace();
            let line = format!(
                "{} {}",
                words.next().unwrap_or_default(),
                words.next().unwrap_or_default()
            );
            let body = request.split_once("\r\n\r\n").map(|(_, body)| body);
            seen.lock().expect("lock").push(
                format!("{line} {}", body.unwrap_or_default())
                    .trim_end()
                    .to_string(),
            );
            let answer = if line.starts_with("GET") {
                ("200 OK", *showing.lock().expect("lock"))
            } else {
                if write_status.starts_with("200")
                    && let Some(after) = after
                {
                    *showing.lock().expect("lock") = after;
                }
                (write_status, write_body)
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
    Double { base_url, requests }
}

fn config() -> Config {
    Config::parse(r#"token_command = ["sh", "-c", "printf test-token"]"#).expect("parses")
}

const EMPTY_THREAD: &str = r#"{"results":[],"next_cursor":null}"#;

/// Two comments, the newer one sent first and one carrying no timestamp at all, so the
/// ordering the screen draws cannot be the order the API answered in.
const THREAD: &str = r#"{"results":[
    {"id":"c2","content":"second","posted_at":"2026-09-17T08:00:00Z","posted_uid":"7",
     "file_attachment":null},
    {"id":"c3","content":"undated","posted_at":null,"posted_uid":null,
     "file_attachment":null},
    {"id":"c1","content":"first","posted_at":"2026-09-16T08:00:00Z","posted_uid":"7",
     "file_attachment":{"file_name":"note.txt"}}],"next_cursor":null}"#;

async fn open(double: &Double, description: &str) -> (Detail, Connection, Config) {
    let config = config();
    let mut connection = Connection::build(&config, &double.base_url).await;
    let detail = Detail::open(
        &mut connection,
        &config,
        &double.base_url,
        "6X",
        "water the plants",
        description,
    )
    .await;
    (detail, connection, config)
}

fn texts(detail: &Detail) -> Vec<String> {
    detail
        .lines()
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect()
        })
        .collect()
}

async fn press(
    detail: &mut Detail,
    connection: &mut Connection,
    config: &Config,
    base_url: &str,
    keys: &[KeyCode],
) -> After {
    let mut after = After::Stay;
    for key in keys {
        after = detail.key(*key, connection, config, base_url).await;
    }
    after
}

fn typed(line: &str) -> Vec<KeyCode> {
    line.chars().map(KeyCode::Char).collect()
}

#[tokio::test]
async fn the_detail_draws_the_description_as_markdown_and_the_thread_oldest_first() {
    let double = serve(THREAD, "200 OK", "null", None).await;
    let (detail, _, _) = open(&double, "## Steps\n- water **well**").await;

    assert_eq!(
        texts(&detail),
        vec![
            "water the plants",
            "",
            "Steps",
            "- water well",
            "",
            "comments (3)",
            "2026-09-16  #7",
            "  first",
            "  [attached] note.txt",
            "",
            "2026-09-17  #7",
            "  second",
            "",
            "undated",
            "  undated",
            "",
        ]
    );
    assert_eq!(detail.status(), "3 comments");
}

#[tokio::test]
async fn a_task_with_neither_a_description_nor_a_comment_says_so_rather_than_drawing_nothing() {
    let double = serve(EMPTY_THREAD, "200 OK", "null", None).await;
    let (detail, _, _) = open(&double, "   ").await;

    assert_eq!(
        texts(&detail),
        vec![
            "water the plants",
            "",
            "no description",
            "",
            "comments (0)",
            "no comments",
        ]
    );
    assert_eq!(detail.status(), "0 comments");
}

#[tokio::test]
async fn c_posts_the_lines_typed_and_the_comment_arrives_by_a_re_read() {
    let double = serve(
        EMPTY_THREAD,
        "200 OK",
        r#"{"id":"c9","content":"typed","posted_at":null,"posted_uid":null,
             "file_attachment":null}"#,
        Some(
            r#"{"results":[{"id":"c9","content":"first line\nsecond line",
                 "posted_at":"2026-09-18T08:00:00Z","posted_uid":"7",
                 "file_attachment":null}],"next_cursor":null}"#,
        ),
    )
    .await;
    let (mut detail, mut connection, config) = open(&double, "").await;

    let mut keys = vec![KeyCode::Char('c')];
    keys.extend(typed("first line"));
    keys.push(KeyCode::Enter);
    keys.extend(typed("second line"));
    keys.push(crate::prompt::SEND);
    let after = press(
        &mut detail,
        &mut connection,
        &config,
        &double.base_url,
        &keys,
    )
    .await;

    assert_eq!(after, After::Stay);
    assert!(
        detail.prompt().is_none(),
        "the box stayed open after a post"
    );
    let writes: Vec<String> = double
        .requests()
        .into_iter()
        .filter(|request| !request.starts_with("GET"))
        .collect();
    assert_eq!(
        writes,
        [r#"POST /comments {"task_id":"6X","content":"first line\nsecond line"}"#.to_string()]
    );
    assert_eq!(detail.status(), "posted  1 comment");
    assert!(
        texts(&detail).contains(&"  first line".to_string()),
        "{:?}",
        texts(&detail)
    );
}

#[tokio::test]
async fn a_refused_post_keeps_every_line_typed_and_reports_the_api_s_own_message() {
    let double = serve(
        EMPTY_THREAD,
        "400 Bad Request",
        r#"{"error":"Invalid argument value",
             "error_extra":{"explanation":"Content is too long"}}"#,
        None,
    )
    .await;
    let (mut detail, mut connection, config) = open(&double, "").await;

    let mut keys = vec![KeyCode::Char('c')];
    keys.extend(typed("a long answer"));
    keys.push(KeyCode::Enter);
    keys.extend(typed("over two lines"));
    keys.push(crate::prompt::SEND);
    press(
        &mut detail,
        &mut connection,
        &config,
        &double.base_url,
        &keys,
    )
    .await;

    assert_eq!(
        detail.status(),
        "Invalid argument value: Content is too long"
    );
    let prompt = detail.prompt().expect("the box is still open");
    let (lines, caret) = prompt
        .entries(&crate::views::Views::new(&[]))
        .expect("lines");
    assert_eq!(lines, [" a long answer", " over two lines_"]);
    assert_eq!(caret, 1, "the caret is still on the line it was typed on");
}

#[tokio::test]
async fn an_empty_box_posts_nothing_and_just_closes() {
    let double = serve(EMPTY_THREAD, "200 OK", "null", None).await;
    let (mut detail, mut connection, config) = open(&double, "").await;
    let read = double.requests().len();

    press(
        &mut detail,
        &mut connection,
        &config,
        &double.base_url,
        &[KeyCode::Char('c'), crate::prompt::SEND],
    )
    .await;

    assert!(detail.prompt().is_none());
    assert_eq!(double.requests().len(), read, "an empty box still sent");
}

#[tokio::test]
async fn esc_throws_the_draft_away_and_a_key_in_the_box_never_leaves_the_screen() {
    let double = serve(EMPTY_THREAD, "200 OK", "null", None).await;
    let (mut detail, mut connection, config) = open(&double, "").await;

    // `q` and `j` are the screen's own keys, so a box that did not take every key would quit
    // or scroll while a comment was half typed.
    let after = press(
        &mut detail,
        &mut connection,
        &config,
        &double.base_url,
        &[KeyCode::Char('c'), KeyCode::Char('q'), KeyCode::Char('j')],
    )
    .await;
    assert_eq!(after, After::Stay);
    assert_eq!(detail.scroll(), 0);

    press(
        &mut detail,
        &mut connection,
        &config,
        &double.base_url,
        &[KeyCode::Esc],
    )
    .await;
    assert!(detail.prompt().is_none(), "Esc left the box open");
}

#[tokio::test]
async fn the_screen_scrolls_and_esc_goes_back_to_the_list() {
    let double = serve(EMPTY_THREAD, "200 OK", "null", None).await;
    let (mut detail, mut connection, config) = open(&double, "").await;

    press(
        &mut detail,
        &mut connection,
        &config,
        &double.base_url,
        &[KeyCode::Char('j'), KeyCode::Char('j'), KeyCode::Char('k')],
    )
    .await;
    assert_eq!(detail.scroll(), 1);

    press(
        &mut detail,
        &mut connection,
        &config,
        &double.base_url,
        &[KeyCode::Char('j'); 20],
    )
    .await;
    assert_eq!(
        detail.scroll(),
        detail.lines().len() as u16 - 1,
        "j past the last line ran into blank space"
    );

    press(
        &mut detail,
        &mut connection,
        &config,
        &double.base_url,
        &[KeyCode::Char('k'); 20],
    )
    .await;
    assert_eq!(detail.scroll(), 0, "the scroll went above the first line");

    assert_eq!(
        press(
            &mut detail,
            &mut connection,
            &config,
            &double.base_url,
            &[KeyCode::Esc]
        )
        .await,
        After::Back
    );
}

#[tokio::test]
async fn a_failed_read_leaves_the_thread_on_screen_and_reports_the_failure() {
    let double = serve(THREAD, "200 OK", "null", None).await;
    let (mut detail, _, config) = open(&double, "").await;
    let dead = {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("addr");
        format!("http://{address}")
    };
    let mut offline = Connection::build(&config, &dead).await;

    press(
        &mut detail,
        &mut offline,
        &config,
        &dead,
        &[KeyCode::Char('r')],
    )
    .await;

    assert!(
        detail.status().starts_with("network error:"),
        "{}",
        detail.status()
    );
    assert!(
        texts(&detail).contains(&"  first".to_string()),
        "the thread left the screen on a failed read"
    );
}

/// 32 columns: roughly a third of a normal terminal, the width a side pane opens at, so anything
/// too wide for it shows up truncated here.
const NARROW: u16 = 32;

/// Draw the detail into an off-screen terminal `NARROW` columns wide and report its rows.
fn narrow_frame(detail: &Detail, rows: u16) -> Vec<String> {
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(NARROW, rows)).expect("terminal");
    terminal
        .draw(|frame| crate::render::draw_detail(frame, &crate::views::Views::new(&[]), detail))
        .expect("draw");
    let buffer = terminal.backend().buffer().clone();
    (0..buffer.area.height)
        .map(|row| {
            (0..buffer.area.width)
                .map(|column| buffer[(column, row)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

#[tokio::test]
async fn every_wide_thing_a_description_can_hold_wraps_inside_a_narrow_pane() {
    let double = serve(EMPTY_THREAD, "200 OK", "null", None).await;
    let description = concat!(
        "A description long enough that it has to be broken across several lines to fit.\n",
        "\n",
        "See [the docs](https://example.invalid/a/quite/long/path/to/somewhere) for more.\n",
        "\n",
        "| column one | column two | column three |\n",
        "| --- | --- | --- |\n",
        "\n",
        "```sh\n",
        "printf '%s\\n' averyveryverylongunbreakableidentifierwithnospacesatallinit\n",
        "```\n",
    );
    let (detail, _, _) = open(&double, description).await;

    let frame = narrow_frame(&detail, 30);

    for row in &frame {
        assert!(
            row.chars().count() <= NARROW as usize,
            "a row ran past {NARROW} columns: {row:?}"
        );
    }
    let drawn = frame.join("\n");
    // Each wide thing is present in full, in pieces across rows, rather than cut off at the edge.
    for tail in [
        "somewhere>",
        "column three |",
        "identifierwithnospacesatallinit",
    ] {
        assert!(
            drawn.replace('\n', "").contains(tail),
            "{tail:?} was cut off rather than wrapped: {frame:?}"
        );
    }
}

#[tokio::test]
async fn the_comment_box_is_a_multi_line_box_drawn_over_the_thread() {
    let double = serve(EMPTY_THREAD, "200 OK", "null", None).await;
    let (mut detail, mut connection, config) = open(&double, "").await;

    let mut keys = vec![KeyCode::Char('c')];
    keys.extend(typed("first"));
    keys.push(KeyCode::Enter);
    keys.extend(typed("second"));
    press(
        &mut detail,
        &mut connection,
        &config,
        &double.base_url,
        &keys,
    )
    .await;
    let frame = narrow_frame(&detail, 10);

    assert!(frame[0].contains("todoist  task"), "{frame:?}");
    assert!(frame[1].contains("comment"), "the box is titled: {frame:?}");
    assert!(frame[2].contains("first"), "{frame:?}");
    assert!(
        frame[3].contains("second_"),
        "the caret is on the line being typed: {frame:?}"
    );
    assert_eq!(
        frame.last().expect("a hint line").trim(),
        "<C-d> post  <Esc> cancel"
    );
    for row in &frame {
        assert!(row.chars().count() <= NARROW as usize, "{row:?}");
    }
}
