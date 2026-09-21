mod unanswered;

use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::*;
use crate::{Finished, SpawnError};

/// A runner that records every argv and hands the test the sender for each job, so a test decides
/// when a job answers and with what.
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
        elapsed: Duration::from_millis(9),
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
        jobs: Jobs::new(Box::new(recorder)),
        log,
        senders,
    }
}

impl Harness {
    fn submit(&mut self, kind: JobKind, argv: Vec<String>) -> Submitted {
        self.jobs.submit(kind, argv)
    }

    /// Answer one spawned job. A send to a superseded job fails because dropping it from the table
    /// dropped its receiver, which is the mechanism by which its result never reaches the model, so
    /// the answer is offered rather than required.
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
        harness.submit(JobKind::Exclusive(SyncKind::Push), crate::argv::push()),
        Submitted::Started(_)
    ));

    let Submitted::Refused(message) =
        harness.submit(JobKind::Exclusive(SyncKind::Push), crate::argv::push())
    else {
        panic!("expected a refusal");
    };

    assert_eq!(message, "a push is already running; <C-c> cancels it.");
    assert_eq!(harness.lines(), vec!["push --json".to_string()]);
}

#[test]
fn a_pull_is_refused_while_a_push_runs_and_names_the_one_that_is_running() {
    let mut harness = harness();
    harness.submit(JobKind::Exclusive(SyncKind::Push), crate::argv::push());

    let Submitted::Refused(message) =
        harness.submit(JobKind::Exclusive(SyncKind::Pull), crate::argv::pull())
    else {
        panic!("expected a refusal");
    };
    assert_eq!(message, "a push is already running; <C-c> cancels it.");
}

#[test]
fn a_write_runs_alongside_a_push_because_dam_serialises_them_at_the_store() {
    let mut harness = harness();
    harness.submit(JobKind::Exclusive(SyncKind::Push), crate::argv::push());
    assert!(matches!(
        harness.submit(JobKind::Write, crate::argv::stage_all()),
        Submitted::Started(_)
    ));
    assert_eq!(harness.jobs.in_flight(), 2);
}

#[test]
fn a_newer_read_of_a_kind_supersedes_the_older_one_and_its_result_is_dropped() {
    let mut harness = harness();
    harness.submit(JobKind::ReadList, crate::argv::list("!done"));
    harness.submit(JobKind::ReadList, crate::argv::list("done"));

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
    harness.submit(JobKind::Write, crate::argv::stage_all());
    harness.submit(JobKind::Write, crate::argv::unstage_all());

    assert_eq!(harness.jobs.in_flight(), 2);
}

/// Every job carries an id of its own, because `drain` clears the table by id: two jobs sharing
/// one would both leave it the moment either answered.
#[test]
fn one_job_answering_never_clears_another_that_is_still_running() {
    let mut harness = harness();
    harness.submit(JobKind::Write, crate::argv::stage_all());
    harness.submit(JobKind::Write, crate::argv::unstage_all());
    harness.answer(0, ok("the first"));

    let completions = harness.jobs.drain();
    assert_eq!(completions.len(), 1);
    assert_eq!(completions[0].finished.stdout, "the first");
    assert_eq!(harness.jobs.in_flight(), 1);
}

#[test]
fn a_write_asks_for_a_fresh_status_and_a_fresh_list_rather_than_patching_the_model() {
    let mut harness = harness();
    harness.submit(JobKind::Write, crate::argv::stage_all());
    harness.answer(0, ok("{}"));

    let completions = harness.jobs.drain();
    assert_eq!(
        completions[0].follow_up,
        vec![JobKind::ReadStatus, JobKind::ReadList]
    );
}

#[test]
fn a_finished_push_asks_for_the_same_two_reads_a_write_does() {
    let mut harness = harness();
    harness.submit(JobKind::Exclusive(SyncKind::Push), crate::argv::push());
    harness.answer(0, ok("{}"));

    assert_eq!(
        harness.jobs.drain()[0].follow_up,
        vec![JobKind::ReadStatus, JobKind::ReadList]
    );
}

#[test]
fn a_read_asks_for_nothing_after_itself() {
    let mut harness = harness();
    harness.submit(JobKind::ReadStatus, crate::argv::status());
    harness.answer(0, ok("{}"));

    assert_eq!(harness.jobs.drain()[0].follow_up, Vec::new());
}

#[test]
fn draining_does_not_block_on_a_job_that_has_not_answered() {
    let mut harness = harness();
    harness.submit(JobKind::ReadStatus, crate::argv::status());

    assert!(harness.jobs.drain().is_empty());
    assert_eq!(harness.jobs.in_flight(), 1);
}

#[test]
fn a_finished_job_leaves_the_table() {
    let mut harness = harness();
    harness.submit(JobKind::ReadStatus, crate::argv::status());
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

/// A `dam` that is on `PATH` and would not start is a different sentence from one that is absent.
#[test]
fn a_spawn_that_failed_for_some_other_reason_carries_dams_own_error() {
    let mut jobs = Jobs::new(Box::new(Recorder {
        io_error: Some("permission denied".to_string()),
        ..Recorder::default()
    }));

    let Submitted::Failed(message) = jobs.submit(JobKind::ReadStatus, crate::argv::status()) else {
        panic!("expected a failure");
    };
    assert_eq!(message, "permission denied");
    assert_eq!(jobs.in_flight(), 0);
}

/// The header's timer, read at a moment the caller chooses rather than off an ambient clock.
#[test]
fn the_header_times_the_job_it_names_and_nothing_when_none_runs() {
    let mut harness = harness();
    assert_eq!(harness.jobs.elapsed_of_current(Instant::now()), None);

    harness.submit(JobKind::ReadStatus, crate::argv::status());
    let later = Instant::now() + Duration::from_secs(5);
    assert!(harness.jobs.elapsed_of_current(later).expect("a job runs") >= Duration::from_secs(5));
}

#[test]
fn a_dam_that_is_not_there_is_reported_rather_than_started() {
    let recorder = Recorder {
        refuse_spawn: true,
        ..Recorder::default()
    };
    let mut jobs = Jobs::new(Box::new(recorder));

    assert!(matches!(
        jobs.submit(JobKind::ReadStatus, crate::argv::status()),
        Submitted::NotInstalled
    ));
    assert_eq!(jobs.in_flight(), 0);
}

#[test]
fn a_push_names_itself_while_it_runs_and_nothing_once_it_is_drained() {
    let mut harness = harness();
    harness.submit(JobKind::Exclusive(SyncKind::Pull), crate::argv::pull());
    assert_eq!(harness.jobs.exclusive(), Some(SyncKind::Pull));

    harness.answer(0, ok("{}"));
    harness.jobs.drain();
    assert_eq!(harness.jobs.exclusive(), None);
}
