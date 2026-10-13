use super::*;

const PARENT: &str = "e6126361bc4f118bb16c869005c9b0afa8232247";
const CHILD: &str = "0cd9bf6459c32dbdb237f895f8dbacef921d782b";
const BLOCKED_COMPLETION: &str = "e612636 cannot be completed:\n  child 0cd9bf6 is open\nuse --force to complete it anyway, or --force --interactive to decide what happens to them";

fn dam_refused(said: &str) -> Failure {
    classify(
        Some(4),
        Some(refusal("blocked", said, &[PARENT, CHILD])),
        "",
    )
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
