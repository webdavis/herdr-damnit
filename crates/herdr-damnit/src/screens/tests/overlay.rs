use super::*;
use crate::app::tests::views::three_views;

const STAGED: &str = r#"{"staged":[
  {"oid":"1a2b3c4","op":"update","subject":"ship the pin bump","fields":["due"]},
  {"oid":"5d6e7f8","op":"update","subject":"refresh the roster row","fields":["due"]}],
  "unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#;

fn rows(harness: &crate::app::tests::Harness, height: u16) -> Vec<String> {
    render_to_text(&harness.app, 32, height)
        .lines()
        .map(str::to_string)
        .chain(std::iter::repeat(String::new()))
        .take(height as usize)
        .collect()
}

fn two_views() -> crate::app::tests::Harness {
    loaded_with(
        Config::parse("icons = \"ascii\"\n[[views]]\nname = \"today\"\nquery = \"due:today\"\n")
            .expect("parses"),
    )
}

fn box_rows(drawn: &[String]) -> Vec<usize> {
    let top = drawn.iter().position(|row| row.starts_with('┌'));
    let bottom = drawn.iter().position(|row| row.starts_with('└'));
    match (top, bottom) {
        (Some(top), Some(bottom)) => (top..=bottom).collect(),
        _ => panic!("no box in {drawn:#?}"),
    }
}

#[test]
fn the_view_picker_is_its_entries_tall_and_sits_on_the_legend() {
    let mut harness = two_views();
    harness.press(KeyCode::Char('v'));
    let drawn = rows(&harness, 20);

    assert_eq!(box_rows(&drawn), vec![15, 16, 17, 18], "{drawn:#?}");
    assert_eq!(drawn[19], "j/k move  enter take  esc cancel");
}

#[test]
fn the_rows_above_an_open_overlay_are_the_rows_before_it_opened() {
    let mut harness = loaded_with(three_views());
    let before = rows(&harness, 20);
    harness.press(KeyCode::Char('v'));
    let after = rows(&harness, 20);

    let top = box_rows(&after)[0];
    assert_eq!(after[..top], before[..top]);
}

#[test]
fn the_due_box_leaves_the_task_it_dates_in_sight() {
    let mut harness = loaded();
    harness.select("1a2b3c4");
    harness.press(KeyCode::Char('s'));
    let drawn = rows(&harness, 20);

    assert!(
        drawn.iter().any(|row| row.contains("ship the pin bump")),
        "{drawn:#?}"
    );
    assert_eq!(box_rows(&drawn), vec![16, 17, 18]);
}

#[test]
fn a_box_that_would_cover_the_cursor_sits_at_the_top_of_the_body() {
    let mut harness = loaded();
    harness.select("9a0b1c2");
    let before = rows(&harness, 8);
    let cursor = before
        .iter()
        .position(|row| row.contains("water the plants"))
        .expect("the cursor row is drawn");
    harness.press(KeyCode::Char('s'));
    let drawn = rows(&harness, 8);

    assert_eq!(box_rows(&drawn), vec![1, 2, 3], "{drawn:#?}");
    assert_eq!(drawn[cursor], before[cursor]);
}

#[test]
fn the_commit_box_leaves_the_staged_rows_in_sight() {
    let mut harness = loaded();
    harness
        .app
        .submit(herdr_damnit_application::Request::Status);
    harness.answer(&["status"], 0, STAGED, "");
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Char('c'));
    let drawn = rows(&harness, 20);

    for oid in ["1a2b3c4", "5d6e7f8"] {
        assert!(drawn.iter().any(|row| row.contains(oid)), "{drawn:#?}");
    }
    assert_eq!(box_rows(&drawn), vec![16, 17, 18]);
}

#[test]
fn the_delete_confirm_asks_without_naming_a_key() {
    let mut harness = loaded();
    harness.select("1a2b3c4");
    harness.press(KeyCode::Char('d'));
    let drawn = rows(&harness, 20);

    assert_eq!(box_rows(&drawn), vec![15, 16, 17, 18], "{drawn:#?}");
    assert_eq!(drawn[16], "│remove 1a2b3c4 ship the pin   │");
    assert_eq!(drawn[17], "│bump?                         │");
    assert_eq!(drawn[19], "d confirm  esc cancel");
}

#[test]
fn the_note_box_borders_name_no_key() {
    let mut harness = loaded();
    harness.open(crate::overlay::Overlay::Note(crate::overlay::NoteBox {
        oid: herdr_damnit_domain::Oid::new("1a2b3c4"),
        text: "hi".to_string(),
    }));
    let drawn = rows(&harness, 20);

    assert_eq!(box_rows(&drawn), vec![16, 17, 18], "{drawn:#?}");
    assert_eq!(drawn[17], "│hi_                           │");
    assert_eq!(drawn[18], format!("└{}┘", "─".repeat(30)));
}

#[test]
fn a_question_one_border_too_wide_for_a_line_gets_a_second_row() {
    let mut harness = loaded();
    harness.select("9a0b1c2");
    harness.press(KeyCode::Char('d'));
    let drawn = rows(&harness, 20);

    assert_eq!(box_rows(&drawn), vec![15, 16, 17, 18], "{drawn:#?}");
    assert_eq!(drawn[16], "│remove 9a0b1c2 water the      │");
    assert_eq!(drawn[17], "│plants?                       │");
}
