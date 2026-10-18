mod marks;
mod rows;
mod summary;

use super::*;

fn change(oid: &str, op: Op, subject: &str, fields: &[&str]) -> Change {
    Change {
        oid: Oid::new(oid),
        op,
        subject: subject.to_string(),
        fields: fields.iter().map(|field| field.to_string()).collect(),
    }
}

pub(super) fn full() -> Stage {
    Stage {
        staged: vec![
            change("1a2b3c4", Op::Create, "ship the pin bump", &[]),
            change(
                "5d6e7f8a9b0",
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
            oid: Oid::new("3d4e5f6a1b2"),
            remote: "todoist".to_string(),
            ours: "mine".to_string(),
            theirs: "theirs".to_string(),
        }],
        notices: vec![Notice {
            oid: Some(Oid::new("7a8b9c0d1e2")),
            message: "removed on todoist: \"old task\" is kept here".to_string(),
        }],
    }
}
