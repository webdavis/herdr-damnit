//! The pane's local copy of a view, so it opens on the tasks it last saw rather than on an empty
//! list waiting for the network.
//!
//! One file per view under the plugin's own state directory, holding the API's own documents
//! rather than drawn rows: the due marks are read against the day the pane opens, not the day the
//! cache was written. A file this version cannot read is no cache at all, so a truncated or
//! older file leaves the pane opening empty instead of refusing to open.

#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use todoist::{Project, Section, Task};

/// The cache format. A file written by any other version is ignored.
const VERSION: u32 = 1;

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;

/// What one view's file holds.
#[derive(Serialize, Deserialize)]
struct Cached {
    version: u32,
    /// Unix seconds at which the read that filled this file succeeded.
    saved_at: u64,
    tasks: Vec<Task>,
    projects: Vec<Project>,
    sections: Vec<Section>,
}

/// The three lists a view is drawn from.
pub struct Snapshot {
    pub tasks: Vec<Task>,
    pub projects: Vec<Project>,
    pub sections: Vec<Section>,
}

/// The cache directory, and when the tasks now on screen were read from the API. That time is
/// what the stale mark counts from: it is set by a load and by every successful save, so it names
/// the freshest data the pane holds either way.
pub struct Cache {
    dir: PathBuf,
    fetched_at: Option<u64>,
}

impl Cache {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            fetched_at: None,
        }
    }

    /// The cache under the plugin's own state directory, beside the pane and view-request files.
    pub fn in_state_dir() -> Self {
        Self::new(crate::state::state_dir().join("cache"))
    }

    /// The view's cached lists, or `None` when there is no readable file for it.
    pub fn load(&mut self, view: &str) -> Option<Snapshot> {
        let text = std::fs::read_to_string(self.path(view)).ok()?;
        let cached: Cached = serde_json::from_str(&text).ok()?;
        if cached.version != VERSION {
            return None;
        }
        self.fetched_at = Some(cached.saved_at);
        Some(Snapshot {
            tasks: cached.tasks,
            projects: cached.projects,
            sections: cached.sections,
        })
    }

    /// Keep what a successful read returned, replacing whatever the view held before.
    pub fn save(
        &mut self,
        view: &str,
        tasks: Vec<Task>,
        projects: Vec<Project>,
        sections: Vec<Section>,
        now: u64,
    ) {
        self.fetched_at = Some(now);
        let cached = Cached {
            version: VERSION,
            saved_at: now,
            tasks,
            projects,
            sections,
        };
        let Ok(text) = serde_json::to_string(&cached) else {
            return;
        };
        let path = self.path(view);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, text);
    }

    /// The directory the view files live in.
    #[cfg(test)]
    pub(crate) fn dir(&self) -> &Path {
        &self.dir
    }

    /// How old the tasks on screen are, or `None` when the pane has never held any.
    pub fn age(&self, now: u64) -> Option<u64> {
        self.fetched_at.map(|at| now.saturating_sub(at))
    }

    /// One file per view, named by the view name's bytes in hex so that a name carrying a slash,
    /// a space or a dot still names one file of its own.
    fn path(&self, view: &str) -> PathBuf {
        let name: String = view.bytes().map(|byte| format!("{byte:02x}")).collect();
        self.dir.join(format!("{name}.json"))
    }
}

/// The stale mark's age, kept to a few characters: a pane is about 32 columns wide.
pub fn age_text(seconds: u64) -> String {
    match seconds {
        seconds if seconds < MINUTE => "now".to_string(),
        seconds if seconds < HOUR => format!("{}m", seconds / MINUTE),
        seconds if seconds < DAY => format!("{}h", seconds / HOUR),
        seconds => format!("{}d", seconds / DAY),
    }
}

/// The wall clock in Unix seconds, which is what a cache file records and the stale mark counts
/// from. A clock before the epoch reads as the epoch rather than panicking.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// Delete whatever the directory holds, which is how a test starts from no cache at all.
#[cfg(test)]
pub(crate) fn clear(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("herdr-todoist-cache-{name}-{}", std::process::id()));
        clear(&dir);
        dir
    }

    fn task(id: &str) -> Task {
        serde_json::from_str(&format!(
            r#"{{"id":"{id}","content":"one","project_id":"p1"}}"#
        ))
        .expect("task")
    }

    fn project() -> Project {
        serde_json::from_str(r#"{"id":"p1","name":"First"}"#).expect("project")
    }

    #[test]
    fn a_saved_view_comes_back_with_its_tasks_and_its_age() {
        let mut cache = Cache::new(scratch("roundtrip"));

        cache.save("all", vec![task("1")], vec![project()], Vec::new(), 1_000);

        let mut reopened = Cache::new(cache.dir.clone());
        let snapshot = reopened.load("all").expect("the view was saved");
        assert_eq!(snapshot.tasks.len(), 1);
        assert_eq!(snapshot.tasks[0].id, "1");
        assert_eq!(snapshot.projects[0].name, "First");
        assert_eq!(reopened.age(1_300), Some(300));
    }

    #[test]
    fn a_view_that_was_never_saved_has_no_cache_and_no_age() {
        let mut cache = Cache::new(scratch("cold"));

        assert!(cache.load("all").is_none());
        assert_eq!(cache.age(1_000), None);
    }

    #[test]
    fn two_views_keep_their_own_files_whatever_their_names_hold() {
        let mut cache = Cache::new(scratch("two-views"));

        cache.save("all", vec![task("1")], Vec::new(), Vec::new(), 10);
        cache.save("work / home", vec![task("2")], Vec::new(), Vec::new(), 20);

        assert_eq!(cache.load("all").expect("all").tasks[0].id, "1");
        assert_eq!(
            cache.load("work / home").expect("the other view").tasks[0].id,
            "2"
        );
    }

    #[test]
    fn a_corrupt_or_half_written_file_is_no_cache_rather_than_a_failure() {
        let dir = scratch("corrupt");
        let mut cache = Cache::new(&dir);
        cache.save("all", vec![task("1")], Vec::new(), Vec::new(), 10);
        let path = cache.path("all");
        let whole = std::fs::read_to_string(&path).expect("the file");
        std::fs::write(&path, &whole[..whole.len() / 2]).expect("truncate");

        assert!(Cache::new(&dir).load("all").is_none());
    }

    #[test]
    fn a_file_written_by_another_version_is_ignored() {
        let dir = scratch("version");
        let mut cache = Cache::new(&dir);
        cache.save("all", vec![task("1")], Vec::new(), Vec::new(), 10);
        let path = cache.path("all");
        let whole = std::fs::read_to_string(&path).expect("the file");
        std::fs::write(&path, whole.replace(r#""version":1"#, r#""version":99"#)).expect("write");

        assert!(Cache::new(&dir).load("all").is_none());
    }

    #[test]
    fn the_age_reads_in_whole_units_and_never_runs_backwards() {
        assert_eq!(age_text(0), "now");
        assert_eq!(age_text(59), "now");
        assert_eq!(age_text(60), "1m");
        assert_eq!(age_text(5 * 60 + 30), "5m");
        assert_eq!(age_text(59 * 60), "59m");
        assert_eq!(age_text(3 * 60 * 60), "3h");
        assert_eq!(age_text(50 * 60 * 60), "2d");

        let mut cache = Cache::new(scratch("backwards"));
        cache.save("all", Vec::new(), Vec::new(), Vec::new(), 1_000);
        assert_eq!(cache.age(900), Some(0), "a clock that went back reads as 0");
    }
}
