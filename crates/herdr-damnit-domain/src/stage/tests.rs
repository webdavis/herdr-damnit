use super::*;

fn change(oid: &str, op: Op, subject: &str, fields: &[&str]) -> Change {
    Change {
        oid: Oid::new(oid),
        op,
        subject: subject.to_string(),
        fields: fields.iter().map(|field| field.to_string()).collect(),
    }
}

fn full() -> Stage {
    Stage {
        staged: vec![
            change("1a2b3c4", Op::Create, "ship the pin bump", &[]),
            change(
                "5d6e7f8",
                Op::Update,
                "refresh the roster row",
                &["due", "priority"],
            ),
        ],
        unstaged: vec![change(
            "9a0b1c2",
            Op::Update,
            "water the plants",
            &["subject"],
        )],
        unpushed: vec![
            Unpushed {
                remote: "todoist".to_string(),
                commits: 1,
                oids: Vec::new(),
            },
            Unpushed {
                remote: "work".to_string(),
                commits: 2,
                oids: vec![Oid::new("c3d4e5f")],
            },
        ],
        conflicts: vec![Conflict {
            oid: Oid::new("3d4e5f6"),
            remote: "todoist".to_string(),
            ours: "mine".to_string(),
            theirs: "theirs".to_string(),
        }],
        notices: vec![Notice {
            kind: "removed_upstream".to_string(),
            oid: Some(Oid::new("7a8b9c0")),
            remote: Some("todoist".to_string()),
            message: "removed on todoist: \"old task\" is kept here".to_string(),
        }],
    }
}

fn drawn(stage: &Stage) -> Vec<String> {
    stage
        .rows()
        .iter()
        .map(|row| match row {
            StatusRow::Heading(text) | StatusRow::Line(text) => text.clone(),
            StatusRow::Change { text, .. } => text.clone(),
        })
        .collect()
}

#[test]
fn the_four_sections_are_drawn_in_dams_own_order() {
    assert_eq!(
        drawn(&full()),
        vec![
            "Staged".to_string(),
            "  new      1a2b3c4  ship the pin bump".to_string(),
            "  changed  5d6e7f8  refresh the roster row  (due, priority)".to_string(),
            "Working".to_string(),
            "  changed  9a0b1c2  water the plants  (subject)".to_string(),
            "Unpushed".to_string(),
            "  todoist  1 commit".to_string(),
            "  work  2 commits".to_string(),
            "Notices".to_string(),
            "  3d4e5f6  todoist  ours: \"mine\"  theirs: \"theirs\"".to_string(),
            "  7a8b9c0  removed on todoist: \"old task\" is kept here".to_string(),
        ]
    );
}

#[test]
fn an_empty_section_is_left_out_entirely() {
    let stage = Stage {
        unstaged: Vec::new(),
        unpushed: Vec::new(),
        conflicts: Vec::new(),
        notices: Vec::new(),
        ..full()
    };
    assert!(
        !stage.is_clean(),
        "a stage with staged changes is not clean"
    );
    let drawn = drawn(&stage);
    assert_eq!(drawn.first().map(String::as_str), Some("Staged"));
    assert!(!drawn.iter().any(|line| line == "Working"), "{drawn:?}");
    assert!(!drawn.iter().any(|line| line == "Notices"), "{drawn:?}");
}

#[test]
fn an_empty_stage_says_so_in_dams_own_words() {
    let stage = Stage {
        staged: Vec::new(),
        unstaged: Vec::new(),
        unpushed: Vec::new(),
        conflicts: Vec::new(),
        notices: Vec::new(),
    };
    assert!(stage.is_clean());
    assert!(!full().is_clean());
    assert_eq!(drawn(&stage), vec!["nothing staged, nothing changed"]);
}

#[test]
fn a_conflict_outranks_staged_and_staged_outranks_working() {
    let stage = full();
    assert_eq!(stage.staged_count(), 2);
    assert_eq!(stage.mark_of(&Oid::new("3d4e5f6")), Some(Mark::Conflict));
    assert_eq!(stage.mark_of(&Oid::new("1a2b3c4")), Some(Mark::Staged));
    assert_eq!(stage.mark_of(&Oid::new("9a0b1c2")), Some(Mark::Working));
    assert_eq!(stage.mark_of(&Oid::new("nothing")), None);
}

#[test]
fn an_object_only_in_an_unpushed_commit_carries_the_unpushed_mark() {
    assert_eq!(full().mark_of(&Oid::new("c3d4e5f")), Some(Mark::Unpushed));
}

#[test]
fn a_working_change_outranks_an_unpushed_commit() {
    let mut stage = full();
    stage.unpushed[1].oids.push(Oid::new("9a0b1c2"));
    assert_eq!(stage.mark_of(&Oid::new("9a0b1c2")), Some(Mark::Working));
}

