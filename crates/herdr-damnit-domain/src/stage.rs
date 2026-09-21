//! The staging model `dam status` reports, and the Status screen drawn from it. A section with
//! nothing in it is left out, and an entirely empty stage says so in `dam`'s own words.

use crate::{Mark, Oid, StagingMarks};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    Create,
    Update,
    Delete,
}

/// One object `dam` reports as changed, and the field names the change touches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub oid: Oid,
    pub op: Op,
    pub subject: String,
    pub fields: Vec<String>,
}

/// How far one remote is behind the local commits, and which objects those commits touch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unpushed {
    pub remote: String,
    pub commits: u64,
    /// The objects whose changes sit in this remote's unpushed commits. Empty when the `dam` that
    /// answered does not publish them, which costs the rows their unpushed mark and nothing else.
    pub oids: Vec<Oid>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict {
    pub oid: Oid,
    pub remote: String,
    pub ours: String,
    pub theirs: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub kind: String,
    pub oid: Option<Oid>,
    pub remote: Option<String>,
    pub message: String,
}

/// The five arrays `dam status --json` answers with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stage {
    pub staged: Vec<Change>,
    pub unstaged: Vec<Change>,
    pub unpushed: Vec<Unpushed>,
    pub conflicts: Vec<Conflict>,
    pub notices: Vec<Notice>,
}

/// One line of the Status screen. A `Change` row names an oid and is therefore a cursor target; a
/// `Line` names none, and carries a mark only where the screen draws one beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatusRow {
    Heading(String),
    Change { oid: Oid, mark: Mark, text: String },
    Line { mark: Option<Mark>, text: String },
}

impl Stage {
    pub fn is_clean(&self) -> bool {
        self.staged.is_empty()
            && self.unstaged.is_empty()
            && self.unpushed.iter().all(|remote| remote.commits == 0)
            && self.conflicts.is_empty()
            && self.notices.is_empty()
    }

    pub fn is_staged(&self, oid: &Oid) -> bool {
        self.staged.iter().any(|change| &change.oid == oid)
    }

    pub fn staged_count(&self) -> usize {
        self.staged.len()
    }

    /// Whether an object's changes sit in some remote's unpushed commits.
    pub fn is_unpushed(&self, oid: &Oid) -> bool {
        self.unpushed
            .iter()
            .any(|remote| remote.oids.iter().any(|unpushed| unpushed == oid))
    }

    pub fn unpushed_commits(&self) -> u64 {
        self.unpushed.iter().map(|remote| remote.commits).sum()
    }

    pub fn summary(&self) -> String {
        format!(
            "{} staged  {} changed  {} unpushed  {} notice{}",
            self.staged.len(),
            self.unstaged.len(),
            self.unpushed_commits(),
            self.notices.len(),
            if self.notices.len() == 1 { "" } else { "s" }
        )
    }

    pub fn rows(&self) -> Vec<StatusRow> {
        let mut rows = Vec::new();
        section(&mut rows, "Staged", &self.staged, Mark::Staged);
        section(&mut rows, "Working", &self.unstaged, Mark::Working);
        if self.unpushed.iter().any(|remote| remote.commits > 0) {
            rows.push(StatusRow::Heading("Unpushed".to_string()));
            for remote in self.unpushed.iter().filter(|remote| remote.commits > 0) {
                rows.push(StatusRow::Line {
                    mark: Some(Mark::Unpushed),
                    text: format!(
                        "  {}  {} commit{}",
                        remote.remote,
                        remote.commits,
                        if remote.commits == 1 { "" } else { "s" }
                    ),
                });
            }
        }
        if !self.conflicts.is_empty() || !self.notices.is_empty() {
            rows.push(StatusRow::Heading("Notices".to_string()));
            for conflict in &self.conflicts {
                rows.push(StatusRow::Change {
                    oid: conflict.oid.clone(),
                    mark: Mark::Conflict,
                    text: format!(
                        "  {}  {}  ours: \"{}\"  theirs: \"{}\"",
                        conflict.oid.short(),
                        conflict.remote,
                        one_line(&conflict.ours),
                        one_line(&conflict.theirs)
                    ),
                });
            }
            rows.extend(self.notices.iter().map(Notice::row));
        }
        if rows.is_empty() {
            rows.push(StatusRow::Line {
                mark: None,
                text: "nothing staged, nothing changed".to_string(),
            });
        }
        rows
    }
}

/// A conflicting value as one row can carry it. The quotes are the spec's; a newline becomes a
/// space so a multi-line value cannot break the row it is drawn in.
fn one_line(value: &str) -> String {
    value.replace(['\n', '\r'], " ")
}

fn section(rows: &mut Vec<StatusRow>, heading: &str, changes: &[Change], mark: Mark) {
    if changes.is_empty() {
        return;
    }
    rows.push(StatusRow::Heading(heading.to_string()));
    rows.extend(changes.iter().map(|change| StatusRow::Change {
        oid: change.oid.clone(),
        mark,
        text: change.line(),
    }));
}

impl Change {
    /// One change as the Status screen draws it, in `dam`'s own column shape.
    fn line(&self) -> String {
        let word = match self.op {
            Op::Create => "new     ",
            Op::Update => "changed ",
            Op::Delete => "removed ",
        };
        let fields = if self.fields.is_empty() {
            String::new()
        } else {
            format!("  ({})", self.fields.join(", "))
        };
        format!("  {word} {}  {}{fields}", self.oid.short(), self.subject)
    }
}

impl Notice {
    /// A notice that names an object is a cursor target, the way a conflict is. One about a remote
    /// rather than an object, such as a failed pull, names none and draws its message alone.
    fn row(&self) -> StatusRow {
        match &self.oid {
            Some(oid) => StatusRow::Change {
                oid: oid.clone(),
                mark: Mark::Notice,
                text: format!("  {}  {}", oid.short(), self.message),
            },
            None => StatusRow::Line {
                mark: None,
                text: format!("  {}", self.message),
            },
        }
    }
}

impl StagingMarks for Stage {
    /// Most urgent first: a conflict beats staged, staged beats working, and working beats
    /// unpushed.
    fn mark_of(&self, oid: &Oid) -> Option<Mark> {
        if self.conflicts.iter().any(|conflict| &conflict.oid == oid) {
            return Some(Mark::Conflict);
        }
        if self.is_staged(oid) {
            return Some(Mark::Staged);
        }
        if self.unstaged.iter().any(|change| &change.oid == oid) {
            return Some(Mark::Working);
        }
        if self.is_unpushed(oid) {
            return Some(Mark::Unpushed);
        }
        None
    }
}

#[cfg(test)]
mod tests;
