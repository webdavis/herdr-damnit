//! Every `dam` command the pane spawns, as the arguments that follow the configured `dam` argv.
//! One function per command, so a test compares the whole line and a flag typo fails it.

use herdr_damnit_domain::{Oid, Priority};

/// `--json` is a global flag on `dam`. It selects the report on standard output and the error
/// document on standard error, so every command whose failure the pane reports ends with it.
pub const JSON: &str = "--json";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Ours,
    Theirs,
}

pub fn version() -> Vec<String> {
    words(&["--version"])
}

pub fn list(query: &str) -> Vec<String> {
    words(&["ls", query, JSON])
}

pub fn status() -> Vec<String> {
    words(&["status", JSON])
}

pub fn show(oid: &Oid) -> Vec<String> {
    words(&["show", oid.as_str(), JSON])
}

pub fn log() -> Vec<String> {
    words(&["log", JSON])
}

pub fn stage(oid: &Oid) -> Vec<String> {
    words(&["add", oid.as_str(), JSON])
}

pub fn stage_all() -> Vec<String> {
    words(&["add", "-A", JSON])
}

pub fn unstage(oid: &Oid) -> Vec<String> {
    words(&["reset", oid.as_str(), JSON])
}

pub fn unstage_all() -> Vec<String> {
    words(&["reset", JSON])
}

pub fn commit(message: &str) -> Vec<String> {
    words(&["commit", "-m", message, JSON])
}

pub fn push() -> Vec<String> {
    words(&["push", JSON])
}

pub fn pull() -> Vec<String> {
    words(&["pull", JSON])
}

pub fn done(oid: &Oid, force: bool) -> Vec<String> {
    match force {
        true => words(&["done", oid.as_str(), "--force", JSON]),
        false => words(&["done", oid.as_str(), JSON]),
    }
}

pub fn remove(oid: &Oid) -> Vec<String> {
    words(&["rm", oid.as_str(), JSON])
}

pub fn set_priority(oid: &Oid, priority: Priority) -> Vec<String> {
    words(&[
        "edit",
        oid.as_str(),
        "-p",
        &priority.get().to_string(),
        JSON,
    ])
}

pub fn set_due(oid: &Oid, when: Option<&str>) -> Vec<String> {
    edit_date(oid, when, "--due", "--no-due")
}

pub fn set_deadline(oid: &Oid, when: Option<&str>) -> Vec<String> {
    edit_date(oid, when, "--deadline", "--no-deadline")
}

fn edit_date(oid: &Oid, when: Option<&str>, set: &str, clear: &str) -> Vec<String> {
    match when {
        Some(text) => words(&["edit", oid.as_str(), set, text, JSON]),
        None => words(&["edit", oid.as_str(), clear, JSON]),
    }
}

pub fn label(oid: &Oid, name: &str) -> Vec<String> {
    words(&["edit", oid.as_str(), "--label", name, JSON])
}

pub fn unlabel(oid: &Oid, name: &str) -> Vec<String> {
    words(&["edit", oid.as_str(), "--unlabel", name, JSON])
}

pub fn move_to(oid: &Oid, path: &str) -> Vec<String> {
    words(&["mv", oid.as_str(), path, JSON])
}

/// A new object under the cursor's path. An empty path leaves the flag off, because `dam`'s own
/// default for `--path` is the empty string.
pub fn create(subject: &str, path: &str) -> Vec<String> {
    match path.is_empty() {
        true => words(&["new", subject, JSON]),
        false => words(&["new", subject, "--path", path, JSON]),
    }
}

/// `dam edit -e` owns the terminal and prints no report the pane reads.
pub fn edit_in_editor(oid: &Oid) -> Vec<String> {
    words(&["edit", oid.as_str(), "-e"])
}

pub fn resolve(oid: &Oid, side: Side) -> Vec<String> {
    let word = match side {
        Side::Ours => "--ours",
        Side::Theirs => "--theirs",
    };
    words(&["resolve", oid.as_str(), word, JSON])
}

pub fn restore(oid: &Oid) -> Vec<String> {
    words(&["restore", oid.as_str(), JSON])
}

fn words(argv: &[&str]) -> Vec<String> {
    argv.iter().map(|word| word.to_string()).collect()
}

#[cfg(test)]
mod tests;
