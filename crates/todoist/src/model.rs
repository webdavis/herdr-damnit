//! The objects the pane reads. Every field the list view shows is named here; anything else the
//! API sends is ignored, so a new field upstream is not a parse failure.

use serde::Deserialize;

/// One page of a list endpoint. `next_cursor` is `None` on the last page.
#[derive(Debug, Deserialize)]
pub(crate) struct Page<T> {
    pub results: Vec<T>,
    pub next_cursor: Option<String>,
}

/// One page of a completed-task endpoint, which names its rows `items` rather than `results`.
#[derive(Debug, Deserialize)]
pub(crate) struct Items<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

/// An open task. `parent_id` is how the API expresses a subtask: it holds the parent task's id,
/// and is absent on a top-level task.
#[derive(Debug, Deserialize)]
pub struct Task {
    pub id: String,
    pub content: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub section_id: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    /// 1 is the lowest and 4 the highest, the inverse of the p1 to p4 the app shows.
    #[serde(default = "lowest_priority")]
    pub priority: u8,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub due: Option<Due>,
    /// Position among its siblings. The ordering key the API spells `child_order`.
    #[serde(default, alias = "child_order")]
    pub order: i64,
}

fn lowest_priority() -> u8 {
    1
}

/// A task's due date. `date` is a date or a date and time; the list shows its first ten
/// characters, which are the date either way.
#[derive(Debug, Deserialize)]
pub struct Due {
    pub date: String,
}

#[derive(Debug, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    #[serde(default, alias = "child_order")]
    pub order: i64,
}

#[derive(Debug, Deserialize)]
pub struct Section {
    pub id: String,
    pub name: String,
    pub project_id: String,
    #[serde(default, alias = "section_order")]
    pub order: i64,
}

/// A completed task. The vendor schema sends `completed_at` as `null` for an active (not yet
/// completed) task rather than omitting it, so the field is optional, not defaulted: a default
/// only covers an absent key and a null still fails to parse as a plain `String`.
#[derive(Debug, Deserialize)]
pub struct CompletedTask {
    pub id: String,
    pub content: String,
    #[serde(default)]
    pub completed_at: Option<String>,
}
