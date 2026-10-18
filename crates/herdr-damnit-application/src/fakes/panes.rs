use std::cell::RefCell;
use std::rc::Rc;

use crate::{PaneOpen, PluginPanes, SplitSide};

#[derive(Clone, Debug, PartialEq)]
pub enum PaneCall {
    Live(String),
    Remembered(String),
    Open(PaneOpen),
    Focus(String),
    Close(String),
    Resize(String, SplitSide, f32),
    Remember(String, String),
    Forget(String),
}

pub type PaneLog = Rc<RefCell<Vec<PaneCall>>>;

pub struct FakePanes {
    pub log: PaneLog,
    pub live: Vec<String>,
    pub opens: Result<String, String>,
    pub focus_refused: bool,
    pub close_refused: bool,
    pub resizes: Result<bool, String>,
}

impl FakePanes {
    pub fn showing(log: &PaneLog, live: &[&str]) -> Self {
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
            .push(PaneCall::Live(workspace.to_string()));
        Ok(self.live.clone())
    }

    fn open(&self, open: &PaneOpen) -> Result<String, String> {
        self.log.borrow_mut().push(PaneCall::Open(open.clone()));
        self.opens.clone()
    }

    fn focus(&self, pane: &str) -> Result<(), String> {
        self.log
            .borrow_mut()
            .push(PaneCall::Focus(pane.to_string()));
        refused_when(self.focus_refused)
    }

    fn close(&self, pane: &str) -> Result<(), String> {
        self.log
            .borrow_mut()
            .push(PaneCall::Close(pane.to_string()));
        refused_when(self.close_refused)
    }

    fn resize(&self, pane: &str, side: SplitSide, share: f32) -> Result<bool, String> {
        self.log
            .borrow_mut()
            .push(PaneCall::Resize(pane.to_string(), side, share));
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
