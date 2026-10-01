use crate::app::Overlay;
use crate::screens::tests::{LS, ascii_config, loaded};

use super::*;

fn working(fields: &str) -> String {
    format!(
        r#"{{"staged":[],"unstaged":[{{"oid":"1a2b3c4","op":"update",
          "subject":"ship the pin bump","fields":{fields}}}],
          "conflicts":[],"notices":[],"unpushed":[]}}"#
    )
}

fn with_working_change(fields: &str) -> Harness {
    let mut harness = loaded();
    harness.app.submit(
        JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness.answer(3, 0, &working(fields), "");
    harness.select("1a2b3c4");
    harness
}

fn question(harness: &Harness) -> String {
    match harness.app.overlay.as_ref() {
        Some(Overlay::Confirm(confirm)) => confirm.question.clone(),
        _ => panic!("expected a confirm"),
    }
}

#[test]
fn the_discard_key_asks_once_naming_the_fields_and_restores_on_the_second_press() {
    let mut harness = with_working_change(r#"["due","priority"]"#);
    let before = harness.lines().len();
    harness.press(KeyCode::Char('!'));

    assert_eq!(harness.lines().len(), before, "the first ! sent something");
    let asked = question(&harness);
    assert!(asked.contains("ship the pin bump"), "{asked}");
    assert!(asked.contains("due, priority"), "{asked}");

    harness.press(KeyCode::Char('!'));
    assert_eq!(harness.last(), "restore 1a2b3c4 --json");
}

#[test]
fn a_change_that_names_no_fields_is_called_its_working_change() {
    let mut harness = with_working_change("[]");
    harness.press(KeyCode::Char('!'));

    assert!(
        question(&harness).contains("its working change"),
        "{}",
        question(&harness)
    );
}

#[test]
fn any_other_key_dismisses_the_discard_confirm_and_sends_nothing() {
    let mut harness = with_working_change(r#"["due"]"#);
    harness.press(KeyCode::Char('!'));
    let before = harness.lines().len();
    harness.press(KeyCode::Char('j'));

    assert!(harness.app.overlay.is_none());
    assert_eq!(harness.lines().len(), before);
}

#[test]
fn the_discard_key_does_nothing_on_a_row_with_no_working_change() {
    let mut harness = with_working_change(r#"["due"]"#);
    harness.select("5d6e7f8");
    let before = harness.lines().len();
    harness.press(KeyCode::Char('!'));

    assert!(harness.app.overlay.is_none());
    assert_eq!(harness.lines().len(), before);
}

#[test]
fn the_discard_key_is_unbound_until_dam_has_named_a_version_with_restore() {
    let mut harness = harness_with(ascii_config());
    harness.app.submit(
        JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness.answer(0, 0, &working(r#"["due"]"#), "");
    harness.app.submit(
        JobKind::ReadList,
        herdr_damnit_application::argv::list("!done"),
    );
    harness.answer(1, 0, LS, "");
    harness.select("1a2b3c4");
    harness.press(KeyCode::Char('!'));

    assert!(harness.app.overlay.is_none());
}
