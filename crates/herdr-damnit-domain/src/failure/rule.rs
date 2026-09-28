#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rule {
    Blocked,
    Cycle,
    ExclusiveLabel,
    UnknownCategory,
    NoSuchObject,
    NoWorkingObject,
    NoSuchRemote,
    NotATask,
    NotAnEvent,
    NotCompleted,
    NotCommitted,
    DirtyOnPull,
    MoveInsideItself,
    NothingToCommit,
    NeedsAnAnswer,
    NeedsAnEditor,
    UnresolvedConflicts,
    MissingCredential,
    Unknown(String),
}

impl Rule {
    pub fn named(word: &str) -> Self {
        match word {
            "blocked" => Self::Blocked,
            "cycle" => Self::Cycle,
            "exclusive_label" => Self::ExclusiveLabel,
            "unknown_category" => Self::UnknownCategory,
            "no_such_object" => Self::NoSuchObject,
            "no_working_object" => Self::NoWorkingObject,
            "no_such_remote" => Self::NoSuchRemote,
            "not_a_task" => Self::NotATask,
            "not_an_event" => Self::NotAnEvent,
            "not_completed" => Self::NotCompleted,
            "not_committed" => Self::NotCommitted,
            "dirty_on_pull" => Self::DirtyOnPull,
            "move_inside_itself" => Self::MoveInsideItself,
            "nothing_to_commit" => Self::NothingToCommit,
            "needs_an_answer" => Self::NeedsAnAnswer,
            "needs_an_editor" => Self::NeedsAnEditor,
            "unresolved_conflicts" => Self::UnresolvedConflicts,
            "missing_credential" => Self::MissingCredential,
            other => Self::Unknown(other.to_string()),
        }
    }
}
