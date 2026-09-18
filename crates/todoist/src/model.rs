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
    /// The task's own long text, free-form markdown a person typed. The API sends it as a plain
    /// string, empty when the task has none.
    #[serde(default)]
    pub description: String,
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

/// The app URL of a task. The v1 task object carries no `url` field of its own: the vendor removed
/// it and documents this form instead (Migrating from v9, "Task URLs").
pub fn task_url(id: &str) -> String {
    format!("https://app.todoist.com/app/task/{id}")
}

/// A task's due date. `date` is a date or a date and time; the list shows its first ten
/// characters, which are the date either way. `is_recurring` says the task repeats, which the
/// vendor's own v1 client documents as a boolean defaulting to false.
#[derive(Debug, Deserialize)]
pub struct Due {
    pub date: String,
    #[serde(default)]
    pub is_recurring: bool,
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

/// A label, which the picker offers by name: an update writes a task's label NAMES, not ids.
/// `order` is `null`, not omitted, when the account has no explicit order for it, so the field is
/// optional rather than defaulted: a default only covers an absent key.
#[derive(Debug, Deserialize)]
pub struct Label {
    pub name: String,
    #[serde(default)]
    pub order: Option<i64>,
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

/// One comment on a task. The vendor calls this object a note: a comment's poster is `posted_uid`
/// and its attachment `file_attachment`, both of which the API sends as `null` rather than
/// omitting when there is none. The attachment is a free-form object in the schema, so it is held
/// as raw JSON and read for the two keys the pane draws.
#[derive(Debug, Deserialize)]
pub struct Comment {
    pub id: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub posted_at: Option<String>,
    #[serde(default)]
    pub posted_uid: Option<String>,
    #[serde(default)]
    pub file_attachment: Option<serde_json::Value>,
}

impl Comment {
    /// What the attachment line says: the file's name, else its type, else that there is one at
    /// all. The pane names an attachment and never fetches it: a file URL is not something a
    /// task list should be downloading on a key press.
    pub fn attachment(&self) -> Option<String> {
        let attachment = self.file_attachment.as_ref()?;
        if attachment.is_null() {
            return None;
        }
        let named = |key: &str| {
            attachment
                .get(key)
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        };
        Some(
            named("file_name")
                .or_else(|| named("file_type"))
                .unwrap_or_else(|| "file".to_string()),
        )
    }
}
