use crossterm::event::KeyCode;
use herdr_damnit_application::Request;

use super::*;
use crate::app::tests::{harness, harness_with_missing_dam};
use crate::screens::tests::LS;

const CLEAN: &str = r#"{"staged":[],"unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#;

const NOT_ON_PATH: &str = "dam is not on PATH; install it with cargo install damnit, then press R.";

fn unwrapped(drawn: &str) -> String {
    drawn.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn opening_asks_for_the_version_and_the_status_before_anything_else() {
    let mut harness = harness();
    start(&mut harness.app);
    harness.answer(0, 0, "dam 0.2.0\n", "");
    harness.answer(1, 0, CLEAN, "");

    let lines = harness.lines();
    assert_eq!(lines[0], "--version");
    assert_eq!(lines[1], "status --json");
    assert_eq!(lines[2], "ls !done --json");
}

#[test]
fn changing_screens_during_the_handshake_defers_their_reads_until_acceptance() {
    let mut harness = harness();
    start(&mut harness.app);
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Tab);

    assert_eq!(harness.lines(), vec!["--version".to_string()]);

    harness.answer(0, 0, "dam 0.2.0\n", "");
    harness.answer(1, 0, CLEAN, "");

    assert_eq!(
        harness.lines(),
        vec![
            "--version",
            "status --json",
            "ls !done --json",
            "ls done --json",
            "log --json",
        ]
    );
}

#[test]
fn a_dam_below_the_floor_draws_the_refusal_and_makes_no_further_call() {
    let mut harness = harness();
    start(&mut harness.app);
    harness.answer(0, 0, "dam 0.0.9\n", "");

    assert_eq!(harness.lines(), vec!["--version".to_string()]);
    assert!(harness.app.refusal.is_some());
    let drawn = crate::screens::render_to_text(&harness.app, 80, 6);
    assert!(
        drawn.contains("is older than the 0.2 this pane needs"),
        "{drawn}"
    );
}

#[test]
fn a_newer_dam_warns_once_rather_than_once_per_read() {
    let mut harness = harness();
    start(&mut harness.app);
    harness.answer(0, 0, "dam 0.4.0\n", "");
    harness.answer(1, 0, CLEAN, "");
    let warned = harness.app.message.clone();
    assert!(warned.contains("newer than this pane knows"), "{warned}");

    harness.app.message.clear();
    harness.app.submit(Request::Status);
    harness.answer(3, 0, CLEAN, "");

    assert_eq!(
        harness.app.message, "",
        "the handshake warning came back on a later read"
    );
}

#[test]
fn a_status_document_missing_a_key_refuses_to_draw() {
    let mut harness = harness();
    start(&mut harness.app);
    harness.answer(0, 0, "dam 0.2.0\n", "");
    harness.answer(
        1,
        0,
        r#"{"staged":[],"unstaged":[],"conflicts":[],"notices":[]}"#,
        "",
    );

    assert!(harness.app.refusal.is_some());
    assert_eq!(
        harness.lines().len(),
        2,
        "it read on past a failed handshake"
    );
}

#[test]
fn a_dam_that_is_not_there_draws_a_refusal_that_outlives_the_keys() {
    let mut harness = harness_with_missing_dam();
    start(&mut harness.app);
    harness.press(KeyCode::Char('j'));
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Char('x'));

    let drawn = crate::screens::render_to_text(&harness.app, 80, 6);
    assert!(unwrapped(&drawn).contains(NOT_ON_PATH), "{drawn}");
    assert!(!drawn.contains("0 open"), "{drawn}");
    assert!(!drawn.contains("nothing in this view"), "{drawn}");
}

#[test]
fn r_on_the_missing_dam_refusal_asks_for_the_version_again_and_a_dam_found_now_leads_to_the_list() {
    let mut harness = harness_with_missing_dam();
    start(&mut harness.app);
    harness.press(KeyCode::Char('R'));
    assert_eq!(harness.lines(), Vec::<String>::new());

    harness.install_dam();
    harness.press(KeyCode::Char('R'));
    assert_eq!(harness.lines(), vec!["--version".to_string()]);

    harness.answer(0, 0, "dam 0.2.0\n", "");
    harness.answer(1, 0, CLEAN, "");
    harness.answer(2, 0, LS, "");

    let drawn = crate::screens::render_to_text(&harness.app, 80, 6);
    assert!(drawn.contains("3 open"), "{drawn}");
    assert!(!unwrapped(&drawn).contains(NOT_ON_PATH), "{drawn}");
}
