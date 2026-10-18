use herdr_damnit_domain::{DONE_QUERY, Oid};

use crate::argv;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncKind {
    Commit,
    Push,
    Pull,
}

impl SyncKind {
    pub fn verb(self) -> &'static str {
        match self {
            Self::Commit => "commit",
            Self::Push => "push",
            Self::Pull => "pull",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    Version,
    Status,
    List(String),
    Done,
    Show(Oid),
    Log,
    Write(Vec<String>),
    Sync(SyncKind, Vec<String>),
}

impl Request {
    pub fn argv(&self) -> Vec<String> {
        match self {
            Self::Version => argv::version(),
            Self::Status => argv::status(),
            Self::List(query) => argv::list(query),
            Self::Done => argv::list(DONE_QUERY),
            Self::Show(oid) => argv::show(oid),
            Self::Log => argv::log(),
            Self::Write(argv) | Self::Sync(_, argv) => argv.clone(),
        }
    }

    pub fn rereads_after(&self) -> bool {
        matches!(self, Self::Write(_) | Self::Sync(..))
    }

    pub(super) fn exclusive(&self) -> Option<SyncKind> {
        match self {
            Self::Sync(kind, _) => Some(*kind),
            _ => None,
        }
    }

    pub(super) fn supersedes(&self, older: &Self) -> bool {
        self.is_read() && std::mem::discriminant(self) == std::mem::discriminant(older)
    }

    fn is_read(&self) -> bool {
        matches!(
            self,
            Self::Status | Self::List(_) | Self::Done | Self::Show(_) | Self::Log
        )
    }
}

#[cfg(test)]
mod tests;
