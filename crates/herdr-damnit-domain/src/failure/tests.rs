mod one_line;

use super::*;

fn refusal(message: &str) -> ErrorDocument {
    failed(false, message)
}

fn failed(store: bool, message: &str) -> ErrorDocument {
    ErrorDocument {
        store,
        message: message.to_string(),
    }
}

#[test]
fn a_refusal_carries_dams_own_sentence() {
    let failure = classify(
        Some(4),
        Some(refusal(
            "98d8780 cannot be completed: child a9db854 is open",
        )),
        "",
    );
    assert_eq!(
        failure,
        Failure::Refused("98d8780 cannot be completed: child a9db854 is open".to_string())
    );
    assert_eq!(
        message(&failure),
        "98d8780 cannot be completed: child a9db854 is open"
    );
}

#[test]
fn an_empty_stage_is_a_refusal_now_rather_than_an_ordinary_failure() {
    let failure = classify(Some(4), Some(refusal("nothing to commit")), "");
    assert_eq!(failure, Failure::Refused("nothing to commit".to_string()));
    assert_eq!(message(&failure), "nothing to commit");
}

#[test]
fn a_held_store_says_who_is_holding_it_and_what_to_press() {
    let failure = classify(Some(1), Some(failed(true, "database is locked")), "");
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

#[test]
fn a_failure_whose_own_kind_is_not_the_store_takes_no_retry_advice_even_naming_a_lock() {
    let failure = classify(
        Some(1),
        Some(failed(
            false,
            "the todoist helper says the database is locked",
        )),
        "",
    );
    assert_eq!(
        message(&failure),
        "the todoist helper says the database is locked"
    );
}

#[test]
fn a_store_failure_that_is_not_the_busy_timeout_takes_no_retry_advice() {
    let failure = classify(Some(1), Some(failed(true, "disk I/O error")), "");
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
        matches!(failure, Failure::Other(_)),
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
    assert_eq!(classify(Some(3), None, ""), Failure::Cancelled);
    assert_eq!(message(&classify(Some(3), None, "")), "cancelled");
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
fn a_signal_death_with_no_code_is_reported_rather_than_swallowed() {
    assert_eq!(
        message(&classify(None, None, "")),
        "dam was killed before it answered."
    );
}
