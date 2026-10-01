use crate::app::Overlay;
use crate::overlay::LinePurpose;
use crate::screens::tests::loaded;

use super::*;

const ONE_STAGED: &str = r#"{"staged":[
  {"oid":"5d6e7f8","op":"update","subject":"refresh the roster row","fields":["due"]}],
  "unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#;

fn staged() -> Harness {
    let mut harness = loaded();
    harness.app.submit(
        JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness.answer(3, 0, ONE_STAGED, "");
    harness
}

#[test]
fn space_stages_an_unstaged_row_and_unstages_a_staged_one() {
    let mut harness = staged();
    harness.select("1a2b3c4");
    harness.press(KeyCode::Char(' '));
    assert_eq!(harness.last(), "add 1a2b3c4 --json");

    harness.select("5d6e7f8");
    harness.press(KeyCode::Char(' '));
    assert_eq!(harness.last(), "reset 5d6e7f8 --json");
}

#[test]
fn space_with_no_object_under_the_cursor_does_nothing_and_says_nothing() {
    let mut harness = harness();
    harness.press(KeyCode::Char(' '));

    assert!(harness.lines().is_empty());
    assert!(harness.app.message.is_empty());
}

#[test]
fn shift_a_stages_everything_and_shift_u_unstages_everything() {
    let mut harness = staged();
    harness.press(KeyCode::Char('A'));
    assert_eq!(harness.last(), "add -A --json");
    harness.press(KeyCode::Char('U'));
    assert_eq!(harness.last(), "reset --json");
}

#[test]
fn c_opens_the_commit_box_headed_with_the_count_dam_reported() {
    let mut harness = staged();
    harness.press(KeyCode::Char('c'));

    let Some(Overlay::Line(line)) = harness.app.overlay.as_ref() else {
        panic!("expected the commit box");
    };
    assert_eq!(line.title, "commit 1 staged change");
    assert_eq!(line.purpose, LinePurpose::Commit);
    let drawn = crate::screens::render_to_text(&harness.app, 32, 8);
    assert!(drawn.contains("commit 1 staged change"), "{drawn}");
}

#[test]
fn a_commit_sends_the_line_as_one_argument_and_closes_the_box() {
    let mut harness = staged();
    harness.press(KeyCode::Char('c'));
    harness.type_line("refresh the roster row");
    harness.press(KeyCode::Enter);

    assert_eq!(
        harness.log_argv_last(),
        vec![
            "commit".to_string(),
            "-m".to_string(),
            "refresh the roster row".to_string(),
            "--json".to_string()
        ]
    );
    assert!(harness.app.overlay.is_none());
}

#[test]
fn a_blank_commit_message_is_refused_in_the_pane_and_nothing_is_spawned() {
    let mut harness = staged();
    harness.press(KeyCode::Char('c'));
    harness.press(KeyCode::Char(' '));
    let before = harness.lines().len();
    harness.press(KeyCode::Enter);

    assert_eq!(harness.app.message, "a commit needs a message.");
    assert_eq!(harness.lines().len(), before);
    assert!(harness.app.overlay.is_some(), "the box closed on a refusal");
}

#[test]
fn escape_closes_the_commit_box_and_spawns_nothing() {
    let mut harness = staged();
    harness.press(KeyCode::Char('c'));
    let before = harness.lines().len();
    harness.press(KeyCode::Esc);

    assert!(harness.app.overlay.is_none());
    assert_eq!(harness.lines().len(), before);
}

#[test]
fn c_with_nothing_staged_says_what_to_press_and_opens_no_box() {
    let mut harness = loaded();
    harness.press(KeyCode::Char('c'));

    assert_eq!(
        harness.app.message,
        "nothing staged; press <Space> on a row or A to stage everything."
    );
    assert!(harness.app.overlay.is_none());
}

#[test]
fn backspace_takes_a_character_off_the_line() {
    let mut harness = staged();
    harness.press(KeyCode::Char('c'));
    harness.type_line("ab");
    harness.press(KeyCode::Backspace);
    harness.press(KeyCode::Enter);

    assert_eq!(harness.last(), "commit -m a --json");
}
