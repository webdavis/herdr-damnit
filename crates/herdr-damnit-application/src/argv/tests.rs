use super::*;

fn oid() -> Oid {
    Oid::new("1a2b3c4")
}

fn line(argv: Vec<String>) -> String {
    argv.join(" ")
}

#[test]
fn the_reads_carry_their_query_and_the_json_flag() {
    assert_eq!(line(version()), "--version");
    assert_eq!(line(list("!done")), "ls !done --json");
    assert_eq!(line(status()), "status --json");
    assert_eq!(line(show(&oid())), "show 1a2b3c4 --json");
    assert_eq!(line(log()), "log --json");
}

#[test]
fn staging_is_one_oid_or_all_of_them() {
    assert_eq!(line(stage(&oid())), "add 1a2b3c4 --json");
    assert_eq!(line(stage_all()), "add -A --json");
    assert_eq!(line(unstage(&oid())), "reset 1a2b3c4 --json");
    assert_eq!(line(unstage_all()), "reset --json");
}

#[test]
fn a_commit_message_stays_one_argument_however_many_spaces_it_has() {
    assert_eq!(
        commit("refresh the roster row"),
        vec![
            "commit".to_string(),
            "-m".to_string(),
            "refresh the roster row".to_string(),
            "--json".to_string(),
        ]
    );
}

#[test]
fn a_push_and_a_pull_name_no_remote_because_dam_acts_on_every_one() {
    assert_eq!(line(push()), "push --json");
    assert_eq!(line(pull()), "pull --json");
}

#[test]
fn completing_is_one_flag_apart_from_forcing_it() {
    assert_eq!(line(done(&oid(), false)), "done 1a2b3c4 --json");
    assert_eq!(line(done(&oid(), true)), "done 1a2b3c4 --force --json");
}

/// The exact line the spec pins: `p` on a priority-2 task.
#[test]
fn the_priority_key_spells_dams_own_short_flag() {
    let next = Priority::new(2).expect("a priority").next();
    assert_eq!(
        set_priority(&oid(), next),
        vec![
            "edit".to_string(),
            "1a2b3c4".to_string(),
            "-p".to_string(),
            "1".to_string(),
            "--json".to_string(),
        ]
    );
}

#[test]
fn a_date_is_set_or_cleared_by_two_different_flags() {
    assert_eq!(
        line(set_due(&oid(), Some("2026-09-20"))),
        "edit 1a2b3c4 --due 2026-09-20 --json"
    );
    assert_eq!(line(set_due(&oid(), None)), "edit 1a2b3c4 --no-due --json");
    assert_eq!(
        line(set_deadline(&oid(), Some("2026-10-02"))),
        "edit 1a2b3c4 --deadline 2026-10-02 --json"
    );
    assert_eq!(
        line(set_deadline(&oid(), None)),
        "edit 1a2b3c4 --no-deadline --json"
    );
}

#[test]
fn a_label_goes_on_and_comes_off_by_name() {
    assert_eq!(
        line(label(&oid(), "home")),
        "edit 1a2b3c4 --label home --json"
    );
    assert_eq!(
        line(unlabel(&oid(), "home")),
        "edit 1a2b3c4 --unlabel home --json"
    );
}

#[test]
fn moving_and_removing_and_creating_are_their_own_verbs() {
    assert_eq!(
        line(move_to(&oid(), "home/admin/")),
        "mv 1a2b3c4 home/admin/ --json"
    );
    assert_eq!(line(remove(&oid())), "rm 1a2b3c4 --json");
    assert_eq!(
        line(create("file taxes", "home/admin/")),
        "new file taxes --path home/admin/ --json"
    );
}

#[test]
fn a_new_object_under_no_path_leaves_the_flag_off_rather_than_sending_an_empty_one() {
    assert_eq!(line(create("file taxes", "")), "new file taxes --json");
}

/// `dam edit -e --json` is refused as `needs_an_editor`, so the editor round trip is the one
/// command that must not carry the flag.
#[test]
fn the_editor_round_trip_asks_for_no_report_because_it_owns_the_terminal() {
    assert_eq!(line(edit_in_editor(&oid())), "edit 1a2b3c4 -e");
    assert!(!edit_in_editor(&oid()).contains(&JSON.to_string()));
}

/// Every other command carries `--json`, which is what makes a failure answer with the error
/// document Task 14 maps instead of a line to match substrings against.
#[test]
fn every_command_that_reports_a_failure_asks_for_the_document() {
    for argv in [
        list("!done"),
        status(),
        show(&oid()),
        log(),
        stage(&oid()),
        stage_all(),
        unstage(&oid()),
        unstage_all(),
        commit("m"),
        push(),
        pull(),
        done(&oid(), false),
        remove(&oid()),
        set_priority(&oid(), Priority::default()),
        set_due(&oid(), None),
        set_deadline(&oid(), None),
        label(&oid(), "home"),
        unlabel(&oid(), "home"),
        move_to(&oid(), "home/"),
        create("s", ""),
        resolve(&oid(), Side::Ours),
        restore(&oid()),
    ] {
        assert!(argv.contains(&JSON.to_string()), "{argv:?}");
    }
}

#[test]
fn a_conflict_is_resolved_toward_one_side_by_name() {
    assert_eq!(
        line(resolve(&oid(), Side::Ours)),
        "resolve 1a2b3c4 --ours --json"
    );
    assert_eq!(
        line(resolve(&oid(), Side::Theirs)),
        "resolve 1a2b3c4 --theirs --json"
    );
}

#[test]
fn discarding_a_working_change_is_dams_restore() {
    assert_eq!(line(restore(&oid())), "restore 1a2b3c4 --json");
}
