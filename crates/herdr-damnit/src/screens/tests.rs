use std::time::Instant;

use crossterm::event::KeyCode;
use herdr_damnit_adapters::Config;

use super::*;
use crate::app::Screen;
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
    crate::open::start(&mut harness.app);
    harness.answer(0, 0, "dam 0.2.0\n", "");
    harness.answer(1, 0, CLEAN, "");
    harness.answer(2, 0, LS, "");
    harness
}

#[test]
fn the_list_screen_draws_its_headings_marks_and_hint_line() {
    let harness = loaded();
    assert_eq!(
        render_to_text(&harness.app, 32, 8),
        "\
dam  open                 3 open
proj
  dotfiles
    ~ refresh the roster row
    ! < 09-18 ship the pin bump…
  home
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
    let priority = &buffer.content()[row * 32 + 4];
    assert_eq!(priority.fg, palette.color(herdr_damnit_domain::Slot::Red));
}

const FULL_STATUS: &str = r#"{
  "staged":[
    {"oid":"1a2b3c4","op":"create","before":null,
     "after":{"oid":"1a2b3c4","kind":"task","subject":"ship the pin bump"}}],
  "unstaged":[
    {"oid":"9a0b1c2","op":"update",
     "before":{"oid":"9a0b1c2","kind":"task","subject":"water the plants"},
     "after":{"oid":"9a0b1c2","kind":"task","subject":"water the plants"}}],
  "unpushed":[{"remote":"example","commits":1}],
  "conflicts":[{"oid":"3d4e5f6","remote":"example",
    "ours":{"oid":"3d4e5f6","kind":"task","subject":"mine"},
    "theirs":{"oid":"3d4e5f6","kind":"task","subject":"theirs"}}],
  "notices":[{"kind":"pull_failed","remote":"example","why":"unreachable"}]}"#;

fn on_status() -> crate::app::tests::Harness {
    let mut harness = harness_with(ascii_config());
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness.answer(0, 0, FULL_STATUS, "");
    harness.app.screen = Screen::Status;
    harness
}

#[test]
fn tab_cycles_the_three_screens_forward_and_shift_tab_back() {
    let mut harness = loaded();
    assert_eq!(harness.app.screen, Screen::List);

    harness.press(KeyCode::Tab);
    assert_eq!(harness.app.screen, Screen::Status);
    harness.press(KeyCode::Tab);
    assert_eq!(harness.app.screen, Screen::Done);
    harness.press(KeyCode::Tab);
    assert_eq!(harness.app.screen, Screen::List);
    harness.press(KeyCode::BackTab);
    assert_eq!(harness.app.screen, Screen::Done);
}

#[test]
fn each_screen_keeps_its_own_cursor_across_the_cycle() {
    let mut harness = loaded();
    assert!(harness.app.list.move_by(1), "the list has a second row");
    let on_list = harness.app.list.selected_oid().cloned();

    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Tab);

    assert_eq!(harness.app.list.selected_oid().cloned(), on_list);
}

#[test]
fn the_status_screen_draws_the_four_sections_in_dams_order() {
    let harness = on_status();

    let drawn = render_to_text(&harness.app, 32, 12);
    let headings: Vec<&str> = drawn
        .lines()
        .filter(|line| ["Staged", "Working", "Unpushed", "Notices"].contains(line))
        .collect();
    assert_eq!(headings, vec!["Staged", "Working", "Unpushed", "Notices"]);
    assert!(drawn.contains("example: pull failed"), "{drawn}");
}

/// The plan's rule for this screen: a row draws the mark its own change carries, and a row that
/// carries none draws none. A mark taken from the section heading above it would say a remote's
/// failed pull was staged.
#[test]
fn a_status_row_draws_its_own_mark_and_a_row_with_none_draws_none() {
    let harness = on_status();

    let drawn = render_to_text(&harness.app, 32, 12);
    assert!(
        drawn.lines().any(|line| line == "  ^ example  1 commit"),
        "{drawn}"
    );
    assert!(
        drawn
            .lines()
            .any(|line| line.starts_with("  example: pull failed")),
        "{drawn}"
    );
    assert!(
        drawn.lines().any(|line| line.starts_with("  + new")),
        "{drawn}"
    );
}

#[test]
fn the_status_screen_names_itself_and_counts_the_stage_in_dams_own_words() {
    let harness = on_status();

    assert_eq!(harness.app.header(Instant::now()), "dam  status");
    assert_eq!(
        counts(&harness.app),
        "1 staged  1 changed  1 unpushed  1 notice"
    );
}

#[test]
fn a_clean_status_screen_says_so_in_dams_own_words() {
    let mut harness = harness_with(ascii_config());
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness.answer(0, 0, CLEAN, "");
    harness.app.screen = Screen::Status;

    assert!(
        render_to_text(&harness.app, 32, 6).contains("nothing staged, nothing changed"),
        "{}",
        render_to_text(&harness.app, 32, 6)
    );
}

#[test]
fn entering_the_done_screen_reads_the_done_list_and_the_log_once() {
    let mut harness = loaded();
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Tab);

    let lines = harness.lines();
    assert!(lines.contains(&"ls done --json".to_string()), "{lines:?}");
    assert!(lines.contains(&"log --json".to_string()), "{lines:?}");

    let before = lines.len();
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Tab);
    assert_eq!(
        harness.lines().len(),
        before,
        "it re-read a screen it already had"
    );
}

#[test]
fn the_done_screen_draws_the_date_first_newest_first_with_the_uncommitted_ones_on_top() {
    let mut harness = harness_with(ascii_config());
    harness.app.screen = Screen::Done;
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadDone,
        herdr_damnit_application::argv::list("done"),
    );
    harness.answer(
        0,
        0,
        r#"{"objects":[
          {"oid":"aaa","kind":"task","subject":"older","task":{"done":true,"priority":4}},
          {"oid":"bbb","kind":"task","subject":"newer","task":{"done":true,"priority":4}},
          {"oid":"ccc","kind":"task","subject":"just now","task":{"done":true,"priority":4}}]}"#,
        "",
    );
    harness.app.submit(
        herdr_damnit_application::JobKind::ReadLog,
        herdr_damnit_application::argv::log(),
    );
    harness.answer(
        1,
        0,
        r#"{"commits":[
          {"id":"c1","at":"2026-09-18T08:00:00Z","message":"one","changes":[
            {"oid":"aaa","op":"update","fields":["done"],"kind":"task","subject":"older",
             "done":true}]},
          {"id":"c2","at":"2026-09-20T08:00:00Z","message":"two","changes":[
            {"oid":"bbb","op":"update","fields":["done"],"kind":"task","subject":"newer",
             "done":true}]}]}"#,
        "",
    );

    let drawn = render_to_text(&harness.app, 32, 10);
    assert!(
        drawn
            .lines()
            .next()
            .expect("a header")
            .contains("dam  done"),
        "{drawn}"
    );
    let body: Vec<&str> = drawn
        .lines()
        .skip(1)
        .filter(|line| !line.is_empty())
        .collect();
    assert_eq!(body[0], "not committed");
    assert_eq!(body[1], "  just now");
    assert_eq!(body[2], "2026-09-20  newer");
    assert_eq!(body[3], "2026-09-18  older");
}
