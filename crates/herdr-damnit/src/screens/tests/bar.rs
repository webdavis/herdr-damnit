use ratatui::style::Color;

use super::*;
use crate::app::Overlay;
use crate::app::tests::Harness;
use crate::app::tests::views::three_views;
use herdr_damnit_domain::Slot;

const BAR: Color = Color::Rgb(54, 54, 70);

const WIDTH: u16 = 32;

struct Case {
    screen: &'static str,
    harness: fn() -> Harness,
    needle: fn(&Harness) -> String,
    columns: std::ops::Range<u16>,
    marked_column: u16,
    marked_slot: Slot,
}

fn on_list() -> Harness {
    let mut harness = loaded();
    harness.select("1a2b3c4");
    harness
}

fn on_status() -> Harness {
    let mut harness = harness_with(ascii_config());
    harness
        .app
        .submit(herdr_damnit_application::Request::Status);
    harness.answer(&["status"], 0, super::status::FULL_STATUS, "");
    harness.put_on(Screen::Status);
    harness.select("1a2b3c4");
    harness
}

fn on_done() -> Harness {
    let mut harness = loaded();
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Tab);
    harness.answer(&["ls", "done"], 0, super::done::WATERED, "");
    harness.answer(&["log"], 0, r#"{"commits":[]}"#, "");
    harness.select("9a0b1c2");
    harness
}

fn on_picker() -> Harness {
    let mut harness = loaded_with(three_views());
    harness.press(KeyCode::Char('v'));
    harness
}

fn picked_entry(harness: &Harness) -> String {
    let Some(Overlay::View(picker)) = harness.app.showing().overlay() else {
        panic!("expected the view picker");
    };
    picker.entries[picker.selected].text.clone()
}

#[test]
fn the_current_row_sits_on_one_dark_grey_bar_and_each_mark_keeps_its_colour() {
    for case in [
        Case {
            screen: "list",
            harness: on_list,
            needle: |_| "ship the pin".to_string(),
            columns: 0..WIDTH,
            marked_column: 4,
            marked_slot: Slot::Red,
        },
        Case {
            screen: "status",
            harness: on_status,
            needle: |_| "+ new".to_string(),
            columns: 0..WIDTH,
            marked_column: 2,
            marked_slot: Slot::Green,
        },
        Case {
            screen: "done",
            harness: on_done,
            needle: |_| "water the plants".to_string(),
            columns: 0..WIDTH,
            marked_column: 2,
            marked_slot: Slot::Text,
        },
        Case {
            screen: "picker",
            harness: on_picker,
            needle: picked_entry,
            columns: 1..WIDTH - 1,
            marked_column: 3,
            marked_slot: Slot::Text,
        },
    ] {
        let harness = (case.harness)();
        let buffer = drawn(&harness.app, WIDTH, 12);
        let needle = (case.needle)(&harness);
        let row = (1..12)
            .find(|row| line_of(&buffer, *row).contains(&needle))
            .unwrap_or_else(|| panic!("{}: {needle:?} is not on screen", case.screen));
        let line = line_of(&buffer, row);

        for column in case.columns {
            assert_eq!(
                buffer[(column, row)].bg,
                BAR,
                "{} column {column}: {line:?}",
                case.screen
            );
        }
        assert_eq!(
            buffer[(case.marked_column, row)].fg,
            crate::theme::color(case.marked_slot),
            "{}: {line:?}",
            case.screen
        );
    }
}
