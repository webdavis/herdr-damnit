use super::*;

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
