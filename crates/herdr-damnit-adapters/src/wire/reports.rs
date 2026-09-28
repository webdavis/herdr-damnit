use std::collections::HashMap;

use herdr_damnit_domain::{Change, Conflict, Date, Notice, Oid, Op, Stage, Unpushed, parse_date};
use serde::Deserialize;

use super::{WireObject, read};

#[derive(Deserialize)]
struct WireStatus {
    staged: Vec<WireChange>,
    unstaged: Vec<WireChange>,
    conflicts: Vec<WireConflict>,
    notices: Vec<serde_json::Value>,
    unpushed: Vec<WireUnpushed>,
}

#[derive(Deserialize)]
struct WireChange {
    oid: String,
    op: String,
    #[serde(default)]
    subject: String,
    #[serde(default)]
    fields: Vec<String>,
    #[serde(default, rename = "done")]
    leaves_the_object_done: bool,
    #[serde(default)]
    completed_at: Option<String>,
    #[serde(default)]
    after: Option<WireObject>,
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

fn into_change(wire: WireChange) -> Change {
    let subject = match wire.subject.is_empty() {
        false => wire.subject,
        true => wire
            .after
            .as_ref()
            .map(|object| object.subject.clone())
            .unwrap_or_default(),
    };
    Change {
        oid: Oid::new(wire.oid),
        op: match wire.op.as_str() {
            "create" => Op::Create,
            "delete" => Op::Delete,
            _ => Op::Update,
        },
        subject,
        fields: wire.fields,
    }
}

fn into_notice(wire: &serde_json::Value) -> Notice {
    let text = |key: &str| wire.get(key).and_then(|value| value.as_str()).unwrap_or("");
    let kind = text("kind").to_string();
    let remote = text("remote");
    let subject = text("subject");
    let why = text("why");
    let message = match kind.as_str() {
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
        kind,
        oid: wire
            .get("oid")
            .and_then(|value| value.as_str())
            .map(Oid::new),
        remote: (!remote.is_empty()).then(|| remote.to_string()),
        message,
    }
}

#[derive(Deserialize)]
struct WireLog {
    commits: Vec<WireCommit>,
}

#[derive(Deserialize)]
struct WireCommit {
    at: String,
    changes: Vec<WireChange>,
}

pub fn completions(json: &str) -> Result<HashMap<Oid, Date>, String> {
    let log: WireLog = read(json)?;
    let mut completed = HashMap::new();
    for commit in log.commits {
        for change in commit.changes {
            if !completes_the_object(&change) {
                continue;
            }
            let at = change
                .completed_at
                .as_deref()
                .and_then(parse_date)
                .or_else(|| parse_date(&commit.at));
            if let Some(at) = at {
                completed.insert(Oid::new(change.oid), at);
            }
        }
    }
    Ok(completed)
}

fn completes_the_object(change: &WireChange) -> bool {
    change.leaves_the_object_done && change.fields.iter().any(|field| field == "done")
}

#[derive(Deserialize)]
struct WireSync<T> {
    remotes: Vec<T>,
}

#[derive(Deserialize)]
struct WirePush {
    remote: String,
    sent: u64,
    succeeded: u64,
    skipped: u64,
    failed: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct WirePull {
    remote: String,
    created: u64,
    updated: u64,
    unchanged: u64,
    conflicts: u64,
    removed_upstream: u64,
}

pub fn push_summary(json: &str) -> Result<String, String> {
    let report: WireSync<WirePush> = read(json)?;
    Ok(report
        .remotes
        .iter()
        .map(|remote| {
            format!(
                "{}: {} sent, {} ok, {} failed, {} skipped",
                remote.remote,
                remote.sent,
                remote.succeeded,
                remote.failed.len(),
                remote.skipped
            )
        })
        .collect::<Vec<_>>()
        .join("; "))
}

pub fn pull_summary(json: &str) -> Result<String, String> {
    let report: WireSync<WirePull> = read(json)?;
    Ok(report
        .remotes
        .iter()
        .map(|remote| {
            format!(
                "{}: {} new, {} updated, {} unchanged, {} conflict(s), {} removed upstream",
                remote.remote,
                remote.created,
                remote.updated,
                remote.unchanged,
                remote.conflicts,
                remote.removed_upstream
            )
        })
        .collect::<Vec<_>>()
        .join("; "))
}

#[cfg(test)]
mod tests;
