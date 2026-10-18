#[derive(Debug)]
pub struct Finished {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Clone, Debug)]
pub enum SpawnError {
    NotFound,
    Io(String),
}

pub struct RunningJob {
    pub cancel: Box<dyn Fn() + Send + Sync>,
    pub results: std::sync::mpsc::Receiver<Finished>,
}

pub trait DamRunner: Send + Sync {
    fn spawn(&self, argv: &[String]) -> Result<RunningJob, SpawnError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListedAgent {
    pub pane: String,
    pub workspace: String,
    pub kind: Option<String>,
    pub name: Option<String>,
    pub profile: Option<String>,
}

pub trait AgentPanes {
    fn agents(&self) -> Result<Vec<ListedAgent>, String>;
    fn send_text(&self, pane: &str, text: &str) -> Result<(), String>;
    fn focus_agent(&self, pane: &str) -> Result<(), String>;
}

pub trait Clock: Send + Sync {
    fn today(&self) -> herdr_damnit_domain::Date;
    fn now(&self) -> std::time::Instant;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitSide {
    Right,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanePlacement {
    Overlay,
    Split(SplitSide),
    Tab,
    Zoomed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaneFocus {
    Take,
    Leave,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaneOpen {
    pub workspace: String,
    pub neighbor: String,
    pub placement: PanePlacement,
    pub focus: PaneFocus,
}

pub trait PluginPanes {
    fn live(&self, workspace: &str) -> Result<Vec<String>, String>;
    fn open(&self, open: &PaneOpen) -> Result<String, String>;
    fn focus(&self, pane: &str) -> Result<(), String>;
    fn close(&self, pane: &str) -> Result<(), String>;
    fn resize(&self, pane: &str, side: SplitSide, share: f32) -> Result<bool, String>;
}

pub trait PaneMemory {
    fn remembered(&self, workspace: &str) -> Option<String>;
    fn remember(&self, workspace: &str, pane: &str);
    fn forget(&self, workspace: &str);
}
