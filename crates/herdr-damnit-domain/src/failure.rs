//! What a non-zero `dam` exit means, and the one sentence the status line carries for it. Under
//! `--json` a failure is one document on standard error, so the text and the rule are `dam`'s own.

use crate::Oid;

mod kind;
mod rule;

pub use kind::ErrorKind;
pub use rule::Rule;

/// The read deadline the pane cancels a local read at. A SQLite read that takes this long is a
/// wedged store rather than a slow one.
pub const READ_DEADLINE_SECONDS: u64 = 30;

/// `dam`'s error document, parsed by the adapters crate and handed here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorDocument {
    pub kind: ErrorKind,
    pub message: String,
    /// The rule a refusal broke, and `None` for every other kind.
    pub rule: Option<Rule>,
    /// The objects the message names, in the order it names them.
    pub oids: Vec<Oid>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    Refused {
        rule: Rule,
        said: String,
        oids: Vec<Oid>,
    },
    Store(String),
    Other(String),
    Cancelled {
        killed: bool,
    },
    Unreadable,
    NotInstalled,
    Deadline(String),
}

/// The exit code `dam` reports for a rule of its own, and for nothing else.
const REFUSED: i32 = 4;
/// The exit code `dam` reports when it was interrupted or a prompt went unanswered.
const CANCELLED: i32 = 3;

pub fn classify(code: Option<i32>, error: Option<ErrorDocument>, fallback: &str) -> Failure {
    let said = |error: &Option<ErrorDocument>| match error {
        Some(document) => document.message.clone(),
        None => first_line(fallback),
    };
    match (code, error) {
        (Some(REFUSED), Some(document)) => Failure::Refused {
            rule: document
                .rule
                .unwrap_or_else(|| Rule::Unknown(String::new())),
            said: document.message,
            oids: document.oids,
        },
        (Some(CANCELLED), _) => Failure::Cancelled { killed: false },
        (Some(_), error) if is_store(&error, fallback) => Failure::Store(said(&error)),
        (Some(_), error) => Failure::Other(said(&error)),
        (None, _) => Failure::Other(String::new()),
    }
}

pub fn message(failure: &Failure) -> String {
    match failure {
        Failure::Other(said) if said.is_empty() => "dam was killed before it answered.".to_string(),
        Failure::Refused { said, .. } | Failure::Other(said) => said.clone(),
        Failure::Store(said) => format!("{said}; another dam is writing, press R to retry."),
        Failure::Cancelled { killed: false } => "cancelled".to_string(),
        Failure::Cancelled { killed: true } => "cancelled (killed)".to_string(),
        Failure::Unreadable => "dam answered with something this pane could not read.".to_string(),
        Failure::NotInstalled => {
            "dam is not on PATH; install it with cargo install damnit, then press R.".to_string()
        }
        Failure::Deadline(command) => {
            format!("{command} took longer than {READ_DEADLINE_SECONDS}s and was cancelled.")
        }
    }
}

/// Whether the pane's model and any open prompt survive this failure untouched, which is what a
/// refused write leaves behind.
pub fn leaves_model_untouched(failure: &Failure) -> bool {
    matches!(
        failure,
        Failure::Refused { .. } | Failure::Cancelled { .. } | Failure::Deadline(_)
    )
}

fn first_line(stderr: &str) -> String {
    let line = stderr.lines().next().unwrap_or_default().trim();
    line.strip_prefix("dam: ").unwrap_or(line).to_string()
}

/// A held store is both halves `dam` reports: its own `store` kind, and SQLite's words for a
/// writer holding the store past the five-second busy timeout. A disk error under the same kind
/// takes no advice about waiting for another writer. A failure with no document is judged on its
/// line alone, which is what a `dam` too old to print one leaves behind.
fn is_store(error: &Option<ErrorDocument>, fallback: &str) -> bool {
    let said = match error {
        Some(document) => {
            if document.kind != ErrorKind::Store {
                return false;
            }
            document.message.as_str()
        }
        None => fallback,
    }
    .to_ascii_lowercase();
    said.contains("database is locked") || said.contains("database table is locked")
}

#[cfg(test)]
mod tests;