#[test]
fn a_dam_that_sends_no_oids_leaves_the_unpushed_set_empty() {
    let stage = Stage {
        unpushed: vec![Unpushed {
            remote: "work".to_string(),
            commits: 2,
            oids: Vec::new(),
        }],
        ..full()
    };
    assert_eq!(stage.mark_of(&Oid::new("c3d4e5f")), None);
}

#[test]
fn an_object_both_staged_and_in_conflict_shows_the_conflict_mark() {
    let mut stage = full();
    stage.staged.push(change(
        "3d4e5f6",
        Op::Update,
        "the conflicted one",
        &["due"],
    ));
    assert_eq!(stage.mark_of(&Oid::new("3d4e5f6")), Some(Mark::Conflict));
}

#[test]
fn an_object_both_staged_and_changed_again_shows_the_staged_mark() {
    let mut stage = full();
    stage.unstaged.push(change(
        "1a2b3c4",
        Op::Update,
        "ship the pin bump",
        &["body"],
    ));
    assert_eq!(stage.mark_of(&Oid::new("1a2b3c4")), Some(Mark::Staged));
}

#[test]
fn every_row_naming_an_oid_is_a_cursor_target() {
    let targets = full()
        .rows()
        .into_iter()
        .filter(|row| matches!(row, StatusRow::Change { .. }))
        .count();
    assert_eq!(
        targets, 5,
        "staged two, working one, conflict one, notice one"
    );
}

#[test]
fn a_notice_about_no_object_in_particular_draws_its_message_alone() {
    let stage = Stage {
        notices: vec![Notice {
            kind: "pull_failed".to_string(),
            oid: None,
            remote: Some("todoist".to_string()),
            message: "pull failed on todoist: the service did not answer".to_string(),
        }],
        ..full()
    };

    assert!(
        drawn(&stage).contains(&"  pull failed on todoist: the service did not answer".to_string()),
        "{:?}",
        drawn(&stage)
    );
    assert_eq!(
        stage
            .rows()
            .into_iter()
            .filter(|row| matches!(row, StatusRow::Change { .. }))
            .count(),
        4,
        "a notice naming no object is not a cursor target"
    );
}

#[test]
fn a_notice_that_names_an_object_carries_its_mark() {
    let marked = full()
        .rows()
        .into_iter()
        .find(|row| matches!(row, StatusRow::Change { oid, .. } if oid.as_str() == "7a8b9c0"));
    let Some(StatusRow::Change { mark, .. }) = marked else {
        panic!("the notice row is not a cursor target");
    };
    assert_eq!(mark, Mark::Notice);
}

#[test]
fn a_conflict_value_is_quoted_without_leaking_rust_escaping() {
    let stage = Stage {
        conflicts: vec![Conflict {
            oid: Oid::new("3d4e5f6"),
            remote: "todoist".to_string(),
            ours: "say \"hi\"".to_string(),
            theirs: "a\\b".to_string(),
        }],
        ..full()
    };

    assert!(
        drawn(&stage)
            .contains(&"  3d4e5f6  todoist  ours: \"say \"hi\"\"  theirs: \"a\\b\"".to_string()),
        "{:?}",
        drawn(&stage)
    );
}

#[test]
fn a_conflict_value_that_spans_lines_still_draws_as_one_row() {
    let stage = Stage {
        conflicts: vec![Conflict {
            oid: Oid::new("3d4e5f6"),
            remote: "todoist".to_string(),
            ours: "line\nbreak".to_string(),
            theirs: "plain".to_string(),
        }],
        ..full()
    };

    assert!(
        drawn(&stage).iter().all(|line| !line.contains('\n')),
        "{:?}",
        drawn(&stage)
    );
    assert!(
        drawn(&stage)
            .contains(&"  3d4e5f6  todoist  ours: \"line break\"  theirs: \"plain\"".to_string()),
        "{:?}",
        drawn(&stage)
    );
}

#[test]
fn the_summary_counts_what_the_status_line_carries() {
    assert_eq!(
        full().summary(),
        "2 staged  1 changed  3 unpushed  1 notice"
    );
    assert_eq!(
        full().unpushed_commits(),
        3,
        "two remotes, one and two commits"
    );
}

#[test]
fn one_commit_and_two_commits_are_both_spelled_correctly() {
    let one = Stage {
        unpushed: vec![Unpushed {
            remote: "todoist".to_string(),
            commits: 1,
            oids: Vec::new(),
        }],
        ..full()
    };
    let two = Stage {
        unpushed: vec![Unpushed {
            remote: "todoist".to_string(),
            commits: 2,
            oids: Vec::new(),
        }],
        ..full()
    };
    assert!(drawn(&one).contains(&"  todoist  1 commit".to_string()));
    assert!(drawn(&two).contains(&"  todoist  2 commits".to_string()));
}
