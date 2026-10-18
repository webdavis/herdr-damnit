use std::path::{Path, PathBuf};
use std::process::Command;

mod support;

use support::Scratch;

fn fake() -> &'static str {
    env!("CARGO_BIN_EXE_fake-dam")
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
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

#[test]
fn a_knob_given_before_the_argv_works_like_the_environment_and_is_not_logged() {
    let scratch = Scratch::new("argv-knobs");
    let log = scratch.file("argv.jsonl");
    let output = Command::new(fake())
        .arg(format!("FAKE_DAM_LOG={}", log.display()))
        .arg(format!("FAKE_DAM_FIXTURE_DIR={}", fixtures().display()))
        .args(["FAKE_DAM_FIXTURE=status-clean", "status", "--json"])
        .output()
        .expect("the fake ran");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("\"staged\""));
    assert_eq!(
        std::fs::read_to_string(&log).expect("a log").trim(),
        r#"["status","--json"]"#
    );
}
