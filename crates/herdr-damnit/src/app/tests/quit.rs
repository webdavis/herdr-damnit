use crate::app::Overlay;
use crate::screens::tests::loaded;

use super::*;

const CONFLICTED: &str = r#"{"staged":[],"unstaged":[],"unpushed":[],
  "conflicts":[{"oid":"3d4e5f6","remote":"example",
    "ours":{"oid":"3d4e5f6","kind":"task","subject":"mine"},
    "theirs":{"oid":"3d4e5f6","kind":"task","subject":"theirs"}}],
  "notices":[]}"#;

fn conflicted() -> Harness {
    let mut harness = loaded();
    harness.app.submit(
        JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness.answer(3, 0, CONFLICTED, "");
    harness.press(KeyCode::Tab);
    harness.select("3d4e5f6");
    harness
}

#[test]
fn o_and_t_resolve_the_conflict_under_the_cursor_toward_one_side() {
    let mut harness = conflicted();
    harness.press(KeyCode::Char('o'));
    assert_eq!(harness.last(), "resolve 3d4e5f6 --ours --json");

    harness.press(KeyCode::Char('t'));
    assert_eq!(harness.last(), "resolve 3d4e5f6 --theirs --json");
}

#[test]
fn o_on_a_row_that_is_not_a_conflict_does_nothing() {
    let mut harness = loaded();
    let before = harness.lines().len();
    harness.press(KeyCode::Char('o'));

    assert_eq!(harness.lines().len(), before);
    assert!(harness.app.message.is_empty());
}

#[test]
fn q_closes_the_pane_when_nothing_exclusive_is_running() {
    let mut harness = harness();
    assert_eq!(harness.press(KeyCode::Char('q')), After::Quit);
}

#[test]
fn q_mid_push_asks_once_and_quits_on_the_second_press_without_cancelling() {
    let mut harness = harness();
    harness.press(KeyCode::Char('P'));

    assert_eq!(harness.press(KeyCode::Char('q')), After::Stay);
    let Some(Overlay::Confirm(confirm)) = harness.app.overlay.as_ref() else {
        panic!("expected a confirm");
    };
    assert_eq!(
        confirm.question,
        "a push is running; q again quits and lets it finish"
    );

    assert_eq!(harness.press(KeyCode::Char('q')), After::Quit);
    assert_eq!(harness.cancels(), 0, "it killed the push on the way out");
}

#[test]
fn q_mid_read_closes_without_asking() {
    let mut harness = harness();
    harness.app.submit(
        JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    assert_eq!(harness.press(KeyCode::Char('q')), After::Quit);
}

#[test]
fn escape_closes_an_overlay_and_closes_the_pane_when_nothing_is_open() {
    let mut harness = loaded();
    harness.press(KeyCode::Char('v'));
    assert_eq!(harness.press(KeyCode::Esc), After::Stay);
    assert!(harness.app.overlay.is_none());

    assert_eq!(harness.press(KeyCode::Esc), After::Quit);
}

#[test]
fn q_leaves_the_detail_screen_too() {
    let mut harness = loaded();
    harness.press(KeyCode::Enter);
    harness.answer(
        3,
        0,
        r#"{"oid":"5d6e7f8","kind":"task","subject":"x","task":{"done":false,"priority":4}}"#,
        "",
    );
    assert_eq!(harness.press(KeyCode::Char('q')), After::Quit);
}

#[test]
fn q_and_escape_still_quit_while_the_refusal_is_drawn() {
    let mut harness = harness();
    crate::open::start(&mut harness.app);
    harness.answer(0, 0, "dam 0.0.9\n", "");

    assert_eq!(harness.press(KeyCode::Char('q')), After::Quit);
    assert_eq!(harness.press(KeyCode::Esc), After::Quit);
}
