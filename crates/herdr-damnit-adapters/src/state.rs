use std::path::{Path, PathBuf};

use herdr_damnit_application::PaneMemory;

pub struct StateDir(PathBuf);

impl StateDir {
    pub fn new(dir: PathBuf) -> Self {
        Self(dir)
    }

    pub fn locate(herdr_plugin_state_dir: Option<&Path>, state_home: &Path) -> Self {
        Self(match herdr_plugin_state_dir {
            Some(dir) => dir.to_path_buf(),
            None => state_home.join("herdr/plugins/state/herdr-damnit"),
        })
    }

    pub fn view_request_path(&self) -> PathBuf {
        self.0.join("requested-view")
    }

    pub fn request_view(&self, name: &str) {
        write(&self.view_request_path(), name);
    }

    pub fn clear_view_request(&self) {
        let _ = std::fs::remove_file(self.view_request_path());
    }

    fn pane_path(&self, workspace: &str) -> PathBuf {
        self.0.join("panes").join(workspace)
    }
}

impl PaneMemory for StateDir {
    fn remembered(&self, workspace: &str) -> Option<String> {
        read(&self.pane_path(workspace))
    }

    fn remember(&self, workspace: &str, pane: &str) {
        write(&self.pane_path(workspace), pane);
    }

    fn forget(&self, workspace: &str) {
        let _ = std::fs::remove_file(self.pane_path(workspace));
    }
}

pub fn take_requested_view(path: &Path) -> Option<String> {
    let contents = read(path);
    let _ = std::fs::remove_file(path);
    contents
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

        assert_eq!(take_requested_view(&path), None);
        write(&path, "today");
        assert_eq!(take_requested_view(&path), Some("today".to_string()));
        assert_eq!(
            take_requested_view(&path),
            None,
            "the request outlived its read"
        );
    }

    #[test]
    fn a_blank_file_is_no_request_at_all() {
        let path = scratch("blank");
        write(&path, "  \n");

        assert_eq!(read(&path), None);
    }

    #[test]
    fn the_state_directory_is_the_plugins_own_name_under_herdr() {
        assert_eq!(
            StateDir::locate(None, Path::new("/x/.local/state")).0,
            Path::new("/x/.local/state/herdr/plugins/state/herdr-damnit")
        );
    }

    #[test]
    fn herdrs_own_state_directory_wins_when_it_names_one() {
        assert_eq!(
            StateDir::locate(Some(Path::new("/given")), Path::new("/x/.local/state")).0,
            Path::new("/given")
        );
    }
}
