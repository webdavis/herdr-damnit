mod unanswered;

use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::*;
use crate::{Clock, Finished, SpawnError};

struct TestClock;

impl Clock for TestClock {
    fn today(&self) -> herdr_damnit_domain::Date {
        herdr_damnit_domain::parse_date("2026-09-20").expect("a date")
    }

    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[derive(Default)]
struct Recorder {
    log: Arc<Mutex<Vec<Vec<String>>>>,
    senders: Arc<Mutex<Vec<Sender<Finished>>>>,
    refuse_spawn: bool,
    io_error: Option<String>,
}

impl DamRunner for Recorder {
    fn spawn(&self, argv: &[String]) -> Result<RunningJob, SpawnError> {
        if self.refuse_spawn {
            return Err(SpawnError::NotFound);
        }
        if let Some(error) = &self.io_error {
            return Err(SpawnError::Io(error.clone()));
        }
        self.log.lock().expect("the log").push(argv.to_vec());
        let (sender, results) = channel();
        self.senders.lock().expect("the senders").push(sender);
        Ok(RunningJob {
            cancel: Box::new(|| {}),
            results,
        })
    }
}

fn ok(stdout: &str) -> Finished {
    Finished {
        code: Some(0),
        stdout: stdout.to_string(),
        stderr: String::new(),
    }
}

struct Harness {
    jobs: Jobs,
    log: Arc<Mutex<Vec<Vec<String>>>>,
    senders: Arc<Mutex<Vec<Sender<Finished>>>>,
}

fn harness() -> Harness {
    let recorder = Recorder::default();
    let log = Arc::clone(&recorder.log);
    let senders = Arc::clone(&recorder.senders);
    Harness {
        jobs: Jobs::new(Box::new(recorder), Box::new(TestClock)),
        log,
        senders,
    }
}

impl Harness {
    fn submit(&mut self, request: Request) -> Submitted {
        self.jobs.submit(request)
    }

    fn answer(&self, which: usize, finished: Finished) {
        let _ = self.senders.lock().expect("the senders")[which].send(finished);
    }

