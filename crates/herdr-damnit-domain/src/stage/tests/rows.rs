use super::*;
use crate::{IconSet, Row, Segment, Slot};

fn rows(stage: &Stage) -> Vec<Row> {
    status_rows(stage, IconSet::Ascii)
}

fn drawn(stage: &Stage) -> Vec<String> {
    rows(stage).iter().map(Row::text).collect()
}

fn targets(stage: &Stage) -> usize {
    rows(stage).iter().filter(|row| row.oid().is_some()).count()
}

#[test]
fn the_four_sections_are_drawn_in_dams_own_order() {
    assert_eq!(
        drawn(&full()),
        vec![
            "Staged".to_string(),
            "  + new      1a2b3c4  ship the pin bump".to_string(),
            "  + changed  5d6e7f8  refresh the roster row  (due, priority)".to_string(),
            "Working".to_string(),
            "  * changed  9a0b1c2  water the plants  (subject)".to_string(),
            "Unpushed".to_string(),
            "  ^ todoist  1 commit".to_string(),
            "  ^ work  2 commits".to_string(),
            "Notices".to_string(),
            "  x 3d4e5f6  todoist  ours: \"mine\"  theirs: \"theirs\"".to_string(),
            "  ! 7a8b9c0  removed on todoist: \"old task\" is kept here".to_string(),
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
    assert_eq!(
        rows(&stage),
        vec![Row::Line {
            oid: None,
            segments: vec![Segment {
                text: "nothing staged, nothing changed".to_string(),
                slot: Slot::Text,
            }],
        }]
    );
}

#[test]
fn every_row_naming_an_oid_is_a_cursor_target() {
    assert_eq!(
        targets(&full()),
        5,
        "staged two, working one, conflict one, notice one"
    );
}

#[test]
fn an_unpushed_row_and_a_notice_about_no_object_take_no_cursor() {
    let mut stage = full();
    stage.notices.push(Notice {
        oid: None,
        message: "pull failed on todoist: the service did not answer".to_string(),
    });

    let untargeted: Vec<String> = rows(&stage)
        .into_iter()
        .filter(|row| matches!(row, Row::Line { oid: None, .. }))
        .map(|row| row.text())
        .collect();

    assert_eq!(
        untargeted,
        vec![
            "  ^ todoist  1 commit",
            "  ^ work  2 commits",
            "  pull failed on todoist: the service did not answer",
        ]
    );
}

#[test]
fn a_notice_about_no_object_in_particular_draws_its_message_alone() {
    let stage = Stage {
        notices: vec![Notice {
            oid: None,
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
        targets(&stage),
        4,
        "a notice naming no object is not a cursor target"
    );
}

#[test]
fn a_notice_that_names_an_object_carries_its_mark_in_the_notice_colour() {
    let marked = rows(&full())
        .into_iter()
        .find(|row| row.oid().is_some_and(|oid| oid.as_str() == "7a8b9c0d1e2"));
    let Some(Row::Line { segments, .. }) = marked else {
        panic!("the notice row is not a cursor target");
    };
    assert_eq!(
        segments[1],
        Segment {
            text: "! ".to_string(),
            slot: Slot::Orange,
        }
    );
}

#[test]
fn a_conflict_value_is_quoted_on_one_row_without_leaking_rust_escaping() {
    let conflict = |ours: &str, theirs: &str| Stage {
        conflicts: vec![Conflict {
            oid: Oid::new("3d4e5f6"),
            remote: "todoist".to_string(),
            ours: ours.to_string(),
            theirs: theirs.to_string(),
        }],
        ..full()
    };

    let quoted = drawn(&conflict("say \"hi\"", "a\\b"));
    let spanning = drawn(&conflict("line\nbreak", "carriage\rreturn"));

    assert!(
        quoted
            .contains(&"  x 3d4e5f6  todoist  ours: \"say \"hi\"\"  theirs: \"a\\b\"".to_string()),
        "{quoted:?}"
    );
    assert!(
        spanning.contains(
            &"  x 3d4e5f6  todoist  ours: \"line break\"  theirs: \"carriage return\"".to_string()
        ),
        "{spanning:?}"
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
    assert!(drawn(&one).contains(&"  ^ todoist  1 commit".to_string()));
    assert!(drawn(&two).contains(&"  ^ todoist  2 commits".to_string()));
}
