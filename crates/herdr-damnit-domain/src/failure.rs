use crate::Oid;

mod kind;
mod rule;

pub use kind::ErrorKind;
pub use rule::Rule;

pub const READ_DEADLINE_SECONDS: u64 = 30;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorDocument {
    pub kind: ErrorKind,
    pub message: String,
    pub rule: Option<Rule>,
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

const EXIT_REFUSED_BY_A_RULE: i32 = 4;
const EXIT_INTERRUPTED_OR_UNANSWERED: i32 = 3;

const SQLITE_BUSY_TIMEOUT_MESSAGES: [&str; 2] = ["database is locked", "database table is locked"];

pub fn classify(code: Option<i32>, error: Option<ErrorDocument>, fallback: &str) -> Failure {
    let said = |error: &Option<ErrorDocument>| match error {
        Some(document) => document.message.clone(),
        None => first_line(fallback),
    };
    match (code, error) {
        (Some(EXIT_REFUSED_BY_A_RULE), Some(document)) => Failure::Refused {
            rule: document
                .rule
                .unwrap_or_else(|| Rule::Unknown(String::new())),
            said: document.message,
            oids: document.oids,
        },
        (Some(EXIT_INTERRUPTED_OR_UNANSWERED), _) => Failure::Cancelled { killed: false },
        (Some(_), error) if is_the_store_held_by_another_writer(&error, fallback) => {
            Failure::Store(said(&error))
        }
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

pub fn leaves_model_and_prompt_untouched(failure: &Failure) -> bool {
    matches!(
        failure,
        Failure::Refused { .. } | Failure::Cancelled { .. } | Failure::Deadline(_)
    )
}

fn first_line(stderr: &str) -> String {
    let line = stderr.lines().next().unwrap_or_default().trim();
    line.strip_prefix("dam: ").unwrap_or(line).to_string()
}

fn is_the_store_held_by_another_writer(error: &Option<ErrorDocument>, fallback: &str) -> bool {
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
    SQLITE_BUSY_TIMEOUT_MESSAGES
        .iter()
        .any(|busy| said.contains(busy))
}

#[cfg(test)]
mod tests;
