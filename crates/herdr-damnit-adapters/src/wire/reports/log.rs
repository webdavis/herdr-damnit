use std::collections::HashMap;

use herdr_damnit_domain::{Date, Oid, parse_date};
use serde::Deserialize;

use super::change::WireChange;
use crate::wire::read;

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
