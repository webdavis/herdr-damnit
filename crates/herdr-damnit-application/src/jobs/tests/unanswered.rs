use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};

use crate::argv;
use crate::jobs::{JobKind, Jobs, SyncKind};
use crate::{DamRunner, RunningJob, SpawnError};

use super::TestClock;

#[test]
fn cancelling_reaches_the_exclusive_job_before_any_read_that_started_earlier() {
    let runner = RecordsCancelledIndex::new();
    let cancelled = Arc::clone(&runner.cancelled);
    let mut jobs = Jobs::new(Box::new(runner), Box::new(TestClock));
    jobs.submit(JobKind::ReadList, argv::list("!done"));
    jobs.submit(JobKind::Exclusive(SyncKind::Push), argv::push());

    assert!(jobs.cancel_current());
    assert_eq!(*cancelled.lock().expect("the record"), vec![1]);
}

#[test]
fn a_job_whose_thread_died_without_answering_leaves_the_table_rather_than_blocking_later_ones() {
    let mut jobs = Jobs::new(Box::new(RecordsCancelledIndex::new()), Box::new(TestClock));
    jobs.submit(JobKind::Exclusive(SyncKind::Push), argv::push());

    assert!(jobs.drain().is_empty());
    assert_eq!(jobs.in_flight(), 0);
    assert_eq!(jobs.exclusive(), None);
}

#[test]
fn with_no_exclusive_job_the_cancel_reaches_the_oldest_read() {
    let runner = RecordsCancelledIndex::new();
    let cancelled = Arc::clone(&runner.cancelled);
    let mut jobs = Jobs::new(Box::new(runner), Box::new(TestClock));
    jobs.submit(JobKind::ReadList, argv::list("!done"));
    jobs.submit(JobKind::ReadStatus, argv::status());

    assert!(jobs.cancel_current());
    assert_eq!(*cancelled.lock().expect("the record"), vec![0]);
}

#[test]
fn cancelling_with_nothing_in_flight_says_so() {
    let mut jobs = Jobs::new(Box::new(RecordsCancelledIndex::new()), Box::new(TestClock));
    assert!(!jobs.cancel_current());
}

struct RecordsCancelledIndex {
    cancelled: Arc<Mutex<Vec<usize>>>,
    next: Mutex<usize>,
}

impl RecordsCancelledIndex {
    fn new() -> Self {
        Self {
            cancelled: Arc::new(Mutex::new(Vec::new())),
            next: Mutex::new(0),
        }
    }
}

impl DamRunner for RecordsCancelledIndex {
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
