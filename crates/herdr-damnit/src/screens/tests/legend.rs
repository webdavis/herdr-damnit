use super::*;
use crate::overlay::{
    Confirm, ConfirmPurpose, LineBox, LinePurpose, NoteBox, Overlay, Picker, PickerEntry,
};
use herdr_damnit_domain::{Oid, Slot};

const WIDTH: u16 = 70;

fn bottom_row(harness: &crate::app::tests::Harness) -> String {
    render_to_text(&harness.app, WIDTH, 8)
        .lines()
        .last()
        .unwrap_or_default()
        .to_string()
}

fn with_overlay(overlay: Overlay) -> crate::app::tests::Harness {
    let mut harness = loaded();
    harness.app.overlay = Some(overlay);
    harness
}

fn picker() -> Overlay {
    Overlay::View(Picker {
        title: "views".to_string(),
        entries: vec![PickerEntry {
            text: "open".to_string(),
            marked: false,
        }],
        selected: 0,
    })
}

fn due_box() -> Overlay {
    Overlay::Line(LineBox {
        title: "due".to_string(),
        hint: String::new(),
        text: String::new(),
        purpose: LinePurpose::Due(Oid::new("1a2b3c4")),
    })
}

fn note_box() -> Overlay {
    Overlay::Note(NoteBox {
        oid: Oid::new("1a2b3c4"),
        text: String::new(),
    })
}

fn delete_confirm() -> Overlay {
    Overlay::Confirm(Confirm {
        question: "remove?".to_string(),
        key: 'd',
        purpose: ConfirmPurpose::Delete(Oid::new("1a2b3c4")),
    })
}

fn on(screen: Screen) -> crate::app::tests::Harness {
    let mut harness = loaded();
    harness.app.screen = screen;
    harness
}

fn over_a_detail() -> crate::app::tests::Harness {
    let mut harness = loaded();
    harness.press(KeyCode::Enter);
    harness.answer(
        3,
        0,
        r#"{"oid":"5d6e7f8","kind":"task","subject":"x","task":{"done":false,"priority":4}}"#,
        "",
    );
    harness
}

#[test]
fn the_bottom_row_on_each_screen_is_that_screens_legend() {
    for (harness, expected) in [
        (on(Screen::List), "j/k move  space stage  x done  q close"),
        (
            on(Screen::Status),
            "j/k move  space stage  c commit  q close",
        ),
        (on(Screen::Done), "j/k move  enter open  v view  q close"),
        (over_a_detail(), "esc back  q close"),
    ] {
        assert_eq!(bottom_row(&harness), expected);
    }
}

#[test]
fn the_bottom_row_under_an_overlay_is_that_overlays_legend() {
    for (overlay, expected) in [
        (picker(), "j/k move  enter take  esc cancel"),
        (due_box(), "enter send  esc cancel"),
        (note_box(), "enter new line  ctrl+d send  esc cancel"),
        (delete_confirm(), "d confirm  esc cancel"),
    ] {
        assert_eq!(bottom_row(&with_overlay(overlay)), expected);
    }
}

#[test]
fn the_legend_is_drawn_in_the_faint_color() {
    let buffer = drawn(&loaded().app, WIDTH, 8);
    assert_eq!(buffer[(0, 7)].symbol(), "j");
    assert_eq!(buffer[(0, 7)].fg, crate::theme::color(Slot::Dim1));
}
