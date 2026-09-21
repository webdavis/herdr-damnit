//! Cancelling a `dam` run, both ways it happens: the pane asking, and a read outliving its
//! deadline. Nothing here waits for a guessed interval: a test blocks on the fake's own argv line
//! to know the child is running, and on the result channel to know it answered. A loaded machine
//! makes these tests slower and never flakier.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use herdr_damnit_adapters::{Cancel, ProcessDamRunner};
use herdr_damnit_application::DamRunner;
use std::os::unix::process::CommandExt;

/// Longer than any fake in this file needs, so a machine under load waits rather than fails.
const PATIENCE: Duration = Duration::from_secs(20);

/// How long a fake meant to be cancelled pretends to work for. Well past every deadline and grace
/// under test, so a passing test is proof the cancellation fired rather than that the fake
/// happened to finish.
const FOREVER_MS: &str = "30000";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("herdr-damnit-{name}-{}", std::process::id()));
    let _ = std::fs::remove_file(&path);
    path
}

fn runner() -> ProcessDamRunner {
    ProcessDamRunner::new(vec![env!("CARGO_BIN_EXE_fake-dam").to_string()])
}

fn words(argv: &[&str]) -> Vec<String> {
    argv.iter().map(|word| word.to_string()).collect()
}

/// Every knob of the fake, cleared. Each test then sets only the ones it needs, so a test that
/// panics before its own cleanup cannot hand its environment to the next one in this file.
fn reset_fake_env() {
    for knob in [
        "FAKE_DAM_FIXTURE_DIR",
        "FAKE_DAM_FIXTURE",
        "FAKE_DAM_EXIT",
        "FAKE_DAM_STDERR",
        "FAKE_DAM_SLEEP_MS",
        "FAKE_DAM_LOG",
        "FAKE_DAM_IGNORE_SIGINT",
    ] {
        unsafe { std::env::remove_var(knob) };
    }
}

fn set(knob: &str, value: impl AsRef<std::ffi::OsStr>) {
    unsafe { std::env::set_var(knob, value) };
}

