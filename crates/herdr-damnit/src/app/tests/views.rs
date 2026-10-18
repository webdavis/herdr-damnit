use std::time::Instant;

use crate::app::Overlay;
use crate::screens::tests::loaded_with;

use super::*;

pub(crate) fn three_views() -> Config {
    Config::parse(
        "icons = \"ascii\"\n\
         [[views]]\nname = \"today\"\nquery = \"!done & due:today\"\n\
         [[views]]\nname = \"deep\"\nquery = \"!done & effort:deep\"\n",
    )
    .expect("parses")
}

fn picker_names(app: &App) -> (Vec<String>, usize) {
    let Some(Overlay::View(picker)) = app.showing().overlay() else {
        panic!("expected the view picker");
    };
    let names = picker.entries.iter().map(|entry| entry.text.clone());
    (names.collect(), picker.selected)
}

#[test]
fn a_number_key_shows_that_view_and_reads_its_query() {
    let mut harness = loaded_with(three_views());
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Char('2'));

    assert_eq!(harness.app.list().views().current().name, "today");
    assert_eq!(harness.last(), "ls !done & due:today --json");
    assert_eq!(harness.app.showing().screen(), Screen::List);
}

#[test]
fn a_number_with_no_view_behind_it_says_how_many_there_are_and_reads_nothing() {
    let mut harness = loaded_with(three_views());
    let before = harness.lines().len();
    harness.press(KeyCode::Char('7'));

    assert_eq!(harness.app.message(), "the config has 3 views.");
    assert_eq!(harness.lines().len(), before);
}

#[test]
fn the_number_of_the_showing_view_reads_nothing_and_says_nothing() {
    let mut harness = loaded_with(three_views());
    let before = harness.lines().len();
    harness.press(KeyCode::Char('1'));

    assert!(harness.app.message().is_empty());
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

    assert!(harness.app.showing().overlay().is_none());
}

#[test]
fn the_view_picker_takes_the_entry_under_the_cursor_and_closes() {
    let mut harness = loaded_with(three_views());
    harness.press(KeyCode::Char('v'));
    harness.press(KeyCode::Char('j'));
    harness.press(KeyCode::Enter);

    assert!(harness.app.showing().overlay().is_none());
    assert_eq!(harness.app.list().views().current().name, "today");
    assert_eq!(harness.last(), "ls !done & due:today --json");
}

#[test]
fn escape_closes_the_picker_and_changes_no_view() {
    let mut harness = loaded_with(three_views());
    harness.press(KeyCode::Char('v'));
    harness.press(KeyCode::Char('j'));
    let before = harness.lines().len();
    harness.press(KeyCode::Esc);

    assert!(harness.app.showing().overlay().is_none());
    assert_eq!(harness.app.list().views().current().name, "open");
    assert_eq!(harness.lines().len(), before);
}

#[test]
fn a_configured_default_view_is_the_one_the_pane_opens_on() {
    let config = Config::parse(
        "default_view = \"deep\"\n[[views]]\nname = \"deep\"\nquery = \"effort:deep\"\n",
    )
    .expect("parses");
    let mut harness = loaded_with(config);

    assert_eq!(harness.app.list().views().current().name, "deep");
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
    harness.app.watch_view_requests(path.clone());

    harness.app.tick(Instant::now());
    assert_eq!(harness.app.list().views().current().name, "deep");
    assert_eq!(harness.last(), "ls !done & effort:deep --json");

    harness.press(KeyCode::Char('1'));
    harness.app.tick(Instant::now());
    assert_eq!(
        harness.app.list().views().current().name,
        "open",
        "the request outlived its read"
    );
    assert!(!path.exists());
}

#[test]
fn a_status_request_shows_the_status_screen_rather_than_a_view() {
    let mut harness = loaded_with(three_views());
    let path = std::env::temp_dir().join(format!("herdr-damnit-status-{}", std::process::id()));
    std::fs::write(&path, crate::pane::STATUS_REQUEST).expect("the request is written");
    harness.app.watch_view_requests(path.clone());

    harness.app.tick(Instant::now());

    assert_eq!(harness.app.showing().screen(), Screen::Status);
    assert_eq!(harness.app.list().views().current().name, "open");
    assert!(!path.exists());
}
