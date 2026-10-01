use crossterm::event::KeyModifiers;

use super::*;

fn control(character: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL)
}

#[test]
fn p_pushes_and_l_pulls() {
    let mut harness = harness();
    harness.press(KeyCode::Char('P'));
    assert_eq!(harness.last(), "push --json");
    harness.answer(0, 0, r#"{"remotes":[]}"#, "");

    harness.press(KeyCode::Char('L'));
    assert_eq!(harness.last(), "pull --json");
}

#[test]
fn a_second_push_is_refused_with_a_sentence_and_never_spawned() {
    let mut harness = harness();
    harness.press(KeyCode::Char('P'));
    let before = harness.lines().len();
    harness.press(KeyCode::Char('L'));

    assert_eq!(
        harness.app.message,
        "a push is already running; <C-c> cancels it."
    );
    assert_eq!(harness.lines().len(), before);
}

#[test]
fn a_push_with_a_failed_mutation_is_still_a_success_and_names_the_count() {
    let mut harness = harness();
    harness.press(KeyCode::Char('P'));
    harness.answer(
        0,
        0,
        r#"{"remotes":[{"remote":"example","sent":3,"succeeded":2,"skipped":0,
          "failed":[{"oid":"9a0b1c2","why":"the service refused the write"}]}]}"#,
        "",
    );

    assert_eq!(
        harness.app.message,
        "example: 3 sent, 2 ok, 1 failed, 0 skipped"
    );
}

#[test]
fn control_c_cancels_the_job_the_header_names() {
    let mut harness = harness();
    harness.press(KeyCode::Char('P'));
    harness.app.key(control('c'));

    assert_eq!(harness.cancels(), 1);
    harness.answer(0, 3, "", "");
    assert_eq!(harness.app.message, "cancelled");
}

#[test]
fn control_c_with_nothing_in_flight_says_nothing_and_opens_no_box() {
    let mut harness = harness();
    harness.app.key(control('c'));

    assert!(harness.app.message.is_empty());
    assert!(harness.app.overlay.is_none());
}
