mod bar;
mod detail;
mod done;
mod frames;
mod legend;
mod list;
mod overlay;
mod palette;
mod refusal;
mod status;

use std::time::Instant;

use crossterm::event::KeyCode;
use herdr_damnit_adapters::Config;

use super::*;
use crate::app::Screen;
use crate::app::tests::harness_with;

pub(crate) const LS: &str = r#"{"objects":[
  {"oid":"1a2b3c4","kind":"task","subject":"ship the pin bump","path":"proj/dotfiles",
   "labels":["a","b"],"task":{"done":false,"priority":1,"due":"2026-09-18"}},
  {"oid":"5d6e7f8","kind":"task","subject":"refresh the roster row","path":"proj/dotfiles",
   "recurrence":"every week","task":{"done":false,"priority":4}},
  {"oid":"9a0b1c2","kind":"task","subject":"water the plants","path":"proj/home",
   "task":{"done":false,"priority":4,"due":"2026-10-02"}}
]}"#;

pub(crate) const CLEAN: &str =
    r#"{"staged":[],"unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#;

pub(crate) fn drawn(app: &crate::app::App, width: u16, height: u16) -> ratatui::buffer::Buffer {
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height))
        .expect("a test terminal");
    terminal.draw(|frame| draw(frame, app)).expect("it drew");
    terminal.backend().buffer().clone()
}

pub(crate) fn line_of(buffer: &ratatui::buffer::Buffer, row: u16) -> String {
    (0..buffer.area.width)
        .map(|column| buffer[(column, row)].symbol())
        .collect()
}

pub(crate) fn ascii_config() -> Config {
    Config::parse("icons = \"ascii\"\n").expect("parses")
}

pub(crate) fn loaded() -> crate::app::tests::Harness {
    loaded_with(ascii_config())
}

pub(crate) fn loaded_with(config: Config) -> crate::app::tests::Harness {
    let mut harness = harness_with(config);
    crate::open::start(&mut harness.app);
    harness.answer(0, 0, "dam 0.2.0\n", "");
    harness.answer(1, 0, CLEAN, "");
    harness.answer(2, 0, LS, "");
    harness
}

#[test]
fn tab_cycles_the_three_screens_forward_and_shift_tab_back() {
    let mut harness = loaded();
    assert_eq!(harness.app.showing().screen(), Screen::List);

    harness.press(KeyCode::Tab);
    assert_eq!(harness.app.showing().screen(), Screen::Status);
    harness.press(KeyCode::Tab);
    assert_eq!(harness.app.showing().screen(), Screen::Done);
    harness.press(KeyCode::Tab);
    assert_eq!(harness.app.showing().screen(), Screen::List);
    harness.press(KeyCode::BackTab);
    assert_eq!(harness.app.showing().screen(), Screen::Done);
}

#[test]
fn each_screen_keeps_its_own_cursor_across_the_cycle() {
    let mut harness = loaded();
    assert!(harness.cursor_mut().move_by(1), "the list has a second row");
    let on_list = harness.app.list().cursor().selected_oid().cloned();

    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Tab);

    assert_eq!(harness.app.list().cursor().selected_oid().cloned(), on_list);
}

#[test]
fn a_message_takes_the_legend_row_until_the_next_key() {
    let mut harness = loaded();
    harness.app.say("the config has 1 views.".to_string());
    let drawn = render_to_text(&harness.app, 32, 8);
    assert_eq!(drawn.lines().last(), Some("the config has 1 views."));

    harness.press(KeyCode::Char('j'));
    let drawn = render_to_text(&harness.app, 32, 8);
    assert!(
        drawn
            .lines()
            .last()
            .is_some_and(|line| line == "j/k move  space stage  q close"),
        "{drawn}"
    );
}
