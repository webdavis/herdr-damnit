use herdr_damnit_adapters::StateDir;
use herdr_damnit_application::Workspace;

pub(super) fn here() -> Workspace {
    Workspace {
        workspace: Some("w".to_string()),
        me: "w:p1".to_string(),
    }
}

pub(super) fn scratch_state(name: &str) -> StateDir {
    let dir = std::env::temp_dir().join(format!("herdr-damnit-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    StateDir::new(dir)
}

pub(super) fn requested(state: &StateDir) -> Option<String> {
    std::fs::read_to_string(state.view_request_path()).ok()
}
