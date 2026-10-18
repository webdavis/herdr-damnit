use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

mod support;

use herdr_damnit_adapters::ProcessDamRunner;
use herdr_damnit_application::DamRunner;
use support::Scratch;
use support::fake_env::FakeEnv;
use support::fake_log::{PATIENCE, POLL, interrupts, logged};

const FOREVER_MS: &str = "30000";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn runner() -> ProcessDamRunner {
    ProcessDamRunner::new(vec![env!("CARGO_BIN_EXE_fake-dam").to_string()])
}

fn words(argv: &[&str]) -> Vec<String> {
    argv.iter().map(|word| word.to_string()).collect()
}

fn await_argv_line_of_a_job_with_no_pid_to_ask(log: &Path, whose: &str) {
    let deadline = Instant::now() + PATIENCE;
    while Instant::now() < deadline {
        if logged(log) >= 1 {
            return;
        }
        std::thread::sleep(POLL);
    }
    panic!("{whose} logged no argv within {PATIENCE:?}");
}

#[test]
fn cancelling_sends_sigint_and_dam_answers_with_its_cancelled_code() {
    let env = FakeEnv::lock_cleared();
    let scratch = Scratch::new("cancel-log");
    let log = scratch.file("argv.jsonl");
    env.set("FAKE_DAM_LOG", &log);
    env.set("FAKE_DAM_SLEEP_MS", FOREVER_MS);
    env.set("FAKE_DAM_FIXTURE_DIR", fixtures());
    env.set("FAKE_DAM_FIXTURE", "push-ok");

    let job = runner()
        .spawn(&words(&["push", "--json"]))
        .expect("it spawned");
    await_argv_line_of_a_job_with_no_pid_to_ask(&log, "the push");
    (job.cancel)();

    let finished = job.results.recv_timeout(PATIENCE).expect("one result");
    assert_eq!(finished.code, Some(3));
    assert_eq!(interrupts(&log), 1, "the fake recorded no interrupt");
}

#[test]
fn the_doctor_past_its_own_deadline_is_cancelled_without_the_pane_asking() {
    let env = FakeEnv::lock_cleared();
    env.set("FAKE_DAM_SLEEP_MS", FOREVER_MS);
    env.set("FAKE_DAM_FIXTURE_DIR", fixtures());
    env.set("FAKE_DAM_FIXTURE", "ls");

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
    let env = FakeEnv::lock_cleared();
    env.set("FAKE_DAM_FIXTURE_DIR", fixtures());
    env.set("FAKE_DAM_FIXTURE", "status-clean");

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

#[test]
fn a_job_with_no_deadline_such_as_an_exclusive_one_answers_for_itself() {
    let env = FakeEnv::lock_cleared();
    env.set("FAKE_DAM_FIXTURE_DIR", fixtures());
    env.set("FAKE_DAM_FIXTURE", "push-ok");

    let job = runner()
        .spawn_with_deadline(&words(&["push", "--json"]), None)
        .expect("it spawned");

    assert_eq!(
        job.results.recv_timeout(PATIENCE).expect("one result").code,
        Some(0)
    );
}

#[test]
fn cancelling_returns_at_once_and_a_deaf_dam_is_still_killed_after_the_grace() {
    let env = FakeEnv::lock_cleared();
    let scratch = Scratch::new("cancel-off-thread");
    let log = scratch.file("argv.jsonl");
    env.set("FAKE_DAM_LOG", &log);
    env.set("FAKE_DAM_SLEEP_MS", FOREVER_MS);
    env.set("FAKE_DAM_IGNORE_SIGINT", "1");

    let job = runner()
        .spawn(&words(&["push", "--json"]))
        .expect("it spawned");
    await_argv_line_of_a_job_with_no_pid_to_ask(&log, "the deaf push");

    let asked = Instant::now();
    (job.cancel)();
    let returned_after = asked.elapsed();

    let finished = job
        .results
        .recv_timeout(Duration::from_secs(3))
        .expect("the deaf dam was killed within the grace plus a margin");
    assert_eq!(finished.code, None, "a killed child carries no code");
    assert!(
        returned_after < Duration::from_millis(100),
        "cancel blocked the caller for {returned_after:?}"
    );
}
