use crate::{Mark, Oid, StagingMarks};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    Create,
    Update,
    Delete,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub oid: Oid,
    pub op: Op,
    pub subject: String,
    pub fields: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unpushed {
    pub remote: String,
    pub commits: u64,
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stage {
    pub staged: Vec<Change>,
    pub unstaged: Vec<Change>,
    pub unpushed: Vec<Unpushed>,
    pub conflicts: Vec<Conflict>,
    pub notices: Vec<Notice>,
}

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
                        newlines_as_spaces(&conflict.ours),
                        newlines_as_spaces(&conflict.theirs)
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

fn newlines_as_spaces(value: &str) -> String {
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
