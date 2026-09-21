use super::*;

fn refusal(rule: &str, message: &str, oids: &[&str]) -> ErrorDocument {
    ErrorDocument {
        kind: ErrorKind::Refused,
        message: message.to_string(),
        rule: Some(Rule::named(rule)),
        oids: oids.iter().map(|oid| Oid::new(*oid)).collect(),
    }
}

#[test]
fn a_refusal_carries_dams_own_sentence_its_rule_and_its_oids() {
    let failure = classify(
        Some(4),
        Some(refusal(
            "blocked",
            "98d8780 cannot be completed: child a9db854 is open",
            &["98d8780", "a9db854"],
        )),
        "",
    );
    assert_eq!(
        failure,
        Failure::Refused {
            rule: Rule::Blocked,
            said: "98d8780 cannot be completed: child a9db854 is open".to_string(),
            oids: vec![Oid::new("98d8780"), Oid::new("a9db854")],
        }
    );
    assert_eq!(
        message(&failure),
        "98d8780 cannot be completed: child a9db854 is open"
    );
    assert!(leaves_model_untouched(&failure));
}

#[test]
fn every_rule_word_dam_publishes_has_a_variant_of_its_own() {
    for word in [
        "blocked",
        "cycle",
        "exclusive_label",
        "unknown_category",
        "no_such_object",
        "no_working_object",
        "no_such_remote",
        "not_a_task",
        "not_an_event",
        "not_completed",
        "not_committed",
        "dirty_on_pull",
        "move_inside_itself",
        "nothing_to_commit",
        "needs_an_answer",
        "needs_an_editor",
        "unresolved_conflicts",
        "missing_credential",
    ] {
        assert!(
            !matches!(Rule::named(word), Rule::Unknown(_)),
            "{word} has no variant"
        );
    }
}

#[test]
fn a_rule_this_pane_has_not_heard_of_keeps_its_word() {
    assert_eq!(
        Rule::named("invented_tomorrow"),
        Rule::Unknown("invented_tomorrow".to_string())
    );
}

#[test]
fn an_empty_stage_is_a_refusal_now_rather_than_an_ordinary_failure() {
    let failure = classify(
        Some(4),
        Some(refusal("nothing_to_commit", "nothing to commit", &[])),
        "",
    );
    assert!(leaves_model_untouched(&failure));
    assert_eq!(message(&failure), "nothing to commit");
}

#[test]
fn a_held_store_says_who_is_holding_it_and_what_to_press() {
    let failure = classify(
        Some(1),
        Some(ErrorDocument {
            kind: ErrorKind::Store,
            message: "database is locked".to_string(),
            rule: None,
            oids: Vec::new(),
        }),
        "",
    );
    assert_eq!(
        message(&failure),
        "database is locked; another dam is writing, press R to retry."
    );
    let without_document = classify(Some(1), None, "dam: database is locked\n");
    assert_eq!(
        message(&without_document),
        "database is locked; another dam is writing, press R to retry."
    );
}

/// clap answers a bad command line before `dam` runs, so there is no document to read.
#[test]
fn a_failure_whose_own_kind_is_not_the_store_takes_no_retry_advice() {
    let failure = classify(
        Some(1),
        Some(ErrorDocument {
            kind: ErrorKind::Helper,
            message: "the todoist helper says the database is locked".to_string(),
            rule: None,
            oids: Vec::new(),
        }),
        "",
    );
    assert_eq!(
        message(&failure),
        "the todoist helper says the database is locked"
    );
}

#[test]
fn a_store_failure_that_is_not_the_busy_timeout_takes_no_retry_advice() {
    let failure = classify(
        Some(1),
        Some(ErrorDocument {
            kind: ErrorKind::Store,
            message: "disk I/O error".to_string(),
            rule: None,
            oids: Vec::new(),
        }),
        "",
    );
    assert_eq!(message(&failure), "disk I/O error");
}

#[test]
fn a_command_line_dam_would_not_read_falls_back_to_its_first_line() {
    let failure = classify(
        Some(2),
        None,
        "error: unexpected argument '--nope'\n\nUsage: dam ls [QUERY]\n",
    );
    assert_eq!(message(&failure), "error: unexpected argument '--nope'");
    assert!(
        !leaves_model_untouched(&failure),
        "a bad command line is this pane's own bug, not a rule dam kept"
    );
}

#[test]
fn a_dam_too_old_to_print_a_document_still_reaches_the_status_line() {
    let failure = classify(Some(1), None, "dam: no object matches \"zzzzzzz\"\n");
    assert_eq!(message(&failure), "no object matches \"zzzzzzz\"");
}

#[test]
fn a_cancelled_run_is_never_an_error_banner() {
    assert_eq!(
        classify(Some(3), None, ""),
        Failure::Cancelled { killed: false }
    );
    assert!(leaves_model_untouched(&classify(Some(3), None, "")));
    assert_eq!(message(&classify(Some(3), None, "")), "cancelled");
    assert_eq!(
        message(&Failure::Cancelled { killed: true }),
        "cancelled (killed)"
    );
    assert!(leaves_model_untouched(&Failure::Cancelled {
        killed: false
    }));
}

#[test]
fn a_dam_that_is_not_there_says_how_to_get_one() {
    assert_eq!(
        message(&Failure::NotInstalled),
        "dam is not on PATH; install it with cargo install damnit, then press R."
    );
}

#[test]
fn output_that_will_not_parse_says_so_without_quoting_it() {
    assert_eq!(
        message(&Failure::Unreadable),
        "dam answered with something this pane could not read."
    );
}

#[test]
fn a_read_past_its_deadline_names_the_command_and_the_wait() {
    assert_eq!(
        message(&Failure::Deadline("dam ls".to_string())),
        "dam ls took longer than 30s and was cancelled."
    );
}

#[test]
fn a_signal_death_with_no_code_is_reported_rather_than_swallowed() {
    assert_eq!(
        message(&classify(None, None, "")),
        "dam was killed before it answered."
    );
}
