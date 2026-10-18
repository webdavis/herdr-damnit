use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use herdr_damnit_adapters::StateDir;
use herdr_damnit_application::{PaneMemory, PaneOpen, PluginPanes, SplitSide, Workspace};

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Call {
    Live(String),
    Remembered(String),
    Open(PaneOpen),
    Focus(String),
    Close(String),
    Resize(String, SplitSide, f32),
    Remember(String, String),
    Forget(String),
}

pub(super) type Log = Rc<RefCell<Vec<Call>>>;

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

pub(super) struct FakePanes {
    pub(super) log: Log,
    pub(super) live: Vec<String>,
    pub(super) opens: Result<String, String>,
    pub(super) focus_refused: bool,
    pub(super) close_refused: bool,
    pub(super) resizes: Result<bool, String>,
}

impl FakePanes {
    pub(super) fn showing(log: &Log, live: &[&str]) -> Self {
        Self {
            log: Rc::clone(log),
            live: live.iter().map(|pane| pane.to_string()).collect(),
            opens: Ok("w:p9".to_string()),
            focus_refused: false,
            close_refused: false,
            resizes: Ok(true),
        }
    }
}

impl PluginPanes for FakePanes {
    fn live(&self, workspace: &str) -> Result<Vec<String>, String> {
        self.log
            .borrow_mut()
            .push(Call::Live(workspace.to_string()));
        Ok(self.live.clone())
    }

    fn open(&self, open: &PaneOpen) -> Result<String, String> {
        self.log.borrow_mut().push(Call::Open(open.clone()));
        self.opens.clone()
    }

    fn focus(&self, pane: &str) -> Result<(), String> {
        self.log.borrow_mut().push(Call::Focus(pane.to_string()));
        refused_when(self.focus_refused)
    }

    fn close(&self, pane: &str) -> Result<(), String> {
        self.log.borrow_mut().push(Call::Close(pane.to_string()));
        refused_when(self.close_refused)
    }

    fn resize(&self, pane: &str, side: SplitSide, share: f32) -> Result<bool, String> {
        self.log
            .borrow_mut()
            .push(Call::Resize(pane.to_string(), side, share));
        self.resizes.clone()
    }
}

fn refused_when(refused: bool) -> Result<(), String> {
    if refused {
        Err("herdr: no such pane".to_string())
    } else {
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct FakeMemory {
    pub(super) log: Log,
    pub(super) panes: RefCell<HashMap<String, String>>,
}

impl PaneMemory for FakeMemory {
    fn remembered(&self, workspace: &str) -> Option<String> {
        self.log
            .borrow_mut()
            .push(Call::Remembered(workspace.to_string()));
        self.panes.borrow().get(workspace).cloned()
    }

    fn remember(&self, workspace: &str, pane: &str) {
        self.log
            .borrow_mut()
            .push(Call::Remember(workspace.to_string(), pane.to_string()));
        self.panes
            .borrow_mut()
            .insert(workspace.to_string(), pane.to_string());
    }

    fn forget(&self, workspace: &str) {
        self.log
            .borrow_mut()
            .push(Call::Forget(workspace.to_string()));
        self.panes.borrow_mut().remove(workspace);
    }
}
