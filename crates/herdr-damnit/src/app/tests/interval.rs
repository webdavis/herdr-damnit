use std::time::{Duration, Instant};

use crate::screens::tests::{loaded, loaded_with};

use super::views::three_views;
use super::*;

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
