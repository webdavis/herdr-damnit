use herdr_damnit_domain::Oid;

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
pub enum JobKind {
    Version,
    Handshake,
    ReadList,
    ReadDone,
    ReadStatus,
    ReadShow(Oid),
    ReadLog,
    Write,
    Exclusive(SyncKind),
}

impl JobKind {
    pub(super) fn supersedes_its_own_kind(&self) -> bool {
        matches!(
            self,
            Self::ReadList | Self::ReadDone | Self::ReadStatus | Self::ReadShow(_) | Self::ReadLog
        )
    }

    pub(super) fn follow_up(&self) -> Vec<JobKind> {
        match self {
            Self::Write | Self::Exclusive(_) => vec![Self::ReadStatus, Self::ReadList],
            _ => Vec::new(),
        }
    }
}
