use std::time::{Duration, Instant};

use crate::app::Overlay;
use crate::screens::tests::{loaded, loaded_with};

use super::*;

const TWO_CHANGES: &str = r#"{"staged":[
  {"oid":"1a2b3c4","op":"update","subject":"ship the pin bump","fields":["due"]},
  {"oid":"5d6e7f8","op":"update","subject":"refresh the roster row","fields":["due"]}],
  "unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#;

fn three_views() -> Config {
    Config::parse(
        "icons = \"ascii\"\n\
         [[views]]\nname = \"today\"\nquery = \"!done & due:today\"\n\
         [[views]]\nname = \"deep\"\nquery = \"!done & effort:deep\"\n",
    )
    .expect("parses")
}

fn picker_names(app: &App) -> (Vec<String>, usize) {
    let Some(Overlay::View(picker)) = app.overlay.as_ref() else {
        panic!("expected the view picker");
    };
    let names = picker.entries.iter().map(|entry| entry.text.clone());
    (names.collect(), picker.selected)
}

#[test]
fn j_and_k_move_the_cursor_and_stop_at_either_end() {
    let mut harness = loaded();
    let first = harness.app.list.selected_oid().cloned();

    harness.press(KeyCode::Char('j'));
    assert_ne!(harness.app.list.selected_oid().cloned(), first);
    harness.press(KeyCode::Up);
    assert_eq!(harness.app.list.selected_oid().cloned(), first);
    harness.press(KeyCode::Char('k'));
    assert_eq!(harness.app.list.selected_oid().cloned(), first);
    harness.press(KeyCode::Down);
    harness.press(KeyCode::Char('j'));
    harness.press(KeyCode::Char('j'));
    assert_eq!(
        harness.app.list.selected_oid().map(|oid| oid.as_str()),
        Some("9a0b1c2")
    );
}

#[test]
fn the_status_screen_has_its_own_cursor_and_enter_reads_the_row_under_it() {
    let mut harness = loaded();
    harness.app.submit(
        JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness.answer(3, 0, TWO_CHANGES, "");
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Char('j'));
    harness.press(KeyCode::Enter);

    assert_eq!(harness.last(), "show 5d6e7f8 --json");
}

#[test]
fn a_number_key_shows_that_view_and_reads_its_query() {
    let mut harness = loaded_with(three_views());
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Char('2'));

    assert_eq!(harness.app.views.current().name, "today");
    assert_eq!(harness.last(), "ls !done & due:today --json");
    assert_eq!(harness.app.screen, Screen::List);
}

#[test]
fn a_number_with_no_view_behind_it_says_how_many_there_are_and_reads_nothing() {
    let mut harness = loaded_with(three_views());
    let before = harness.lines().len();
    harness.press(KeyCode::Char('7'));

    assert_eq!(harness.app.message, "the config has 3 views.");
    assert_eq!(harness.lines().len(), before);
}

#[test]
fn the_number_of_the_showing_view_reads_nothing_and_says_nothing() {
    let mut harness = loaded_with(three_views());
    let before = harness.lines().len();
    harness.press(KeyCode::Char('1'));

    assert!(harness.app.message.is_empty());
    assert_eq!(harness.lines().len(), before);
}

#[test]
fn v_opens_the_view_picker_with_the_showing_view_under_the_cursor() {
    let mut harness = loaded_with(three_views());
    harness.press(KeyCode::Char('3'));
    harness.press(KeyCode::Char('v'));

    assert_eq!(
        picker_names(&harness.app),
        (vec!["open".into(), "today".into(), "deep".into()], 2)
    );
    let drawn = crate::screens::render_to_text(&harness.app, 32, 10);
    assert!(drawn.contains("today"), "{drawn}");
}

#[test]
fn v_on_the_status_screen_opens_nothing() {
    let mut harness = loaded_with(three_views());
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Char('v'));

    assert!(harness.app.overlay.is_none());
}

#[test]
fn the_view_picker_takes_the_entry_under_the_cursor_and_closes() {
    let mut harness = loaded_with(three_views());
    harness.press(KeyCode::Char('v'));
    harness.press(KeyCode::Char('j'));
    harness.press(KeyCode::Enter);

    assert!(harness.app.overlay.is_none());
    assert_eq!(harness.app.views.current().name, "today");
    assert_eq!(harness.last(), "ls !done & due:today --json");
}

#[test]
fn escape_closes_the_picker_and_changes_no_view() {
    let mut harness = loaded_with(three_views());
    harness.press(KeyCode::Char('v'));
    harness.press(KeyCode::Char('j'));
    let before = harness.lines().len();
    harness.press(KeyCode::Esc);

    assert!(harness.app.overlay.is_none());
    assert_eq!(harness.app.views.current().name, "open");
    assert_eq!(harness.lines().len(), before);
}

