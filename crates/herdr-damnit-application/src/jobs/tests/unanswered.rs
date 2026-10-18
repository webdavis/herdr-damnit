use crate::argv;
use crate::fakes::FakeDam;
use crate::jobs::{Request, SyncKind};

use super::harness::jobs_over;

#[test]
fn cancelling_reaches_the_exclusive_job_before_any_read_that_started_earlier() {
    let dam = FakeDam::default();
    let mut jobs = jobs_over(&dam);
    jobs.submit(Request::List("!done".to_string()));
    jobs.submit(Request::Sync(SyncKind::Push, argv::push()));

    assert!(jobs.cancel_current());
    assert_eq!(dam.cancelled(), vec![1]);
}

#[test]
fn a_job_whose_thread_died_without_answering_leaves_the_table_rather_than_blocking_later_ones() {
    let dam = FakeDam::default();
    let mut jobs = jobs_over(&dam);
    jobs.submit(Request::Sync(SyncKind::Push, argv::push()));
    dam.hang_up();

    assert!(jobs.drain().is_empty());
    assert_eq!(jobs.in_flight(), 0);
    assert_eq!(jobs.exclusive(), None);
}

#[test]
fn with_no_exclusive_job_the_cancel_reaches_the_oldest_read() {
    let dam = FakeDam::default();
    let mut jobs = jobs_over(&dam);
    jobs.submit(Request::List("!done".to_string()));
    jobs.submit(Request::Status);

    assert!(jobs.cancel_current());
    assert_eq!(dam.cancelled(), vec![0]);
}

#[test]
fn cancelling_with_nothing_in_flight_says_so() {
    let mut jobs = jobs_over(&FakeDam::default());
    assert!(!jobs.cancel_current());
}

#[test]
fn a_read_superseded_by_a_newer_one_of_its_kind_is_cancelled_rather_than_left_running() {
    let dam = FakeDam::default();
    let mut jobs = jobs_over(&dam);
    jobs.submit(Request::List("!done".to_string()));
    jobs.submit(Request::List("done".to_string()));

    assert_eq!(dam.cancelled(), vec![0]);
    assert_eq!(jobs.in_flight(), 1);
}
