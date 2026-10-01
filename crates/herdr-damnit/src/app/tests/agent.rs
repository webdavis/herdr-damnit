use crossterm::event::KeyModifiers;

use crate::app::Overlay;
use crate::screens::tests::{ascii_config, loaded, loaded_with};

use super::*;

fn send(harness: &mut Harness) {
    harness
        .app
        .key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
}

#[test]
fn shift_s_opens_the_note_box_over_the_object_under_the_cursor() {
    let mut harness = loaded();
    harness.select("1a2b3c4");
    harness.press(KeyCode::Char('S'));

    let Some(Overlay::Note(note)) = harness.app.overlay.as_ref() else {
        panic!("expected the note box");
    };
    assert_eq!(note.oid.as_str(), "1a2b3c4");
    let drawn = crate::screens::render_to_text(&harness.app, 40, 8);
    assert!(drawn.contains("<C-d> sends"), "{drawn}");
}

#[test]
fn enter_opens_a_line_in_the_note_box_rather_than_sending() {
    let mut harness = loaded();
    harness.press(KeyCode::Char('S'));
    harness.type_line("a");
    harness.press(KeyCode::Enter);
    harness.type_line("b");

    let Some(Overlay::Note(note)) = harness.app.overlay.as_ref() else {
        panic!("the note box closed on Enter");
    };
    assert_eq!(note.text, "a\nb");
    assert!(harness.herdr_calls().is_empty(), "it sent on Enter");
}

#[test]
fn control_d_sends_the_brief_with_the_note_and_names_the_agent() {
    let mut harness = loaded();
    harness.select("1a2b3c4");
    harness.press(KeyCode::Char('S'));
    harness.type_line("start here");
    send(&mut harness);

    let sent = harness.herdr_calls();
    let paste = sent
        .iter()
        .find(|call| call.starts_with("pane send-text w1:p2"))
        .unwrap_or_else(|| panic!("{sent:?}"));
    assert!(paste.contains("ship the pin bump"), "{paste}");
    assert!(paste.contains("note: start here"), "{paste}");
    assert_eq!(harness.app.message, "sent to planner");
    assert!(harness.app.overlay.is_none());
}

#[test]
fn a_successful_send_writes_the_configured_handoff_label() {
    let mut harness = loaded();
    harness.select("1a2b3c4");
    harness.press(KeyCode::Char('S'));
    send(&mut harness);

    assert_eq!(harness.last(), "edit 1a2b3c4 --label handed-off --json");
}

#[test]
fn a_refused_label_does_not_pretend_the_send_failed() {
    let mut harness = loaded();
    harness.press(KeyCode::Char('S'));
    send(&mut harness);
    harness.answer(3, 2, "", "dam: \"handed-off\" clashes with \"handed-back\"");

    assert_eq!(
        harness.app.message,
        "sent to planner, label refused: \"handed-off\" clashes with \"handed-back\"."
    );
}

#[test]
fn an_empty_handoff_label_writes_nothing_and_the_status_line_is_the_whole_record() {
    let config = Config::parse("icons = \"ascii\"\nhandoff_label = \"\"\n").expect("parses");
    let mut harness = loaded_with(config);
    harness.press(KeyCode::Char('S'));
    let before = harness.lines().len();
    send(&mut harness);

    assert_eq!(harness.lines().len(), before);
    assert_eq!(harness.app.message, "sent to planner");
}

#[test]
fn escape_throws_the_draft_away_and_sends_nothing() {
    let mut harness = loaded();
    harness.press(KeyCode::Char('S'));
    harness.type_line("x");
    harness.press(KeyCode::Esc);

    assert!(harness.app.overlay.is_none());
    assert!(harness.herdr_calls().is_empty());
}

#[test]
fn a_workspace_with_no_agent_pane_says_so() {
    let mut harness = harness_with_listing(ascii_config(), NO_AGENT);
    crate::open::start(&mut harness.app);
    harness.answer(0, 0, "dam 0.2.0\n", "");
    harness.answer(1, 0, crate::screens::tests::CLEAN, "");
    harness.answer(2, 0, crate::screens::tests::LS, "");
    harness.press(KeyCode::Char('S'));
    send(&mut harness);

    assert_eq!(harness.app.message, "no agent pane in this workspace.");
}