    fn lines(&self) -> Vec<String> {
        self.log
            .lock()
            .expect("the log")
            .iter()
            .map(|argv| argv.join(" "))
            .collect()
    }
}

#[test]
fn a_second_push_is_refused_with_a_sentence_and_never_spawned() {
    let mut harness = harness();
    assert!(matches!(
        harness.submit(Request::Sync(SyncKind::Push, crate::argv::push())),
        Submitted::Started(_)
    ));

    let Submitted::Refused(message) =
        harness.submit(Request::Sync(SyncKind::Push, crate::argv::push()))
    else {
        panic!("expected a refusal");
    };

    assert_eq!(message, "a push is already running; <C-c> cancels it.");
    assert_eq!(harness.lines(), vec!["push --json".to_string()]);
}

#[test]
fn a_pull_is_refused_while_a_push_runs_and_names_the_one_that_is_running() {
    let mut harness = harness();
    harness.submit(Request::Sync(SyncKind::Push, crate::argv::push()));

    let Submitted::Refused(message) =
        harness.submit(Request::Sync(SyncKind::Pull, crate::argv::pull()))
    else {
        panic!("expected a refusal");
    };
    assert_eq!(message, "a push is already running; <C-c> cancels it.");
}

#[test]
fn a_write_runs_alongside_a_push_because_dam_serialises_them_at_the_store() {
    let mut harness = harness();
    harness.submit(Request::Sync(SyncKind::Push, crate::argv::push()));
    assert!(matches!(
        harness.submit(Request::Write(crate::argv::stage_all())),
        Submitted::Started(_)
    ));
    assert_eq!(harness.jobs.in_flight(), 2);
}

#[test]
fn a_newer_read_of_a_kind_supersedes_the_older_one_and_its_result_is_dropped() {
    let mut harness = harness();
    harness.submit(Request::List("!done".to_string()));
    harness.submit(Request::List("done".to_string()));

    harness.answer(0, ok(r#"{"objects":[]}"#));
    harness.answer(1, ok(r#"{"objects":[{"oid":"1"}]}"#));

    let completions = harness.jobs.drain();
    assert_eq!(completions.len(), 1, "a superseded read reached the model");
    assert_eq!(
        completions[0].finished.stdout,
        r#"{"objects":[{"oid":"1"}]}"#
    );
}

#[test]
fn two_writes_are_two_intentions_and_neither_supersedes_the_other() {
    let mut harness = harness();
    harness.submit(Request::Write(crate::argv::stage_all()));
    harness.submit(Request::Write(crate::argv::unstage_all()));

    assert_eq!(harness.jobs.in_flight(), 2);
}

#[test]
fn one_job_answering_never_clears_another_that_is_still_running() {
    let mut harness = harness();
    harness.submit(Request::Write(crate::argv::stage_all()));
    harness.submit(Request::Write(crate::argv::unstage_all()));
    harness.answer(0, ok("the first"));

    let completions = harness.jobs.drain();
    assert_eq!(completions.len(), 1);
    assert_eq!(completions[0].finished.stdout, "the first");
    assert_eq!(harness.jobs.in_flight(), 1);
}

#[test]
fn draining_does_not_block_on_a_job_that_has_not_answered() {
    let mut harness = harness();
    harness.submit(Request::Status);

    assert!(harness.jobs.drain().is_empty());
    assert_eq!(harness.jobs.in_flight(), 1);
}

#[test]
fn a_finished_job_leaves_the_table() {
    let mut harness = harness();
    harness.submit(Request::Status);
    harness.answer(0, ok("{}"));
    harness.jobs.drain();

    assert_eq!(harness.jobs.in_flight(), 0);
    assert_eq!(harness.jobs.exclusive(), None);
}

#[test]
fn each_sync_job_names_itself_in_the_refusal() {
    assert_eq!(SyncKind::Commit.verb(), "commit");
    assert_eq!(SyncKind::Push.verb(), "push");
    assert_eq!(SyncKind::Pull.verb(), "pull");
}

#[test]
fn a_spawn_that_failed_for_some_other_reason_carries_dams_own_error() {
    let mut jobs = Jobs::new(
        Box::new(Recorder {
            io_error: Some("permission denied".to_string()),
            ..Recorder::default()
        }),
        Box::new(TestClock),
    );

    let Submitted::Failed(message) = jobs.submit(Request::Status) else {
        panic!("expected a failure");
    };
    assert_eq!(message, "permission denied");
    assert_eq!(jobs.in_flight(), 0);
}

#[test]
fn the_header_times_the_job_it_names_and_nothing_when_none_runs() {
    let mut harness = harness();
    assert_eq!(harness.jobs.elapsed_of_current(Instant::now()), None);

    harness.submit(Request::Status);
    let later = Instant::now() + Duration::from_secs(5);
    assert!(harness.jobs.elapsed_of_current(later).expect("a job runs") >= Duration::from_secs(5));
}

#[test]
fn a_dam_that_is_not_there_is_reported_rather_than_started() {
    let recorder = Recorder {
        refuse_spawn: true,
        ..Recorder::default()
    };
    let mut jobs = Jobs::new(Box::new(recorder), Box::new(TestClock));

    assert!(matches!(
        jobs.submit(Request::Status),
        Submitted::NotInstalled
    ));
    assert_eq!(jobs.in_flight(), 0);
}

#[test]
fn a_push_names_itself_while_it_runs_and_nothing_once_it_is_drained() {
    let mut harness = harness();
    harness.submit(Request::Sync(SyncKind::Pull, crate::argv::pull()));
    assert_eq!(harness.jobs.exclusive(), Some(SyncKind::Pull));

    harness.answer(0, ok("{}"));
    harness.jobs.drain();
    assert_eq!(harness.jobs.exclusive(), None);
}

#[test]
fn a_completion_carries_the_id_its_submit_answered() {
    let mut harness = harness();
    let Submitted::Started(first) = harness.submit(Request::Write(crate::argv::stage_all())) else {
        panic!("expected a start");
    };
    let Submitted::Started(second) = harness.submit(Request::Write(crate::argv::unstage_all()))
    else {
        panic!("expected a start");
    };
    harness.answer(1, ok("{}"));
    harness.answer(0, ok("{}"));

    let ids: Vec<JobId> = harness.jobs.drain().iter().map(|done| done.id).collect();
    assert_eq!(ids, vec![first, second]);
}
