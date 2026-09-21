//! The one document `dam` prints on standard error rather than standard output.
//!
//! Under `--json` a failure is exactly one error document there and nothing else, so the parse is
//! of the whole stream. Anything else is no document at all: clap's usage text at exit 2, and a
//! `dam` too old to print one.

use herdr_damnit_domain::{ErrorDocument, ErrorKind, Oid, Rule};
use serde::Deserialize;

#[derive(Deserialize)]
struct WireEnvelope {
    error: WireError,
}

#[derive(Deserialize)]
struct WireError {
    kind: String,
    #[serde(default)]
    rule: Option<String>,
    message: String,
    #[serde(default)]
    oids: Vec<String>,
}

/// `dam`'s error document, or `None` for standard error that does not hold one.
pub fn error_document(stderr: &str) -> Option<ErrorDocument> {
    let envelope: WireEnvelope = serde_json::from_str(stderr.trim()).ok()?;
    Some(ErrorDocument {
        kind: ErrorKind::named(&envelope.error.kind),
        message: envelope.error.message,
        rule: envelope.error.rule.as_deref().map(Rule::named),
        oids: envelope.error.oids.into_iter().map(Oid::new).collect(),
    })
}

#[cfg(test)]
mod tests;
