use crate::screens::tests::loaded;

use super::*;

const TWO_CHANGES: &str = r#"{"staged":[
  {"oid":"1a2b3c4","op":"update","subject":"ship the pin bump","fields":["due"]},
  {"oid":"5d6e7f8","op":"update","subject":"refresh the roster row","fields":["due"]}],
  "unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#;

#[test]
fn j_and_k_move_the_cursor_and_stop_at_either_end() {
    let mut harness = loaded();
    let first = harness.app.list.selected_oid().cloned();

    harness.press(KeyCode::Char('j'));
    assert_ne!(harness.app.list.selected_oid().cloned(), first);
    harness.press(KeyCode::Up);
    assert_eq!(harness.app.list.selected_oid().cloned(), first);
    harness.press(KeyCode::Char('k'));
    assert_eq!(harness.app.list.selected_oid().cloned(), first);
    harness.press(KeyCode::Down);
    harness.press(KeyCode::Char('j'));
    harness.press(KeyCode::Char('j'));
    assert_eq!(
        harness.app.list.selected_oid().map(|oid| oid.as_str()),
        Some("9a0b1c2")
    );
}

#[test]
fn the_status_screen_has_its_own_cursor_and_enter_reads_the_row_under_it() {
    let mut harness = loaded();
    harness.app.submit(Request::Status);
    harness.answer(3, 0, TWO_CHANGES, "");
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Char('j'));
    harness.press(KeyCode::Enter);

    assert_eq!(harness.last(), "show 5d6e7f8 --json");
}

#[test]
fn r_re_reads_both_the_list_and_the_status() {
    let mut harness = loaded();
    let before = harness.lines().len();
    harness.press(KeyCode::Char('R'));

    let asked = &harness.lines()[before..];
    assert!(asked.contains(&"ls !done --json".to_string()), "{asked:?}");
    assert!(asked.contains(&"status --json".to_string()), "{asked:?}");
}

#[test]
fn r_before_dam_ever_answered_starts_the_handshake_again() {
    let mut harness = harness();
    harness.press(KeyCode::Char('r'));

    assert_eq!(harness.lines(), vec!["--version".to_string()]);
}

#[test]
fn a_second_r_while_the_version_check_is_in_flight_spawns_nothing() {
    let mut harness = harness();
    crate::open::start(&mut harness.app);
    harness.answer(0, 0, "dam 0.0.9\n", "");
    harness.press(KeyCode::Char('R'));
    harness.press(KeyCode::Char('R'));

    assert_eq!(harness.lines(), vec!["--version".to_string(); 2]);

    harness.answer(1, 0, "dam 0.2.0\n", "");
    harness.press(KeyCode::Char('r'));

    assert_eq!(
        harness.lines(),
        vec!["--version", "--version", "status --json"]
    );
}

#[test]
fn r_on_the_too_old_refusal_asks_for_the_version_again_and_keeps_the_refusal_up_meanwhile() {
    let mut harness = harness();
    crate::open::start(&mut harness.app);
    harness.answer(0, 0, "dam 0.0.9\n", "");
    harness.press(KeyCode::Char('2'));
    harness.press(KeyCode::Char('x'));
    assert_eq!(harness.lines(), vec!["--version".to_string()]);

    harness.press(KeyCode::Char('R'));

    assert_eq!(harness.lines(), vec!["--version".to_string(); 2]);
    let drawn = crate::screens::render_to_text(&harness.app, 80, 4);
    assert!(
        drawn.contains("is older than the 0.2 this pane needs"),
        "{drawn}"
    );
}

#[test]
fn opening_a_second_task_drops_the_first_tasks_later_answer_and_shows_the_second() {
    let mut harness = loaded();
    harness.press(KeyCode::Enter);
    harness.press(KeyCode::Char('j'));
    harness.press(KeyCode::Enter);
    harness.answer(
        4,
        0,
        r#"{"oid":"5d6e7f8","kind":"task","subject":"refresh the roster row","task":{"done":false,"priority":4}}"#,
        "",
    );
    harness.answer(
        3,
        0,
        r#"{"oid":"1a2b3c4","kind":"task","subject":"ship the pin bump","task":{"done":false,"priority":1}}"#,
        "",
    );

    assert_eq!(harness.cancels(), 1);
    assert_eq!(
        harness
            .app
            .showing()
            .detail()
            .map(|shown| shown.subject.as_str()),
        Some("refresh the roster row")
    );
}
