use herdr_damnit_adapters::Config;

use super::*;
use crate::app::tests::harness_with;

const LS: &str = r#"{"objects":[
  {"oid":"1a2b3c4","kind":"task","subject":"ship the pin bump","path":"proj/dotfiles",
   "labels":["a","b"],"task":{"done":false,"priority":1,"due":"2026-09-18"}},
  {"oid":"5d6e7f8","kind":"task","subject":"refresh the roster row","path":"proj/dotfiles",
   "recurrence":"every week","task":{"done":false,"priority":4}},
  {"oid":"9a0b1c2","kind":"task","subject":"water the plants","path":"proj/home",
   "task":{"done":false,"priority":4,"due":"2026-10-02"}}
]}"#;

const CLEAN: &str = r#"{"staged":[],"unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#;

/// The frames the plain spinner steps through. Which one is on screen depends on how many ticks
/// the reads before this one took, so the assertion is that one of them is drawn.
const PLAIN_FRAMES: [&str; 4] = ["|", "/", "-", "\\"];

fn ascii_config() -> Config {
    Config::parse("icons = \"ascii\"\n").expect("parses")
}

fn loaded() -> crate::app::tests::Harness {
    let mut harness = harness_with(ascii_config());
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness.answer(0, 0, CLEAN, "");
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadList,
        herdr_damnit_application::argv::list("!done"),
    );
    harness.answer(1, 0, LS, "");
    harness
}

#[test]
fn the_list_screen_draws_its_headings_marks_and_hint_line() {
    let harness = loaded();
    assert_eq!(
        render_to_text(&harness.app, 32, 8),
        "\
dam  open                 3 open
proj/dotfiles
  ~ refresh the roster row
  ! < 09-18 ship the pin bump @2
proj/home
  > 10-02 water the plants

x X dd p s D l m a S e <CR> <Sp…"
    );
}

#[test]
fn an_empty_list_says_so_rather_than_drawing_nothing() {
    let mut harness = harness_with(ascii_config());
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadList,
        herdr_damnit_application::argv::list("!done"),
    );
    harness.answer(0, 0, r#"{"objects":[]}"#, "");

    assert!(
        render_to_text(&harness.app, 32, 6).contains("nothing in this view"),
        "{}",
        render_to_text(&harness.app, 32, 6)
    );
}

#[test]
fn every_mark_at_once_still_fits_one_line_and_cuts_the_subject() {
    let mut harness = harness_with(ascii_config());
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness.answer(
        0,
        0,
        r#"{"staged":[{"oid":"1a2b3c4","op":"update","subject":"x","fields":["subject"]}],
          "unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#,
        "",
    );
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadList,
        herdr_damnit_application::argv::list("!done"),
    );
    harness.answer(
        1,
        0,
        r#"{"objects":[{"oid":"1a2b3c4","kind":"task",
          "subject":"a subject long enough to need cutting","path":"p",
          "labels":["a","b","c"],"recurrence":"every day",
          "task":{"done":false,"priority":1,"due":"2026-09-01"}}]}"#,
        "",
    );

    let drawn = render_to_text(&harness.app, 32, 6);
    for line in drawn.lines() {
        assert!(
            line.chars().count() <= 32,
            "{line:?} is wider than the pane"
        );
    }
    assert!(drawn.contains("+ ! < 09-01 ~"), "{drawn}");
}

#[test]
fn the_header_carries_the_spinner_and_the_elapsed_time_mid_push() {
    let mut harness = loaded();
    harness.app.submit(
        herdr_damnit_application::JobKind::Exclusive(herdr_damnit_application::SyncKind::Push),
        herdr_damnit_application::argv::push(),
    );

    let drawn = render_to_text(&harness.app, 32, 3);
    let header = drawn.lines().next().expect("a header");
    assert!(header.contains("push"), "{header}");
    assert!(
        PLAIN_FRAMES.iter().any(|frame| header.contains(frame)),
        "{header}"
    );
}

#[test]
fn a_mark_takes_its_own_colour_and_the_subject_stays_plain() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let harness = loaded();
    let mut terminal = Terminal::new(TestBackend::new(32, 8)).expect("a terminal");
    terminal
        .draw(|frame| draw(frame, &harness.app))
        .expect("it drew");
    let buffer = terminal.backend().buffer();

    let palette = crate::theme::resolve(None);
    let row = buffer
        .content()
        .chunks(32)
        .position(|line| {
            line.iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
                .contains("ship the pin")
        })
        .expect("the row is on screen");
    let priority = &buffer.content()[row * 32 + 2];
    assert_eq!(priority.fg, palette.color(herdr_damnit_domain::Slot::Red));
}
