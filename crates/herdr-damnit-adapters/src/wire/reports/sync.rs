use serde::Deserialize;

use crate::wire::read;

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
