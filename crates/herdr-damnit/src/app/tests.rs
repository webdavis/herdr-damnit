use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent};
use herdr_damnit_application::{
    Clock, DamRunner, Finished, JobKind, Jobs, RunningJob, SpawnError, SyncKind,
};
use herdr_damnit_domain::{Date, parse_date};

use super::*;

/// The day every harness reads dates against, so an overdue row is overdue whenever the test runs.
const TODAY: &str = "2026-09-20";

#[derive(Default)]
pub(crate) struct Recorder {
    pub(crate) log: Arc<Mutex<Vec<Vec<String>>>>,
    pub(crate) senders: Arc<Mutex<Vec<Sender<Finished>>>>,
}

impl DamRunner for Recorder {
    fn spawn(&self, argv: &[String]) -> Result<RunningJob, SpawnError> {
        self.log.lock().expect("the log").push(argv.to_vec());
        let (sender, results) = channel();
        self.senders.lock().expect("the senders").push(sender);
        Ok(RunningJob {
            cancel: Box::new(|| {}),
            results,
        })
    }
}

/// A clock stopped at one instant, so a header timer is read at an offset the test names rather
/// than at whatever the machine was doing.
struct StoppedClock {
    today: Date,
    now: Instant,
}

impl Clock for StoppedClock {
    fn today(&self) -> Date {
        self.today
    }

    fn now(&self) -> Instant {
        self.now
    }
}

pub(crate) struct Harness {
    pub(crate) app: App,
    started: Instant,
    log: Arc<Mutex<Vec<Vec<String>>>>,
    senders: Arc<Mutex<Vec<Sender<Finished>>>>,
}

pub(crate) fn harness() -> Harness {
    harness_with(Config::parse("").expect("the default config"))
}

pub(crate) fn harness_with(config: Config) -> Harness {
    let recorder = Recorder::default();
    let log = Arc::clone(&recorder.log);
    let senders = Arc::clone(&recorder.senders);
    let today = parse_date(TODAY).expect("a date");
    let started = Instant::now();
    Harness {
        app: App::new(
            config,
            Jobs::new(
                Box::new(recorder),
                Box::new(StoppedClock {
                    today,
                    now: started,
                }),
            ),
            today,
        ),
        started,
        log,
        senders,
    }
}

impl Harness {
    /// The instant every job in this harness started at, which is what a header offset is measured
    /// from.
    pub(crate) fn started(&self) -> Instant {
        self.started
    }

    pub(crate) fn press(&mut self, code: KeyCode) -> After {
        self.app.key(KeyEvent::from(code))
    }

    /// Answer one spawned job. A send to a superseded job fails because dropping it from the table
    /// dropped its receiver, which is the mechanism by which its result never reaches the model, so
    /// the answer is offered rather than required.
    pub(crate) fn answer(&mut self, which: usize, code: i32, stdout: &str, stderr: &str) {
        let _ = self.senders.lock().expect("the senders")[which].send(Finished {
            code: Some(code),
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
            elapsed: Duration::from_millis(9),
        });
        self.app.tick(Instant::now());
    }

    pub(crate) fn lines(&self) -> Vec<String> {
        self.log
            .lock()
            .expect("the log")
            .iter()
            .map(|argv| argv.join(" "))
            .collect()
    }

    pub(crate) fn last(&self) -> String {
        self.lines().last().cloned().unwrap_or_default()
    }
}

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
fn the_header_names_the_exclusive_job_its_spinner_and_its_elapsed_time() {
    let mut harness = harness();
    let started = harness.started();
    harness.app.submit(
        JobKind::Exclusive(SyncKind::Push),
        herdr_damnit_application::argv::push(),
    );
    harness.app.tick(started);

    let header = harness.app.header(started + Duration::from_millis(3200));
    assert!(header.contains("push"), "{header}");
    assert!(header.contains("3.2s"), "{header}");
}

#[test]
fn elapsed_is_tenths_under_ten_seconds_and_whole_seconds_after_it() {
    let mut harness = harness();
    let started = harness.started();
    harness.app.submit(
        JobKind::Exclusive(SyncKind::Pull),
        herdr_damnit_application::argv::pull(),
    );

    assert!(
        harness
            .app
            .header(started + Duration::from_millis(1500))
            .contains("1.5s")
    );
    assert!(
        harness
            .app
            .header(started + Duration::from_secs(42))
            .contains("42s")
    );
}

#[test]
fn two_reads_in_flight_are_counted_rather_than_named() {
    let mut harness = harness();
    harness.app.submit(
        JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );
    harness
        .app
        .submit(JobKind::ReadLog, herdr_damnit_application::argv::log());

    let header = harness.app.header(Instant::now());
    assert!(header.contains("2 reads"), "{header}");
}

#[test]
fn the_spinner_steps_one_frame_per_tick() {
    let mut harness = harness();
    harness.app.submit(
        JobKind::ReadStatus,
        herdr_damnit_application::argv::status(),
    );

    let first = harness.app.spinner;
    harness.app.tick(Instant::now());
    assert_ne!(harness.app.spinner, first);
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