#[test]
fn r_re_reads_both_the_list_and_the_status() {
    let mut harness = loaded();
    let before = harness.lines().len();
    harness.press(KeyCode::Char('R'));

    let asked = &harness.lines()[before..];
    assert!(asked.contains(&"ls !done --json".to_string()), "{asked:?}");
    assert!(asked.contains(&"status --json".to_string()), "{asked:?}");
}

#[test]
fn r_before_dam_ever_answered_starts_the_handshake_again() {
    let mut harness = harness();
    harness.press(KeyCode::Char('r'));

    assert_eq!(harness.lines(), vec!["--version".to_string()]);
}

#[test]
fn keys_do_nothing_while_the_refusal_is_drawn() {
    let mut harness = harness();
    crate::open::start(&mut harness.app);
    harness.answer(0, 0, "dam 0.0.9\n", "");
    harness.press(KeyCode::Char('R'));
    harness.press(KeyCode::Char('2'));

    assert_eq!(harness.lines(), vec!["--version".to_string()]);
}

#[test]
fn a_configured_default_view_is_the_one_the_pane_opens_on() {
    let config = Config::parse(
        "default_view = \"deep\"\n[[views]]\nname = \"deep\"\nquery = \"effort:deep\"\n",
    )
    .expect("parses");
    let mut harness = loaded_with(config);

    assert_eq!(harness.app.views.current().name, "deep");
    assert!(
        harness
            .lines()
            .contains(&"ls effort:deep --json".to_string())
    );
    harness.press(KeyCode::Char('1'));
    assert_eq!(harness.last(), "ls !done --json");
}

#[test]
fn a_view_request_file_outranks_the_showing_view_and_is_read_once() {
    let mut harness = loaded_with(three_views());
    let path = std::env::temp_dir().join(format!("herdr-damnit-request-{}", std::process::id()));
    std::fs::write(&path, "deep").expect("the request is written");
    harness.app.view_request = Some(path.clone());

    harness.app.tick(Instant::now());
    assert_eq!(harness.app.views.current().name, "deep");
    assert_eq!(harness.last(), "ls !done & effort:deep --json");

    harness.press(KeyCode::Char('1'));
    harness.app.tick(Instant::now());
    assert_eq!(
        harness.app.views.current().name,
        "open",
        "the request outlived its read"
    );
    assert!(!path.exists());
}

#[test]
fn the_interval_re_reads_once_a_full_interval_has_passed() {
    let mut harness = loaded();
    let now = Instant::now();
    harness.app.tick(now);
    let before = harness.lines().len();

    harness.app.tick(now + Duration::from_secs(299));
    assert_eq!(harness.lines().len(), before, "it re-read early");
    harness.app.tick(now + Duration::from_secs(300));
    assert_eq!(harness.lines().len(), before + 2);
}

#[test]
fn the_interval_is_held_back_while_an_overlay_is_open_or_another_screen_shows() {
    let mut harness = loaded_with(three_views());
    let now = Instant::now();
    harness.app.tick(now);
    harness.press(KeyCode::Char('v'));
    let before = harness.lines().len();

    harness.app.tick(now + Duration::from_secs(600));
    harness.press(KeyCode::Esc);
    harness.press(KeyCode::Tab);
    harness.app.tick(now + Duration::from_secs(900));

    assert_eq!(
        harness.lines().len(),
        before,
        "it re-read under an open picker"
    );
}

#[test]
fn a_refresh_interval_of_zero_never_re_reads_on_its_own() {
    let config = Config::parse("refresh_seconds = 0\n").expect("parses");
    let mut harness = loaded_with(config);
    let now = Instant::now();
    harness.app.tick(now);
    let before = harness.lines().len();

    harness.app.tick(now + Duration::from_secs(3600));

    assert_eq!(
        harness.lines().len(),
        before,
        "it re-read with the interval off"
    );
}

#[test]
fn a_status_request_shows_the_status_screen_rather_than_a_view() {
    let mut harness = loaded_with(three_views());
    let path = std::env::temp_dir().join(format!("herdr-damnit-status-{}", std::process::id()));
    std::fs::write(&path, crate::pane::STATUS_REQUEST).expect("the request is written");
    harness.app.view_request = Some(path.clone());

    harness.app.tick(Instant::now());

    assert_eq!(harness.app.screen, Screen::Status);
    assert_eq!(harness.app.views.current().name, "open");
    assert!(!path.exists());
}
