use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};

use super::*;
use crate::{Finished, SpawnError};

/// A runner that records every argv and hands the test the sender for each job, so a test decides
/// when a job answers and with what.
#[derive(Default)]
struct Recorder {
    log: Arc<Mutex<Vec<Vec<String>>>>,
    senders: Arc<Mutex<Vec<Sender<Finished>>>>,
    refuse_spawn: bool,
}

impl DamRunner for Recorder {
    fn spawn(&self, argv: &[String]) -> Result<RunningJob, SpawnError> {
        if self.refuse_spawn {
            return Err(SpawnError::NotFound);
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
        elapsed: std::time::Duration::from_millis(9),
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

#[test]
fn cancelling_reaches_the_exclusive_job_before_any_read_that_started_earlier() {
    let cancelled = Arc::new(Mutex::new(Vec::new()));
    let mut jobs = Jobs::new(Box::new(Marking {
        cancelled: Arc::clone(&cancelled),
        next: Mutex::new(0),
    }));
    jobs.submit(JobKind::ReadList, crate::argv::list("!done"));
    jobs.submit(JobKind::Exclusive(SyncKind::Push), crate::argv::push());

    assert!(jobs.cancel_current());
    assert_eq!(*cancelled.lock().expect("the record"), vec![1]);
}

/// A `dam` thread that dies without sending closes the channel. The job leaves the table on the
/// next drain, so a wedged exclusive job cannot block every later one.
#[test]
fn a_job_whose_thread_died_without_answering_leaves_the_table() {
    let mut jobs = Jobs::new(Box::new(Marking {
        cancelled: Arc::new(Mutex::new(Vec::new())),
        next: Mutex::new(0),
    }));
    jobs.submit(JobKind::Exclusive(SyncKind::Push), crate::argv::push());

    assert!(jobs.drain().is_empty());
    assert_eq!(jobs.in_flight(), 0);
    assert_eq!(jobs.exclusive(), None);
}

#[test]
fn cancelling_with_nothing_in_flight_says_so() {
    let mut harness = harness();
    assert!(!harness.jobs.cancel_current());
}

/// A runner whose jobs record their own index when cancelled.
struct Marking {
    cancelled: Arc<Mutex<Vec<usize>>>,
    next: Mutex<usize>,
}

impl DamRunner for Marking {
    fn spawn(&self, _argv: &[String]) -> Result<RunningJob, SpawnError> {
        let mut next = self.next.lock().expect("the counter");
        let which = *next;
        *next += 1;
        let cancelled = Arc::clone(&self.cancelled);
        let (_sender, results) = channel();
        Ok(RunningJob {
            cancel: Box::new(move || cancelled.lock().expect("the record").push(which)),
            results,
        })
    }
}
