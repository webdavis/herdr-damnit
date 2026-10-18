use crossterm::event::KeyModifiers;

use crate::app::Overlay;
use crate::screens::tests::loaded;

use super::*;

const UNREADABLE_DATE: &str =
    r#"{"error":{"kind":"parse","rule":null,"message":"cannot read the date","oids":[]}}"#;

fn set_due(harness: &mut Harness, oid: &str, when: &str) {
    harness.select(oid);
    harness.press(KeyCode::Char('s'));
    harness.type_line(when);
    harness.press(KeyCode::Enter);
}

fn hand_off(harness: &mut Harness) {
    harness.press(KeyCode::Char('S'));
    harness
        .app
        .key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
}

#[test]
fn a_refused_date_brings_back_its_own_box_while_a_later_one_is_in_flight() {
    let mut harness = loaded();
    set_due(&mut harness, "1a2b3c4", "next mon");
    set_due(&mut harness, "5d6e7f8", "today");

    harness.answer(&["edit", "1a2b3c4"], 1, "", UNREADABLE_DATE);

    let Some(Overlay::Line(line)) = harness.app.showing().overlay() else {
        panic!("the refused box did not come back");
    };
    assert_eq!(line.text, "next mon");
}

#[test]
fn a_refused_label_names_its_agent_while_a_later_hand_off_is_in_flight() {
    let mut harness = loaded();
    hand_off(&mut harness);
    hand_off(&mut harness);

    harness.answer(
        &["edit"],
        2,
        "",
        "dam: \"handed-off\" clashes with \"handed-back\"",
    );

    assert_eq!(
        harness.app.message(),
        "sent to planner, label refused: \"handed-off\" clashes with \"handed-back\"."
    );
}

#[test]
fn a_job_that_dies_unanswered_is_settled_and_its_failure_line_is_shown_once() {
    let mut harness = loaded();
    set_due(&mut harness, "1a2b3c4", "next mon");
    harness.hang_up();

    assert_eq!(harness.app.work.pending_count(), 0);
    assert_eq!(harness.app.message(), "dam was killed before it answered.");
    let Some(Overlay::Line(line)) = harness.app.showing().overlay() else {
        panic!("the box did not come back");
    };
    assert_eq!(line.text, "next mon");

    harness.app.tick(Instant::now());
    assert_eq!(harness.app.message(), "dam was killed before it answered.");
    assert_eq!(harness.lines().len(), 4);
}
