mod agent;
mod discard;
mod editor;
mod edits;
mod harness;
mod header;
mod keys;
mod non_blocking;
mod pickers;
mod quit;
mod staging;
mod sync;

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent};
use herdr_damnit_application::{JobKind, Jobs, SyncKind};
use herdr_damnit_domain::parse_date;

pub(crate) use harness::*;

use super::*;

#[test]
fn a_key_that_needs_dam_returns_before_the_job_answers() {
    let mut harness = harness();
    let started = Instant::now();
    harness.app.submit(
        JobKind::Exclusive(SyncKind::Push),
        herdr_damnit_application::argv::push(),
    );

    assert!(
        started.elapsed() < Duration::from_millis(50),
        "the key blocked"
    );
    assert_eq!(harness.lines(), vec!["push --json".to_string()]);
}

#[test]
fn the_poll_window_narrows_while_a_job_is_in_flight() {
    let mut harness = harness();
    assert_eq!(harness.app.poll_window(), Duration::from_millis(200));

    harness.app.submit(
        JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    assert_eq!(harness.app.poll_window(), Duration::from_millis(50));

    harness.answer(
        0,
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
fn a_write_completion_re_reads_the_status_and_the_list_rather_than_patching_the_model() {
    let mut harness = harness();
    harness
        .app
        .submit(JobKind::Write, herdr_damnit_application::argv::stage_all());
    harness.answer(0, 0, "{}", "");

    let lines = harness.lines();
    assert_eq!(lines[0], "add -A --json");
    assert!(lines.contains(&"status --json".to_string()), "{lines:?}");
    assert!(lines.contains(&"ls !done --json".to_string()), "{lines:?}");
}

#[test]
fn a_refusal_puts_dams_own_sentence_in_the_status_line_and_leaves_the_model_alone() {
    let mut harness = harness();
    let before = harness.app.list.object_count();
    harness
        .app
        .submit(JobKind::Write, herdr_damnit_application::argv::stage_all());
    harness.answer(0, 2, "", "dam: no object matches \"zzzzzzz\"\n");

    assert_eq!(harness.app.message, "no object matches \"zzzzzzz\"");
    assert_eq!(harness.app.list.object_count(), before);
}

#[test]
fn a_held_store_adds_the_retry_sentence() {
    let mut harness = harness();
    harness
        .app
        .submit(JobKind::Write, herdr_damnit_application::argv::stage_all());
    harness.answer(0, 1, "", "dam: database is locked");

    assert_eq!(
        harness.app.message,
        "database is locked; another dam is writing, press R to retry."
    );
}

#[test]
fn output_that_will_not_parse_is_reported_without_quoting_it() {
    let mut harness = harness();
    harness.app.submit(
        JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness.answer(0, 0, "this is not json", "");

    assert_eq!(
        harness.app.message,
        "dam answered with something this pane could not read."
    );
}
