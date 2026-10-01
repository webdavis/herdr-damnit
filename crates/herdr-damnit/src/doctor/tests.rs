use super::*;

const CLEAN: &str = r#"{"staged":[],"unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#;

const ONE_REMOTE: &str = "example  todoist://  pulled 3h ago  never pushed\n";

#[test]
fn a_dam_that_answers_reports_its_version_its_status_and_its_remotes() {
    let report =
        report_from("dam 0.2.0\n", CLEAN, Ok(ONE_REMOTE.to_string())).expect("it answered");

    assert!(report.contains("dam:     0.2.0"), "{report}");
    assert!(report.contains("status:  ok"), "{report}");
    assert!(
        report.contains("remotes: example  todoist://  pulled 3h ago"),
        "{report}"
    );
}

#[test]
fn no_configured_remote_is_reported_rather_than_left_blank() {
    let report = report_from("dam 0.2.0\n", CLEAN, Ok(String::new())).expect("it answered");

    assert!(report.contains("remotes: none configured"), "{report}");
}

#[test]
fn a_remote_list_dam_refused_is_reported_with_dams_own_reason() {
    let report = report_from(
        "dam 0.2.0\n",
        CLEAN,
        Err("dam: unrecognized subcommand 'remote'".to_string()),
    )
    .expect("the version and status still answered");

    assert!(
        report.contains("remotes: not available: dam: unrecognized subcommand 'remote'"),
        "{report}"
    );
}

#[test]
fn a_dam_below_the_floor_is_reported_as_the_problem_it_is() {
    let error = report_from("dam 0.0.9\n", CLEAN, Ok(String::new())).expect_err("it refuses");

    assert!(
        error.contains("is older than the 0.2 this pane needs"),
        "{error}"
    );
}

#[test]
fn a_status_missing_a_key_is_reported_by_name() {
    let error = report_from(
        "dam 0.2.0\n",
        r#"{"staged":[],"unstaged":[],"conflicts":[],"notices":[]}"#,
        Ok(String::new()),
    )
    .expect_err("it refuses");

    assert!(error.contains("\"unpushed\""), "{error}");
}

#[test]
fn a_newer_dam_is_reported_with_the_note_the_handshake_would_have_shown() {
    let report = report_from("dam 0.4.0\n", CLEAN, Ok(String::new())).expect("it answered");

    assert!(report.contains("newer than this pane knows"), "{report}");
}

#[test]
fn a_dam_that_is_not_there_names_the_path_it_searched() {
    let report = missing_dam_report(&["dam".to_string()], "/a/bin:/b/bin");

    assert!(report.contains("dam is not on PATH"), "{report}");
    assert!(report.contains("searched for: dam"), "{report}");
    assert!(report.contains("PATH: /a/bin:/b/bin"), "{report}");
}
