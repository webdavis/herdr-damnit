mod agent;
mod detail;
mod discard;
mod editor;
mod edits;
mod fakes;
mod harness;
mod header;
mod interval;
mod keys;
mod non_blocking;
mod pending;
mod pickers;
mod quit;
mod staging;
mod sync;
pub(crate) mod views;

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent};
use herdr_damnit_application::{Jobs, Request, SyncKind};

use crate::screens::tests::loaded;

pub(crate) use fakes::*;
pub(crate) use harness::*;

use super::*;

#[test]
fn a_key_that_needs_dam_returns_before_the_job_answers() {
    let mut harness = harness();
    let started = Instant::now();
    harness.app.submit(Request::Sync(
        SyncKind::Push,
        herdr_damnit_application::argv::push(),
    ));

    assert!(
        started.elapsed() < Duration::from_millis(50),
        "the key blocked"
    );
    assert_eq!(harness.lines(), vec!["push --json".to_string()]);
}

#[test]
fn the_poll_window_narrows_while_a_job_is_in_flight() {
    let mut harness = loaded();
    assert_eq!(harness.app.poll_window(), Duration::from_millis(200));

    harness.app.submit(Request::Status);
    assert_eq!(harness.app.poll_window(), Duration::from_millis(50));

    harness.answer(
        3,
        0,
        r#"{"staged":[],"unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#,
        "",
    );
    assert_eq!(harness.app.poll_window(), Duration::from_millis(200));
}

#[test]
fn a_tick_with_nothing_in_flight_does_not_block() {
    let mut harness = harness();
    let started = Instant::now();
    for _ in 0..100 {
        harness.app.tick(Instant::now());
    }
    assert!(
        started.elapsed() < Duration::from_millis(100),
        "a tick blocked"
    );
}

#[test]
fn a_write_completion_re_reads_the_status_and_the_list_once_each_rather_than_patching_the_model() {
    let mut harness = loaded();
    harness.press(KeyCode::Char('A'));
    let before = harness.lines().len();
    harness.answer(3, 0, "{}", "");

    let mut asked = harness.lines()[before..].to_vec();
    asked.sort();
    assert_eq!(asked, ["ls !done --json", "status --json"]);
}

#[test]
fn a_write_completion_once_done_was_opened_also_re_reads_the_done_list_and_the_log() {
    let mut harness = loaded();
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Tab);
    harness.press(KeyCode::Char('A'));
    let before = harness.lines().len();
    harness.answer(5, 0, "{}", "");

    let mut asked = harness.lines()[before..].to_vec();
    asked.sort();
    assert_eq!(
        asked,
        [
            "log --json",
            "ls !done --json",
            "ls done --json",
            "status --json"
        ]
    );
}

#[test]
fn a_refusal_puts_dams_own_sentence_in_the_status_line_and_leaves_the_model_alone() {
    let mut harness = harness();
    let before = harness.app.list().cursor().object_count();
    harness
        .app
        .submit(Request::Write(herdr_damnit_application::argv::stage_all()));
    harness.answer(0, 2, "", "dam: no object matches \"zzzzzzz\"\n");

    assert_eq!(harness.app.message(), "no object matches \"zzzzzzz\"");
    assert_eq!(harness.app.list().cursor().object_count(), before);
}

#[test]
fn a_held_store_adds_the_retry_sentence() {
    let mut harness = harness();
    harness
        .app
        .submit(Request::Write(herdr_damnit_application::argv::stage_all()));
    harness.answer(0, 1, "", "dam: database is locked");

    assert_eq!(
        harness.app.message(),
        "database is locked; another dam is writing, press R to retry."
    );
}

#[test]
fn output_that_will_not_parse_is_reported_without_quoting_it() {
    let mut harness = loaded();
    harness.app.submit(Request::Status);
    harness.answer(3, 0, "this is not json", "");

    assert_eq!(
        harness.app.message(),
        "dam answered with something this pane could not read."
    );
}
