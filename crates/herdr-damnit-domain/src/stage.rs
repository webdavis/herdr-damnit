use crate::{Mark, Oid, StagingMarks, counted};

mod rows;

pub use rows::status_rows;

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
    pub oid: Option<Oid>,
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

impl Stage {
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
        let notices = if self.notices.len() == 1 {
            "notice"
        } else {
            "notices"
        };
        counted(&[
            (self.staged.len() as u64, "staged"),
            (self.unstaged.len() as u64, "changed"),
            (self.unpushed_commits(), "unpushed"),
            (self.notices.len() as u64, notices),
        ])
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
