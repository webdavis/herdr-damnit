use ratatui::style::{Color, Modifier};

use super::*;
use crate::app::tests::Harness;
use crate::theme::BASE;
use herdr_damnit_domain::Slot;

const SLOTS: [(Slot, &str); 9] = [
    (Slot::Text, "text"),
    (Slot::Dim1, "dim"),
    (Slot::Red, "red"),
    (Slot::Green, "green"),
    (Slot::Yellow, "yellow"),
    (Slot::Orange, "orange"),
    (Slot::Purple, "purple"),
    (Slot::Blue, "blue"),
    (Slot::Cyan, "cyan"),
];

fn named(colour: Color) -> String {
    if colour == BASE {
        return "base".to_string();
    }
    if colour == Color::Rgb(0x36, 0x36, 0x46) {
        return "bar".to_string();
    }
    SLOTS
        .iter()
        .find(|(slot, _)| crate::theme::color(*slot) == colour)
        .map_or_else(|| format!("{colour:?}"), |(_, name)| name.to_string())
}

fn frame(harness: &Harness, width: u16, height: u16) -> Vec<String> {
    let buffer = drawn(&harness.app, width, height);
    (0..height)
        .map(|row| {
            let mut runs: Vec<(String, u16, u16)> = Vec::new();
            for column in 0..width {
                let cell = &buffer[(column, row)];
                let bold = if cell.modifier.contains(Modifier::BOLD) {
                    "+b"
                } else {
                    ""
                };
                let look = format!("{}/{}{bold}", named(cell.fg), named(cell.bg));
                match runs.last_mut() {
                    Some((last, _, end)) if *last == look => *end = column + 1,
                    _ => runs.push((look, column, column + 1)),
                }
            }
            let runs: Vec<String> = runs
                .into_iter()
                .map(|(look, start, end)| format!("{look} {start}-{end}"))
                .collect();
            format!("{}|{}", line_of(&buffer, row).trim_end(), runs.join(", "))
        })
        .collect()
}

fn on_status() -> Harness {
    let mut harness = loaded();
    harness
        .app
        .submit(herdr_damnit_application::Request::Status);
    harness.answer(3, 0, super::status::FULL_STATUS, "");
    harness.put_on(Screen::Status);
    harness.select("9a0b1c2");
    harness
}

fn on_done() -> Harness {
    let mut harness = harness_with(ascii_config());
    harness.put_on(Screen::Done);
    harness.app.submit(herdr_damnit_application::Request::Done);
    harness.answer(
        0,
        0,
        r#"{"objects":[
          {"oid":"aaa","kind":"task","subject":"older","task":{"done":true,"priority":4}},
          {"oid":"ccc","kind":"task","subject":"just now","task":{"done":true,"priority":4}}]}"#,
        "",
    );
    harness.app.submit(herdr_damnit_application::Request::Log);
    harness.answer(
        1,
        0,
        r#"{"commits":[{"id":"c1","at":"2026-09-18T08:00:00Z","message":"one","changes":[
            {"oid":"aaa","op":"update","fields":["done"],"kind":"task","subject":"older",
             "done":true}]}]}"#,
        "",
    );
    harness.select("aaa");
    harness
}

#[test]
fn the_list_frame_draws_headings_bold_marks_in_colour_and_the_bar_on_the_cursor() {
    let mut harness = loaded();
    harness.select("5d6e7f8");
    assert_eq!(
        frame(&harness, 48, 8),
        [
            "dam  open                                 3 open|text/base 0-48",
            "proj|blue/base+b 0-48",
            "  dotfiles|blue/base+b 0-48",
            "    ~ refresh the roster row|text/bar 0-4, green/bar 4-6, text/bar 6-48",
            "    ! < 09-18 ship the pin bump @2|text/base 0-4, red/base 4-14, text/base 14-31, purple/base 31-34, text/base 34-48",
            "  home|blue/base+b 0-48",
            "    > 10-02 water the plants|text/base 0-4, blue/base 4-12, text/base 12-48",
            "j/k move  space stage  x done  q close|dim/base 0-38, text/base 38-48",
        ]
    );
}

#[test]
fn the_status_frame_draws_each_section_with_its_mark_and_the_bar_on_the_cursor() {
    assert_eq!(
        frame(&on_status(), 48, 11),
        [
            "dam  s…1 staged  1 changed  1 unpushed  1 notice|text/base 0-48",
            "Staged|blue/base+b 0-48",
            "  + new      1a2b3c4  ship the pin bump|text/base 0-2, green/base 2-4, text/base 4-48",
            "Working|blue/base+b 0-48",
            "  * changed  9a0b1c2  water the plants|text/bar 0-2, yellow/bar 2-4, text/bar 4-48",
            "Unpushed|blue/base+b 0-48",
            "  ^ example  1 commit|text/base 0-2, cyan/base 2-4, text/base 4-48",
            "Notices|blue/base+b 0-48",
            "  x 3d4e5f6  example  ours: \"mine\"  theirs: \"th…|text/base 0-2, red/base 2-4, text/base 4-48",
            "  example: pull failed: unreachable|text/base 0-48",
            "j/k move  space stage  c commit  q close|dim/base 0-40, text/base 40-48",
        ]
    );
}

#[test]
fn the_done_frame_draws_the_date_faint_and_the_bar_on_the_cursor() {
    assert_eq!(
        frame(&on_done(), 48, 5),
        [
            "dam  done                                 2 done|text/base 0-48",
            "not committed|blue/base+b 0-48",
            "  just now|text/base 0-48",
            "2026-09-18  older|dim/bar 0-12, text/bar 12-48",
            "j/k move  enter open  v view  q close|dim/base 0-37, text/base 37-48",
        ]
    );
}

#[test]
fn a_clean_status_frame_and_an_empty_done_frame_draw_no_bar() {
    let mut clean = loaded();
    clean.put_on(Screen::Status);
    let mut empty = loaded();
    empty.put_on(Screen::Done);
    assert_eq!(
        [frame(&clean, 48, 3), frame(&empty, 48, 3)],
        [
            [
                "dam  status|text/base 0-48",
                "nothing staged, nothing changed|text/base 0-48",
                "j/k move  space stage  c commit  q close|dim/base 0-40, text/base 40-48",
            ],
            [
                "dam  done                                 0 done|text/base 0-48",
                "nothing completed yet|dim/base 0-21, text/base 21-48",
                "j/k move  enter open  v view  q close|dim/base 0-37, text/base 37-48",
            ],
        ]
    );
}
