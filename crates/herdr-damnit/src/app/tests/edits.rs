use crate::app::Overlay;
use crate::screens::tests::loaded;

use super::*;

const DATE_WORDS: &str = "today, tomorrow, YYYY-MM-DD or YYYY-MM-DDTHH:MM";

fn refusal(rule: &str, said: &str) -> String {
    format!(r#"{{"error":{{"kind":"refused","rule":"{rule}","message":"{said}","oids":[]}}}}"#)
}

fn line_text(harness: &Harness) -> Option<String> {
    match harness.app.overlay.as_ref() {
        Some(Overlay::Line(line)) => Some(line.text.clone()),
        _ => None,
    }
}

#[test]
fn x_completes_and_shift_x_forces_it() {
    let mut harness = loaded();
    harness.select("1a2b3c4");

    harness.press(KeyCode::Char('x'));
    assert_eq!(harness.last(), "done 1a2b3c4 --json");
    harness.press(KeyCode::Char('X'));
    assert_eq!(harness.last(), "done 1a2b3c4 --force --json");
}

#[test]
fn a_refused_completion_says_dams_own_sentence_and_changes_nothing() {
    let mut harness = loaded();
    let before = harness.app.list.object_count();
    harness.press(KeyCode::Char('x'));
    let said = "5d6e7f8 has 2 open children; use --force to complete them too";
    harness.answer(3, 4, "", &refusal("blocked", said));

    assert_eq!(harness.app.message, said);
    assert_eq!(harness.app.list.object_count(), before);
}

#[test]
fn x_with_no_object_under_the_cursor_does_nothing_and_says_nothing() {
    let mut harness = harness();
    harness.press(KeyCode::Char('x'));

    assert!(harness.lines().is_empty());
    assert!(harness.app.message.is_empty());
}

#[test]
fn the_first_d_asks_and_the_second_removes() {
    let mut harness = loaded();
    harness.select("1a2b3c4");

    let before = harness.lines().len();
    harness.press(KeyCode::Char('d'));
    assert_eq!(harness.lines().len(), before, "the first d sent something");
    let Some(Overlay::Confirm(confirm)) = harness.app.overlay.as_ref() else {
        panic!("expected a confirm");
    };
    assert!(
        confirm.question.contains("ship the pin bump"),
        "{}",
        confirm.question
    );
    let drawn = crate::screens::render_to_text(&harness.app, 40, 8);
    assert!(drawn.contains("ship the pin bump"), "{drawn}");

    harness.press(KeyCode::Char('d'));
    assert_eq!(harness.last(), "rm 1a2b3c4 --json");
    assert!(harness.app.overlay.is_none());
}

#[test]
fn any_other_key_dismisses_the_delete_confirm_and_sends_nothing() {
    let mut harness = loaded();
    harness.press(KeyCode::Char('d'));
    let before = harness.lines().len();
    let under = harness.app.list.selected_oid().cloned();
    harness.press(KeyCode::Char('j'));

    assert!(harness.app.overlay.is_none());
    assert_eq!(harness.lines().len(), before);
    assert_eq!(
        harness.app.list.selected_oid().cloned(),
        under,
        "the dismissing key acted"
    );
}

#[test]
fn p_cycles_down_the_priorities_and_wraps_at_one() {
    let mut harness = loaded();
    harness.select("1a2b3c4");
    harness.press(KeyCode::Char('p'));
    assert_eq!(harness.last(), "edit 1a2b3c4 -p 4 --json");

    harness.select("5d6e7f8");
    harness.press(KeyCode::Char('p'));
    assert_eq!(harness.last(), "edit 5d6e7f8 -p 3 --json");
}

#[test]
fn s_opens_a_box_hinting_exactly_the_words_dam_accepts() {
    let mut harness = loaded();
    harness.press(KeyCode::Char('s'));

    let Some(Overlay::Line(line)) = harness.app.overlay.as_ref() else {
        panic!("expected the date box");
    };
    assert_eq!(line.hint, DATE_WORDS);
}

#[test]
fn a_date_typed_into_the_box_becomes_the_due_flag() {
    let mut harness = loaded();
    harness.select("1a2b3c4");
    harness.press(KeyCode::Char('s'));
    harness.type_line("tomorrow");
    harness.press(KeyCode::Enter);

    assert_eq!(harness.last(), "edit 1a2b3c4 --due tomorrow --json");
    assert!(harness.app.overlay.is_none());
}

#[test]
fn an_empty_date_box_clears_the_date_rather_than_sending_an_empty_one() {
    let mut harness = loaded();
    harness.select("1a2b3c4");
    harness.press(KeyCode::Char('s'));
    harness.press(KeyCode::Enter);

    assert_eq!(harness.last(), "edit 1a2b3c4 --no-due --json");
}

#[test]
fn shift_d_is_the_same_box_aimed_at_the_deadline() {
    let mut harness = loaded();
    harness.select("1a2b3c4");
    harness.press(KeyCode::Char('D'));
    harness.type_line("2026-10-02");
    harness.press(KeyCode::Enter);

    assert_eq!(harness.last(), "edit 1a2b3c4 --deadline 2026-10-02 --json");
}

#[test]
fn a_date_dam_cannot_read_comes_back_with_the_line_still_in_the_box() {
    let mut harness = loaded();
    harness.press(KeyCode::Char('s'));
    harness.type_line("next mon");
    harness.press(KeyCode::Enter);
    let said = "cannot read \\\"next mon\\\" as a date";
    harness.answer(
        3,
        1,
        "",
        &format!(r#"{{"error":{{"kind":"parse","rule":null,"message":"{said}","oids":[]}}}}"#),
    );

    assert_eq!(harness.app.message, "cannot read \"next mon\" as a date");
    assert_eq!(line_text(&harness).as_deref(), Some("next mon"));
}

#[test]
fn a_held_store_does_not_bring_the_box_back_so_r_reaches_the_retry() {
    let mut harness = loaded();
    harness.press(KeyCode::Char('s'));
    harness.type_line("today");
    harness.press(KeyCode::Enter);
    harness.answer(3, 1, "", "dam: database is locked");

    assert!(harness.app.overlay.is_none());
}

#[test]
fn a_refused_commit_brings_the_box_back_with_its_message() {
    let mut harness = loaded();
    harness.app.submit(
        JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness.answer(
        3,
        0,
        r#"{"staged":[{"oid":"5d6e7f8","op":"update","subject":"x","fields":[]}],
          "unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#,
        "",
    );
    harness.press(KeyCode::Char('c'));
    harness.type_line("ship it");
    harness.press(KeyCode::Enter);
    harness.answer(4, 4, "", &refusal("nothing_to_commit", "nothing to commit"));

    assert_eq!(line_text(&harness).as_deref(), Some("ship it"));
}