/// One fake, in the process group `group` names, with `0` making it a leader of its own.
fn fake_in_group(group: i32) -> Child {
    Command::new(env!("CARGO_BIN_EXE_fake-dam"))
        .args(["push", "--json"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(group)
        .spawn()
        .expect("it spawned")
}

/// Block until the fake's log holds `lines` argv lines, which is how a test knows each child is
/// past its own startup and has its interrupt handler installed.
fn await_lines(log: &Path, lines: usize) {
    let deadline = Instant::now() + PATIENCE;
    while Instant::now() < deadline {
        if logged(log) >= lines {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("the fakes logged {} of {lines} lines", logged(log));
}

fn logged(log: &Path) -> usize {
    std::fs::read_to_string(log)
        .map(|text| text.lines().filter(|line| !line.trim().is_empty()).count())
        .unwrap_or(0)
}

fn interrupts(log: &Path) -> usize {
    std::fs::read_to_string(log)
        .expect("a log")
        .matches("SIGINT")
        .count()
}

#[test]
fn cancelling_sends_sigint_and_dam_answers_with_its_cancelled_code() {
    reset_fake_env();
    let log = scratch("cancel-log");
    set("FAKE_DAM_LOG", &log);
    set("FAKE_DAM_SLEEP_MS", FOREVER_MS);
    set("FAKE_DAM_FIXTURE_DIR", fixtures());
    set("FAKE_DAM_FIXTURE", "push-ok");

    let job = runner()
        .spawn(&words(&["push", "--json"]))
        .expect("it spawned");
    await_lines(&log, 1);
    (job.cancel)();

    let finished = job.results.recv_timeout(PATIENCE).expect("one result");
    assert_eq!(finished.code, Some(3));
    assert_eq!(interrupts(&log), 1, "the fake recorded no interrupt");
}

#[test]
fn a_read_past_its_deadline_is_cancelled_without_the_pane_asking() {
    reset_fake_env();
    set("FAKE_DAM_SLEEP_MS", FOREVER_MS);
    set("FAKE_DAM_FIXTURE_DIR", fixtures());
    set("FAKE_DAM_FIXTURE", "ls");

    let job = runner()
        .spawn_with_deadline(
            &words(&["ls", "!done", "--json"]),
            Some(Duration::from_millis(200)),
        )
        .expect("it spawned");

    assert_eq!(
        job.results.recv_timeout(PATIENCE).expect("one result").code,
        Some(3)
    );
}

#[test]
fn a_job_inside_its_deadline_is_left_alone() {
    reset_fake_env();
    set("FAKE_DAM_FIXTURE_DIR", fixtures());
    set("FAKE_DAM_FIXTURE", "status-clean");

    let job = runner()
        .spawn_with_deadline(&words(&["status", "--json"]), Some(PATIENCE))
        .expect("it spawned");

    let finished = job.results.recv_timeout(PATIENCE).expect("one result");
    assert_eq!(finished.code, Some(0));
    assert!(
        finished.stdout.contains("\"staged\""),
        "{}",
        finished.stdout
    );
}

/// A run with no deadline is never cancelled on its own, which is what an exclusive job gets:
/// `dam`'s own per-remote deadline bounds it.
#[test]
fn a_job_with_no_deadline_answers_for_itself() {
    reset_fake_env();
    set("FAKE_DAM_FIXTURE_DIR", fixtures());
    set("FAKE_DAM_FIXTURE", "push-ok");

    let job = runner()
        .spawn_with_deadline(&words(&["push", "--json"]), None)
        .expect("it spawned");

    assert_eq!(
        job.results.recv_timeout(PATIENCE).expect("one result").code,
        Some(0)
    );
}

/// The signal reaches the whole process group, which is where `dam` puts the remote helper that
/// is doing the waiting, so a second member of the group takes it too.
#[test]
fn the_interrupt_reaches_the_group_rather_than_its_leader_alone() {
    reset_fake_env();
    let log = scratch("group-log");
    set("FAKE_DAM_LOG", &log);
    set("FAKE_DAM_SLEEP_MS", FOREVER_MS);

    let mut leader = fake_in_group(0);
    let group = i32::try_from(leader.id()).expect("a pid");
    let mut helper = fake_in_group(group);
    await_lines(&log, 2);

    Cancel::of(leader.id()).interrupt();

    assert_eq!(leader.wait().expect("it exited").code(), Some(3));
    assert_eq!(helper.wait().expect("it exited").code(), Some(3));
    assert_eq!(interrupts(&log), 2, "the helper was left running");
}

/// A `dam` deaf to `SIGINT` is killed once the grace runs out, and a killed child carries no exit
/// code at all. Driven at `Cancel` with a grace of its own, because the production grace is two
/// seconds and no test in this repository may take that long.
#[test]
fn a_dam_that_ignores_the_interrupt_is_killed_and_carries_no_code() {
    reset_fake_env();
    let log = scratch("kill-log");
    set("FAKE_DAM_LOG", &log);
    set("FAKE_DAM_SLEEP_MS", FOREVER_MS);
    set("FAKE_DAM_IGNORE_SIGINT", "1");

    let mut child = fake_in_group(0);
    await_lines(&log, 1);

    let started = Instant::now();
    Cancel::with_grace(child.id(), Duration::from_millis(100)).interrupt();

    assert_eq!(
        child.wait().expect("it exited").code(),
        None,
        "a killed child reports a signal rather than a code"
    );
    assert!(
        started.elapsed() >= Duration::from_millis(100),
        "the kill skipped the grace: {:?}",
        started.elapsed()
    );
    assert_eq!(interrupts(&log), 0, "the fake was not deaf after all");
}

/// A group that has already gone is not waited out, which is what the grace loop polls for so an
/// interrupted `dam` is never killed on top of the interrupt it already took.
#[test]
fn a_group_that_has_already_gone_is_not_waited_out() {
    reset_fake_env();
    let mut child = fake_in_group(0);
    let cancel = Cancel::with_grace(child.id(), Duration::from_secs(20));
    child.wait().expect("it exited");

    let started = Instant::now();
    cancel.interrupt();

    assert!(
        started.elapsed() < Duration::from_secs(2),
        "interrupt waited out a grace nobody needed: {:?}",
        started.elapsed()
    );
}
