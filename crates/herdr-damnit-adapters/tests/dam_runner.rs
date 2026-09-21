use std::path::{Path, PathBuf};
use std::time::Duration;

mod support;

use herdr_damnit_adapters::ProcessDamRunner;
use herdr_damnit_application::{DamRunner, SpawnError};
use support::Scratch;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn runner() -> ProcessDamRunner {
    ProcessDamRunner::new(vec![env!("CARGO_BIN_EXE_fake-dam").to_string()])
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

fn words(argv: &[&str]) -> Vec<String> {
    argv.iter().map(|word| word.to_string()).collect()
}

#[test]
fn a_run_answers_once_with_its_code_and_its_output() {
    let _env = fake_env();
    unsafe {
        std::env::set_var("FAKE_DAM_FIXTURE_DIR", fixtures());
        std::env::set_var("FAKE_DAM_FIXTURE", "status-clean");
    }
    let job = runner()
        .spawn(&words(&["status", "--json"]))
        .expect("it spawned");

    let finished = job
        .results
        .recv_timeout(Duration::from_secs(5))
        .expect("one result");
    assert_eq!(finished.code, Some(0));
    assert!(
        finished.stdout.contains("\"staged\""),
        "{}",
        finished.stdout
    );
    assert!(finished.stderr.is_empty());
}

#[test]
fn spawning_does_not_wait_for_the_child() {
    let _env = fake_env();
    unsafe {
        std::env::set_var("FAKE_DAM_SLEEP_MS", "400");
        std::env::set_var("FAKE_DAM_FIXTURE_DIR", fixtures());
        std::env::set_var("FAKE_DAM_FIXTURE", "push-ok");
    }
    let started = std::time::Instant::now();
    let job = runner()
        .spawn(&words(&["push", "--json"]))
        .expect("it spawned");
    let returned = started.elapsed();

    assert!(
        returned < Duration::from_millis(200),
        "spawn blocked for {returned:?}"
    );
    job.results
        .recv_timeout(Duration::from_secs(5))
        .expect("one result");
}

#[test]
fn a_failing_run_carries_its_code_and_its_standard_error() {
    let _env = fake_env();
    let document = r#"{"error":{"kind":"refused","rule":"no_such_object","message":"no object matches \"zzzzzzz\"","oids":[]}}"#;
    unsafe {
        std::env::set_var("FAKE_DAM_EXIT", "4");
        std::env::set_var("FAKE_DAM_STDERR", document);
    }
    let job = runner()
        .spawn(&words(&["show", "zzzzzzz", "--json"]))
        .expect("it spawned");
    let finished = job
        .results
        .recv_timeout(Duration::from_secs(5))
        .expect("one result");

    assert_eq!(finished.code, Some(4));
    assert_eq!(finished.stderr.trim(), document);
}

#[test]
fn a_dam_that_is_not_there_is_a_not_found_rather_than_an_error_string() {
    let _env = fake_env();
    let runner = ProcessDamRunner::new(vec!["no-such-dam-anywhere".to_string()]);
    assert!(matches!(
        runner.spawn(&words(&["status"])),
        Err(SpawnError::NotFound)
    ));
}

/// An argv with no binary in it could spawn nothing at all, which the config refuses and this
/// answers for anyway rather than indexing off the end of the list.
#[test]
fn an_empty_argv_is_a_not_found_rather_than_a_panic() {
    let _env = fake_env();
    assert!(matches!(
        ProcessDamRunner::new(Vec::new()).spawn(&words(&["status"])),
        Err(SpawnError::NotFound)
    ));
}

#[test]
fn the_configured_argv_leads_and_the_commands_arguments_follow_it() {
    let _env = fake_env();
    let scratch = Scratch::new("runner");
    let log = scratch.file("argv.jsonl");
    unsafe {
        std::env::set_var("FAKE_DAM_LOG", &log);
        std::env::set_var("FAKE_DAM_FIXTURE_DIR", fixtures());
        std::env::set_var("FAKE_DAM_FIXTURE", "ls");
    }
    let job = ProcessDamRunner::new(vec![
        env!("CARGO_BIN_EXE_fake-dam").to_string(),
        "--leading".to_string(),
    ])
    .spawn(&words(&["ls", "!done", "--json"]))
    .expect("it spawned");
    job.results
        .recv_timeout(Duration::from_secs(5))
        .expect("one result");

    let line = std::fs::read_to_string(&log).expect("a log");
    let argv: Vec<String> =
        serde_json::from_str(line.lines().next().expect("a line")).expect("json");
    assert_eq!(argv, words(&["--leading", "ls", "!done", "--json"]));
}
