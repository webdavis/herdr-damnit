use std::time::{Duration, Instant};

use crate::screens::render_to_text;
use crate::screens::tests::loaded;

use super::*;

const SHOWN: &str =
    r#"{"oid":"5d6e7f8","kind":"task","subject":"x","task":{"done":false,"priority":4}}"#;

const ONE_STAGED: &str = r#"{"staged":[{"oid":"5d6e7f8","op":"update","subject":"x","fields":[]}],
  "unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#;

fn top_row(harness: &Harness) -> String {
    render_to_text(&harness.app, 50, 8)
        .lines()
        .next()
        .unwrap_or_default()
        .to_string()
}

#[test]
fn the_status_line_over_the_detail_names_it_and_counts_nothing() {
    let mut harness = loaded();
    harness.press(KeyCode::Enter);
    harness.answer(&["show"], 0, SHOWN, "");

    assert_eq!(top_row(&harness), "dam  detail");
}

#[test]
fn the_interval_is_held_back_while_the_detail_is_open() {
    let mut harness = loaded();
    let now = Instant::now();
    harness.app.tick(now);
    harness.press(KeyCode::Enter);
    harness.answer(&["show"], 0, SHOWN, "");
    let before = harness.lines().len();

    harness.app.tick(now + Duration::from_secs(900));

    assert_eq!(harness.lines().len(), before, "it re-read under the detail");
}

#[test]
fn escape_from_a_detail_opened_on_the_status_screen_returns_to_its_row() {
    let mut harness = loaded();
    harness.app.submit(Request::Status);
    harness.answer(&["status"], 0, ONE_STAGED, "");
    harness.press(KeyCode::Tab);
    harness.select("5d6e7f8");
    harness.press(KeyCode::Enter);
    harness.answer(&["show"], 0, SHOWN, "");
    assert_eq!(top_row(&harness), "dam  detail");

    harness.press(KeyCode::Esc);

    assert_eq!(harness.app.showing().screen(), Screen::Status);
    assert_eq!(
        harness
            .app
            .status()
            .cursor()
            .selected_oid()
            .map(|oid| oid.as_str()),
        Some("5d6e7f8")
    );
    assert!(
        top_row(&harness).starts_with("dam  status"),
        "{}",
        top_row(&harness)
    );
}

#[test]
fn a_status_request_while_the_detail_is_open_shows_the_status_screen() {
    let mut harness = loaded();
    harness.press(KeyCode::Enter);
    harness.answer(&["show"], 0, SHOWN, "");
    let path = std::env::temp_dir().join(format!("herdr-damnit-detail-{}", std::process::id()));
    std::fs::write(&path, crate::pane::STATUS_REQUEST).expect("the request is written");
    harness.app.watch_view_requests(path);

    harness.app.tick(Instant::now());

    assert_eq!(harness.app.showing().screen(), Screen::Status);
    assert!(
        top_row(&harness).starts_with("dam  status"),
        "{}",
        top_row(&harness)
    );
    assert_eq!(harness.press(KeyCode::Esc), After::Quit);
}
