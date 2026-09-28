use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

mod support;

use herdr_damnit_adapters::{Cancel, ProcessDamRunner};
use herdr_damnit_application::DamRunner;
use std::os::unix::process::CommandExt;
use support::Scratch;
use support::fake_env::FakeEnv;

const PATIENCE: Duration = Duration::from_secs(20);

const FOREVER_MS: &str = "30000";

const LEADER_OF_A_NEW_GROUP: i32 = 0;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn runner() -> ProcessDamRunner {
    ProcessDamRunner::new(vec![env!("CARGO_BIN_EXE_fake-dam").to_string()])
}

fn words(argv: &[&str]) -> Vec<String> {
    argv.iter().map(|word| word.to_string()).collect()
}

const SIGNAL_PATIENCE: Duration = Duration::from_secs(5);

const POLL: Duration = Duration::from_millis(5);

struct Fake {
    name: &'static str,
    child: Child,
    log: PathBuf,
}

impl Fake {
    fn spawn(env: &FakeEnv, name: &'static str, scratch: &Scratch, process_group: i32) -> Self {
        let log = scratch.file(&format!("{name}.jsonl"));
        env.set("FAKE_DAM_LOG", &log);
        let child = Command::new(env!("CARGO_BIN_EXE_fake-dam"))
            .args(["push", "--json"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(process_group)
            .spawn()
            .expect("it spawned");
        Self { name, child, log }
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }

    fn await_handler_installed(&mut self) {
        let deadline = Instant::now() + PATIENCE;
        while Instant::now() < deadline {
            if logged(&self.log) >= 1 {
                return;
            }
            if let Some(status) = self.child.try_wait().expect("the child's state") {
                panic!("{} exited {status} before logging its argv", self.name);
            }
            std::thread::sleep(POLL);
        }
        panic!("{} logged no argv within {PATIENCE:?}", self.name);
    }

    fn await_exit(&mut self) -> std::process::ExitStatus {
        let deadline = Instant::now() + SIGNAL_PATIENCE;
        while Instant::now() < deadline {
            if let Some(status) = self.child.try_wait().expect("the child's state") {
                return status;
            }
            std::thread::sleep(POLL);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        panic!(
            "{} was still running {SIGNAL_PATIENCE:?} after the signal",
            self.name
        );
    }

    fn interrupts(&self) -> usize {
        interrupts(&self.log)
    }
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
fn a_read_past_its_deadline_is_cancelled_without_the_pane_asking() {
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
fn the_interrupt_reaches_dams_helper_in_the_group_rather_than_the_leader_alone() {
    let env = FakeEnv::lock_cleared();
    let scratch = Scratch::new("group");
    env.set("FAKE_DAM_SLEEP_MS", FOREVER_MS);

    let mut leader = Fake::spawn(&env, "the leader", &scratch, LEADER_OF_A_NEW_GROUP);
    let group = i32::try_from(leader.pid()).expect("a pid");
    let mut helper = Fake::spawn(&env, "the helper", &scratch, group);
    leader.await_handler_installed();
    helper.await_handler_installed();

    Cancel::of(leader.pid()).interrupt();

    assert_eq!(leader.await_exit().code(), Some(3));
    assert_eq!(helper.await_exit().code(), Some(3));
    assert_eq!(leader.interrupts(), 1, "the leader took no interrupt");
    assert_eq!(helper.interrupts(), 1, "the helper was left running");
}

#[test]
fn a_dam_that_ignores_the_interrupt_is_killed_and_carries_no_code() {
    let env = FakeEnv::lock_cleared();
    let scratch = Scratch::new("kill");
    env.set("FAKE_DAM_SLEEP_MS", FOREVER_MS);
    env.set("FAKE_DAM_IGNORE_SIGINT", "1");

    let mut deaf = Fake::spawn(&env, "the deaf dam", &scratch, LEADER_OF_A_NEW_GROUP);
    deaf.await_handler_installed();

    let started = Instant::now();
    Cancel::with_grace(deaf.pid(), Duration::from_millis(100)).interrupt();

    assert_eq!(
        deaf.await_exit().code(),
        None,
        "a killed child reports a signal rather than a code"
    );
    assert!(
        started.elapsed() >= Duration::from_millis(100),
        "the kill skipped the grace: {:?}",
        started.elapsed()
    );
    assert_eq!(deaf.interrupts(), 0, "the fake was not deaf after all");
}

#[test]
fn a_group_that_has_already_gone_is_not_waited_out() {
    let env = FakeEnv::lock_cleared();
    let scratch = Scratch::new("already-gone");
    let mut gone = Fake::spawn(&env, "the finished dam", &scratch, LEADER_OF_A_NEW_GROUP);
    let cancel = Cancel::with_grace(gone.pid(), Duration::from_secs(20));
    gone.await_exit();

    let started = Instant::now();
    cancel.interrupt();

    assert!(
        started.elapsed() < Duration::from_secs(2),
        "interrupt waited out a grace nobody needed: {:?}",
        started.elapsed()
    );
}
