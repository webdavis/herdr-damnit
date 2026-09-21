//! The fake `dam` every other test drives. This file proves the double itself behaves, so a
//! failure in a later test is a failure in the pane rather than in its double.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fake() -> &'static str {
    env!("CARGO_BIN_EXE_fake-dam")
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("herdr-damnit-{name}-{}", std::process::id()));
    let _ = std::fs::remove_file(&path);
    path
}

#[test]
fn the_fake_replays_the_fixture_named_by_its_subcommand() {
    let output = Command::new(fake())
        .args(["status", "--json"])
        .env("FAKE_DAM_FIXTURE_DIR", fixtures())
        .env("FAKE_DAM_FIXTURE", "status-clean")
        .output()
        .expect("the fake ran");

    assert!(output.status.success());
    let document: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the fixture is JSON");
    assert!(document.get("staged").is_some(), "{document}");
}

#[test]
fn the_fake_appends_one_json_line_of_argv_per_call() {
    let log = scratch("argv-log");
    for argv in [vec!["status", "--json"], vec!["ls", "!done", "--json"]] {
        Command::new(fake())
            .args(&argv)
            .env("FAKE_DAM_LOG", &log)
            .env("FAKE_DAM_FIXTURE_DIR", fixtures())
            .env("FAKE_DAM_FIXTURE", "status-clean")
            .output()
            .expect("the fake ran");
    }

    let lines: Vec<Vec<String>> = std::fs::read_to_string(&log)
        .expect("a log")
        .lines()
        .map(|line| serde_json::from_str(line).expect("a json line"))
        .collect();
    assert_eq!(lines[0], vec!["status", "--json"]);
    assert_eq!(lines[1], vec!["ls", "!done", "--json"]);
}

/// A refusal is exit 4 and one error document on standard error, which is `dam` 0.2.0's contract
/// under `--json`.
#[test]
fn the_fake_refuses_with_the_code_and_the_document_it_was_given() {
    let document = r#"{"error":{"kind":"refused","rule":"no_such_object","message":"no object matches \"zzzzzzz\"","oids":[]}}"#;
    let output = Command::new(fake())
        .args(["show", "zzzzzzz", "--json"])
        .env("FAKE_DAM_EXIT", "4")
        .env("FAKE_DAM_STDERR", document)
        .output()
        .expect("the fake ran");

    assert_eq!(output.status.code(), Some(4));
    assert_eq!(String::from_utf8_lossy(&output.stderr).trim(), document);
    assert!(output.stdout.is_empty(), "a failing dam prints no report");
}

/// clap answers a bad command line before `dam` runs, so exit 2 carries usage text and no
/// document. The pane has to read that shape too.
#[test]
fn the_fake_can_also_answer_the_way_clap_does() {
    let output = Command::new(fake())
        .args(["ls", "--nope"])
        .env("FAKE_DAM_EXIT", "2")
        .env("FAKE_DAM_STDERR", "error: unexpected argument '--nope'")
        .output()
        .expect("the fake ran");

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "error: unexpected argument '--nope'"
    );
}

#[test]
fn the_fake_sleeps_before_answering_when_it_is_asked_to() {
    let started = std::time::Instant::now();
    Command::new(fake())
        .args(["push", "--json"])
        .env("FAKE_DAM_SLEEP_MS", "150")
        .env("FAKE_DAM_FIXTURE_DIR", fixtures())
        .env("FAKE_DAM_FIXTURE", "push-ok")
        .output()
        .expect("the fake ran");

    assert!(started.elapsed() >= std::time::Duration::from_millis(150));
}

#[test]
fn a_fixture_that_is_not_there_is_an_empty_object_rather_than_a_panic() {
    let output = Command::new(fake())
        .args(["log", "--json"])
        .env("FAKE_DAM_FIXTURE_DIR", fixtures())
        .env("FAKE_DAM_FIXTURE", "nothing-like-this")
        .output()
        .expect("the fake ran");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "{}");
}

#[test]
fn the_version_flag_answers_the_captured_line() {
    let output = Command::new(fake())
        .arg("--version")
        .env("FAKE_DAM_FIXTURE_DIR", fixtures())
        .output()
        .expect("the fake ran");

    assert!(
        String::from_utf8_lossy(&output.stdout).starts_with("dam "),
        "{:?}",
        output.stdout
    );
}

/// Without `FAKE_DAM_FIXTURE` the subcommand is the fixture name, which is what lets a test that
/// drives several reads point at one directory and get a different document per call.
#[test]
fn the_subcommand_alone_chooses_the_fixture_when_no_name_overrides_it() {
    let output = Command::new(fake())
        .args(["ls", "!done", "--json"])
        .env("FAKE_DAM_FIXTURE_DIR", fixtures())
        .output()
        .expect("the fake ran");

    let document: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the fixture is JSON");
    assert_eq!(
        document["objects"].as_array().map(Vec::len),
        Some(3),
        "{document}"
    );
}

/// An interrupt reaches the fake while it sleeps, and it answers the way a cancelled `dam` does:
/// exit 3, with the signal recorded for the cancellation tests to read.
#[test]
fn an_interrupt_is_recorded_and_answered_with_the_cancelled_code() {
    let log = scratch("signal-log");
    let mut child = Command::new(fake())
        .args(["push", "--json"])
        .env("FAKE_DAM_LOG", &log)
        .env("FAKE_DAM_SLEEP_MS", "3000")
        .env("FAKE_DAM_FIXTURE_DIR", fixtures())
        .env("FAKE_DAM_FIXTURE", "push-ok")
        .spawn()
        .expect("the fake ran");

    std::thread::sleep(std::time::Duration::from_millis(150));
    let pid = i32::try_from(child.id()).expect("a pid");
    assert_eq!(unsafe { libc::kill(pid, libc::SIGINT) }, 0);

    assert_eq!(child.wait().expect("it exited").code(), Some(3));
    assert!(
        std::fs::read_to_string(&log)
            .expect("a log")
            .contains("SIGINT"),
        "the fake recorded no interrupt"
    );
}
