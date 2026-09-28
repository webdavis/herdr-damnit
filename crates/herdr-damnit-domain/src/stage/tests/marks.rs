use super::*;

#[test]
fn a_conflict_outranks_staged_and_staged_outranks_working() {
    let stage = full();
    assert_eq!(stage.staged_count(), 2);
    assert_eq!(
        stage.mark_of(&Oid::new("3d4e5f6a1b2")),
        Some(Mark::Conflict)
    );
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
        "3d4e5f6a1b2",
        Op::Update,
        "the conflicted one",
        &["due"],
    ));
    assert_eq!(
        stage.mark_of(&Oid::new("3d4e5f6a1b2")),
        Some(Mark::Conflict)
    );
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
