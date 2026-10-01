use crate::app::Overlay;
use crate::overlay::Picker;
use crate::screens::tests::loaded;

use super::*;

fn texts(picker: &Picker) -> Vec<&str> {
    picker
        .entries
        .iter()
        .map(|entry| entry.text.as_str())
        .collect()
}

#[test]
fn the_label_picker_offers_every_label_in_the_view_and_marks_the_ones_this_object_carries() {
    let mut harness = loaded();
    harness.select("5d6e7f8");
    harness.press(KeyCode::Char('l'));

    let Some(Overlay::Label(_, picker)) = harness.app.overlay.as_ref() else {
        panic!("expected the label picker");
    };
    assert_eq!(texts(picker), vec!["a", "b"]);
    assert!(picker.entries.iter().all(|entry| !entry.marked));
    let drawn = crate::screens::render_to_text(&harness.app, 32, 8);
    assert!(drawn.contains("label"), "{drawn}");
}

#[test]
fn taking_a_marked_label_off_sends_unlabel_and_adding_one_sends_label() {
    let mut harness = loaded();
    harness.select("1a2b3c4");
    harness.press(KeyCode::Char('l'));
    harness.press(KeyCode::Enter);
    assert_eq!(harness.last(), "edit 1a2b3c4 --unlabel a --json");
    harness.press(KeyCode::Esc);

    harness.select("5d6e7f8");
    harness.press(KeyCode::Char('l'));
    harness.press(KeyCode::Enter);
    assert_eq!(harness.last(), "edit 5d6e7f8 --label a --json");
}

#[test]
fn the_label_picker_stays_open_after_a_pick_with_the_mark_flipped() {
    let mut harness = loaded();
    harness.press(KeyCode::Char('l'));
    harness.press(KeyCode::Enter);

    let Some(Overlay::Label(_, picker)) = harness.app.overlay.as_ref() else {
        panic!("the picker closed");
    };
    assert!(picker.entries[0].marked);
}

#[test]
fn the_path_picker_offers_every_path_and_every_parent_prefix() {
    let mut harness = loaded();
    harness.select("1a2b3c4");
    harness.press(KeyCode::Char('m'));

    let Some(Overlay::Path(_, picker)) = harness.app.overlay.as_ref() else {
        panic!("expected the path picker");
    };
    assert_eq!(texts(picker), vec!["proj", "proj/dotfiles", "proj/home"]);
    assert_eq!(
        picker.selected, 1,
        "the object's own path is under the cursor"
    );
}

#[test]
fn picking_a_path_moves_the_object_and_closes_the_picker() {
    let mut harness = loaded();
    harness.select("1a2b3c4");
    harness.press(KeyCode::Char('m'));
    harness.press(KeyCode::Char('j'));
    harness.press(KeyCode::Enter);

    assert_eq!(harness.last(), "mv 1a2b3c4 proj/home --json");
    assert!(harness.app.overlay.is_none());
}

#[test]
fn a_creates_an_object_under_the_path_of_the_row_the_cursor_is_on() {
    let mut harness = loaded();
    harness.select("9a0b1c2");
    harness.press(KeyCode::Char('a'));
    harness.type_line("file taxes");
    harness.press(KeyCode::Enter);

    assert_eq!(harness.last(), "new file taxes --path proj/home --json");
}

#[test]
fn a_new_object_is_named_in_the_status_line_by_the_oid_dam_made() {
    let mut harness = loaded();
    harness.press(KeyCode::Char('a'));
    harness.type_line("file taxes");
    harness.press(KeyCode::Enter);
    harness.answer(
        3,
        0,
        r#"{"oid":"7a8b9c0d1e","kind":"task","subject":"file taxes","task":{"done":false,"priority":4}}"#,
        "",
    );

    assert_eq!(harness.app.message, "made 7a8b9c0");
}

#[test]
fn an_empty_new_subject_is_refused_in_the_pane_and_nothing_is_spawned() {
    let mut harness = loaded();
    harness.press(KeyCode::Char('a'));
    let before = harness.lines().len();
    harness.press(KeyCode::Enter);

    assert_eq!(harness.app.message, "a new task needs a subject.");
    assert_eq!(harness.lines().len(), before);
}
