use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::{PaneCall, PaneLog};
use crate::PaneMemory;

pub struct FakeMemory {
    pub log: PaneLog,
    pub panes: RefCell<HashMap<String, String>>,
}

impl FakeMemory {
    pub fn sharing(log: &PaneLog) -> Self {
        Self {
            log: Rc::clone(log),
            panes: RefCell::default(),
        }
    }

    pub fn remembering(log: &PaneLog, workspace: &str, pane: &str) -> Self {
        let memory = Self::sharing(log);
        memory
            .panes
            .borrow_mut()
            .insert(workspace.to_string(), pane.to_string());
        memory
    }
}

impl PaneMemory for FakeMemory {
    fn remembered(&self, workspace: &str) -> Option<String> {
        self.log
            .borrow_mut()
            .push(PaneCall::Remembered(workspace.to_string()));
        self.panes.borrow().get(workspace).cloned()
    }

    fn remember(&self, workspace: &str, pane: &str) {
        self.log
            .borrow_mut()
            .push(PaneCall::Remember(workspace.to_string(), pane.to_string()));
        self.panes
            .borrow_mut()
            .insert(workspace.to_string(), pane.to_string());
    }

    fn forget(&self, workspace: &str) {
        self.log
            .borrow_mut()
            .push(PaneCall::Forget(workspace.to_string()));
        self.panes.borrow_mut().remove(workspace);
    }
}
