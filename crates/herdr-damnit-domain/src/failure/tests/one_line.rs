use super::*;

const BLOCKED_COMPLETION: &str = "e612636 cannot be completed:\n  child 0cd9bf6 is open\nuse --force to complete it anyway, or --force --interactive to decide what happens to them";

fn dam_refused(said: &str) -> Failure {
    classify(Some(4), Some(refusal(said)), "")
}

#[test]
fn a_blocked_completion_reads_as_one_line_that_keeps_its_remedy() {
    assert_eq!(
        message(&dam_refused(BLOCKED_COMPLETION)),
        "e612636 cannot be completed: child 0cd9bf6 is open; use --force to complete it anyway, or --force --interactive to decide what happens to them"
    );
}

#[test]
fn a_message_dam_already_keeps_to_one_line_passes_through_unchanged() {
    assert_eq!(
        message(&dam_refused(
            "e612636 cannot be completed: child 0cd9bf6 is open"
        )),
        "e612636 cannot be completed: child 0cd9bf6 is open"
    );
}

fn dam_failed(said: &str) -> Failure {
    classify(Some(1), Some(failed(false, said)), "")
}

#[test]
fn a_line_ending_in_a_colon_runs_into_the_next_with_one_space() {
    assert_eq!(
        message(&dam_refused("cannot pull:\nno remote")),
        "cannot pull: no remote"
    );
}

#[test]
fn lines_without_a_colon_are_joined_with_a_semicolon() {
    assert_eq!(message(&dam_refused("first\nsecond")), "first; second");
}

#[test]
fn each_line_is_trimmed_and_blank_lines_are_dropped() {
    assert_eq!(
        message(&dam_refused("  first  \n\n   \n\tsecond ")),
        "first; second"
    );
}

#[test]
fn the_store_advice_follows_the_folded_message() {
    let failure = classify(
        Some(1),
        Some(failed(true, "write failed:\n  database is locked\n")),
        "",
    );
    assert_eq!(
        message(&failure),
        "write failed: database is locked; another dam is writing, press R to retry."
    );
}

#[test]
fn a_failure_whose_message_is_only_whitespace_reads_as_one_that_said_nothing() {
    assert_eq!(message(&dam_failed(" \n\t\n ")), message(&dam_failed("")));
    assert_eq!(
        message(&dam_failed(" \n\t\n ")),
        "dam was killed before it answered."
    );
}
