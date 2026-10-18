use herdr_damnit_domain::{Conflict, Notice, Oid, Stage, Unpushed};
use serde::Deserialize;

use super::change::{WireChange, into_change};
use crate::wire::{WireObject, read};

#[derive(Deserialize)]
struct WireStatus {
    staged: Vec<WireChange>,
    unstaged: Vec<WireChange>,
    conflicts: Vec<WireConflict>,
    notices: Vec<serde_json::Value>,
    unpushed: Vec<WireUnpushed>,
}

#[derive(Deserialize)]
struct WireConflict {
    oid: String,
    remote: String,
    ours: WireObject,
    theirs: WireObject,
}

#[derive(Deserialize)]
struct WireUnpushed {
    remote: String,
    commits: u64,
    #[serde(default)]
    oids: Vec<String>,
}

pub fn stage(json: &str) -> Result<Stage, String> {
    let status: WireStatus = read(json)?;
    Ok(Stage {
        staged: status.staged.into_iter().map(into_change).collect(),
        unstaged: status.unstaged.into_iter().map(into_change).collect(),
        unpushed: status
            .unpushed
            .into_iter()
            .map(|remote| Unpushed {
                remote: remote.remote,
                commits: remote.commits,
                oids: remote.oids.into_iter().map(Oid::new).collect(),
            })
            .collect(),
        conflicts: status
            .conflicts
            .into_iter()
            .map(|conflict| Conflict {
                oid: Oid::new(conflict.oid),
                remote: conflict.remote,
                ours: conflict.ours.subject,
                theirs: conflict.theirs.subject,
            })
            .collect(),
        notices: status.notices.iter().map(into_notice).collect(),
    })
}

fn into_notice(wire: &serde_json::Value) -> Notice {
    let text = |key: &str| wire.get(key).and_then(|value| value.as_str()).unwrap_or("");
    let kind = text("kind");
    let remote = text("remote");
    let subject = text("subject");
    let why = text("why");
    let message = match kind {
        "removed_upstream" => format!("removed on {remote}: {subject:?} is kept here"),
        "event_cancelled" => format!("event cancelled: {subject:?}"),
        "push_failed" => format!("{remote}: push failed: {why}"),
        "pull_failed" => format!("{remote}: pull failed: {why}"),
        "kind_changed" => format!(
            "kind changed: a {} here and an {} upstream",
            text("ours"),
            text("theirs")
        ),
        other => format!("{other}: {why}"),
    };
    Notice {
        oid: wire
            .get("oid")
            .and_then(|value| value.as_str())
            .map(Oid::new),
        message,
    }
}
