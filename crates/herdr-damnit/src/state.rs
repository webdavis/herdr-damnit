//! The plugin's own files under `HERDR_PLUGIN_STATE_DIR`: the pane it last opened in a workspace,
//! and the view a `view` action asked for. herdr runs every action as its own process, so these
//! files are how one run tells the next, and the pane, what happened.

use std::path::{Path, PathBuf};

/// herdr hands the plugin its own state directory; the documented path is the fallback for a run
/// outside herdr.
pub fn state_dir() -> PathBuf {
    match std::env::var_os("HERDR_PLUGIN_STATE_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => state_home().join("herdr/plugins/state/herdr-damnit"),
    }
}

fn state_home() -> PathBuf {
    match std::env::var_os("XDG_STATE_HOME") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state"),
    }
}

/// The pane this plugin last opened in a workspace, one file per workspace so two workspaces never
/// overwrite each other.
fn pane_path(workspace: &str) -> PathBuf {
    state_dir().join("panes").join(workspace)
}

/// The view a `view` action asked for: one file for the plugin, which a pane about to start reads
/// on its first draw and a pane already open reads on its next tick.
pub fn view_request_path() -> PathBuf {
    state_dir().join("requested-view")
}

pub fn remembered_pane(workspace: &str) -> Option<String> {
    read(&pane_path(workspace))
}

pub fn remember_pane(workspace: &str, pane: &str) {
    write(&pane_path(workspace), pane);
}

pub fn forget_pane(workspace: &str) {
    let _ = std::fs::remove_file(pane_path(workspace));
}

pub fn request_view(path: &Path, name: &str) {
    write(path, name);
}

pub fn clear_view_request(path: &Path) {
    let _ = std::fs::remove_file(path);
}

/// The requested view, which is consumed by the read so the pane does not pull itself back to it
/// after the operator has moved on.
pub fn take_requested_view(path: &Path) -> Option<String> {
    take(path)
}

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, contents);
}

fn read(path: &Path) -> Option<String> {
    let contents = std::fs::read_to_string(path).ok()?;
    let contents = contents.trim().to_string();
    (!contents.is_empty()).then_some(contents)
}

fn take(path: &Path) -> Option<String> {
    let contents = read(path);
    let _ = std::fs::remove_file(path);
    contents
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("herdr-damnit-{name}-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn a_request_is_read_once_and_then_gone() {
        let path = scratch("request");

        assert_eq!(take(&path), None);
        write(&path, "today");
        assert_eq!(take(&path), Some("today".to_string()));
        assert_eq!(take(&path), None, "the request outlived its read");
    }

    #[test]
    fn a_blank_file_is_no_request_at_all() {
        let path = scratch("blank");
        write(&path, "  \n");

        assert_eq!(read(&path), None);
    }
}
