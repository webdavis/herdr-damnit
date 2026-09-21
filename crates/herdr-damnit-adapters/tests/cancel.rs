//! Cancelling a `dam` run, both ways it happens: the pane asking, and a read outliving its
//! deadline. Nothing here waits for a guessed interval: a test blocks on the fake's own argv line
//! to know the child is running, and on the result channel to know it answered. A loaded machine
//! makes these tests slower and never flakier.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

mod support;

use herdr_damnit_adapters::{Cancel, ProcessDamRunner};
use herdr_damnit_application::DamRunner;
use std::os::unix::process::CommandExt;
use support::Scratch;

/// Longer than any fake in this file needs, so a machine under load waits rather than fails.
const PATIENCE: Duration = Duration::from_secs(20);

/// How long a fake meant to be cancelled pretends to work for. Well past every deadline and grace
/// under test, so a passing test is proof the cancellation fired rather than that the fake
/// happened to finish.
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

/// The fake's knobs live in this process's environment, which every test in this file shares.
/// A test holds this lock for its whole body and starts from a cleared environment, so the file
/// is correct under any thread count rather than only under `--test-threads=1`.
fn fake_env() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
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
    guard
}

fn set(knob: &str, value: impl AsRef<std::ffi::OsStr>) {
    unsafe { std::env::set_var(knob, value) };
}

/// How long a signalled fake gets to actually exit. Short next to `PATIENCE` on purpose: past it
/// the signal ladder is broken, and a broken ladder must read as a red rather than as a suite that
/// hangs until the fake's own sleep runs out.
const SIGNAL_PATIENCE: Duration = Duration::from_secs(5);

/// The gap between two looks at an observable event. Nothing here sleeps in place of
/// synchronization; this is only how often a wait re-reads what it is waiting on.
const POLL: Duration = Duration::from_millis(5);

/// One fake, the log it writes its own argv line to, and a name for the failure messages. Each
/// fake gets a log of its own so a wait can say which child is missing rather than reporting a
/// total across both.
struct Fake {
    name: &'static str,
    child: Child,
    log: PathBuf,
}

impl Fake {
    /// Spawn a fake into the process group `group` names, `0` making it a leader of its own.
    fn spawn(name: &'static str, scratch: &Scratch, group: i32) -> Self {
        let log = scratch.file(&format!("{name}.jsonl"));
        set("FAKE_DAM_LOG", &log);
        let child = Command::new(env!("CARGO_BIN_EXE_fake-dam"))
            .args(["push", "--json"])
            .stdin(Stdio::null())
            // Nothing here reads the child's output, so it is given no pipe: a fake cannot stall
            // on a buffer this test would never drain.
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(group)
            .spawn()
            .expect("it spawned");
        Self { name, child, log }
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Block until this fake has logged its argv line, which proves it is past its own startup and
    /// has its interrupt handler installed. A fake that exited without logging is reported at once
    /// rather than waited out, because there is nothing left to wait for.
    fn await_start(&mut self) {
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

    /// This fake's exit status, or a failure naming it once `SIGNAL_PATIENCE` runs out. The
    /// timeout path kills the child so a failing run leaves nothing sleeping behind it.
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

/// Block until `log` holds one argv line, for a child this test did not spawn itself and so cannot
/// ask about: `RunningJob` hands out a cancel and a receiver, never a pid.
fn await_line(log: &Path, whose: &str) {
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
    let _env = fake_env();
    let scratch = Scratch::new("cancel-log");
    let log = scratch.file("argv.jsonl");
    set("FAKE_DAM_LOG", &log);
    set("FAKE_DAM_SLEEP_MS", FOREVER_MS);
    set("FAKE_DAM_FIXTURE_DIR", fixtures());
    set("FAKE_DAM_FIXTURE", "push-ok");

    let job = runner()
        .spawn(&words(&["push", "--json"]))
        .expect("it spawned");
    await_line(&log, "the push");
    (job.cancel)();

    let finished = job.results.recv_timeout(PATIENCE).expect("one result");
    assert_eq!(finished.code, Some(3));
    assert_eq!(interrupts(&log), 1, "the fake recorded no interrupt");
}

#[test]
fn a_read_past_its_deadline_is_cancelled_without_the_pane_asking() {
    let _env = fake_env();
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
    let _env = fake_env();
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
    let _env = fake_env();
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
    let _env = fake_env();
    let scratch = Scratch::new("group");
    set("FAKE_DAM_SLEEP_MS", FOREVER_MS);

    let mut leader = Fake::spawn("the leader", &scratch, 0);
    let group = i32::try_from(leader.pid()).expect("a pid");
    let mut helper = Fake::spawn("the helper", &scratch, group);
    leader.await_start();
    helper.await_start();

    Cancel::of(leader.pid()).interrupt();

    assert_eq!(leader.await_exit().code(), Some(3));
    assert_eq!(helper.await_exit().code(), Some(3));
    assert_eq!(leader.interrupts(), 1, "the leader took no interrupt");
    assert_eq!(helper.interrupts(), 1, "the helper was left running");
}

/// A `dam` deaf to `SIGINT` is killed once the grace runs out, and a killed child carries no exit
/// code at all. Driven at `Cancel` with a grace of its own, because the production grace is two
/// seconds and no test in this repository may take that long.
#[test]
fn a_dam_that_ignores_the_interrupt_is_killed_and_carries_no_code() {
    let _env = fake_env();
    let scratch = Scratch::new("kill");
    set("FAKE_DAM_SLEEP_MS", FOREVER_MS);
    set("FAKE_DAM_IGNORE_SIGINT", "1");

    let mut deaf = Fake::spawn("the deaf dam", &scratch, 0);
    deaf.await_start();

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

/// A group that has already gone is not waited out, which is what the grace loop polls for so an
/// interrupted `dam` is never killed on top of the interrupt it already took.
#[test]
fn a_group_that_has_already_gone_is_not_waited_out() {
    let _env = fake_env();
    let scratch = Scratch::new("already-gone");
    let mut gone = Fake::spawn("the finished dam", &scratch, 0);
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
