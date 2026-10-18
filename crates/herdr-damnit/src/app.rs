mod agent;
mod apply;
mod boxes;
mod edits;
mod handshake;
mod header;
mod keys;
mod list;
mod overlay_keys;
mod pending;
mod pickers;
mod screen;
mod showing;
mod views;

pub use crate::overlay::Overlay;
pub(crate) use handshake::status_refusal;
pub use list::ListScreen;
use pending::Pending;
pub use screen::Screen;
pub use showing::Showing;

#[cfg(test)]
pub(crate) mod tests;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use herdr_damnit_adapters::Config;
use herdr_damnit_application::{
    AgentPanes, Clock, JobId, Jobs, Phase, Request, Submitted, Workspace,
};
use herdr_damnit_domain::{Cursor, Date, Failure, Object, Oid, Stage, Views, message, status_rows};

const BUSY_POLL_WINDOW: Duration = Duration::from_millis(50);

const IDLE_POLL_WINDOW: Duration = Duration::from_millis(200);

#[derive(Debug, PartialEq, Eq)]
pub enum After {
    Stay,
    Quit,
    Editor(Vec<String>),
}

pub struct App {
    showing: Showing,
    list: ListScreen,
    pub status: Cursor,
    pub done: Cursor,
    pub view_request: Option<PathBuf>,
    pub stage: Stage,
    pub message: String,
    pub spinner: usize,
    pub config: Config,
    pub phase: Phase,
    last_reread: Option<Instant>,
    pending: HashMap<JobId, Pending>,
    agents: Box<dyn AgentPanes>,
    here: Workspace,
    done_objects: Vec<Object>,
    completion_days: HashMap<Oid, Date>,
    done_opened: bool,
    jobs: Jobs,
}

impl App {
    pub fn new(
        config: Config,
        jobs: Jobs,
        agents: Box<dyn AgentPanes>,
        here: Workspace,
        clock: Box<dyn Clock>,
    ) -> Self {
        let mut views = Views::new(&config.views());
        if let Some(name) = &config.default_view {
            views.select_named(name);
        }
        Self {
            showing: Showing::new(),
            list: ListScreen::new(views, clock),
            status: Cursor::new(status_rows(&Stage::default(), config.icons())),
            done: Cursor::new(Vec::new()),
            view_request: None,
            stage: Stage::default(),
            message: String::new(),
            spinner: 0,
            config,
            phase: Phase::Starting,
            last_reread: None,
            pending: HashMap::new(),
            agents,
            here,
            done_objects: Vec::new(),
            completion_days: HashMap::new(),
            done_opened: false,
            jobs,
        }
    }

    pub fn poll_window(&self) -> Duration {
        match self.jobs.in_flight() {
            0 => IDLE_POLL_WINDOW,
            _ => BUSY_POLL_WINDOW,
        }
    }

    pub fn submit(&mut self, request: Request) -> Option<JobId> {
        match self.jobs.submit(request) {
            Submitted::Started(id) => return Some(id),
            Submitted::Refused(said) | Submitted::Failed(said) => self.message = said,
            Submitted::NotInstalled => self.phase = Phase::Refused(message(&Failure::NotInstalled)),
        }
        None
    }

    pub fn list(&self) -> &ListScreen {
        &self.list
    }

    pub fn showing(&self) -> &Showing {
        &self.showing
    }

    pub fn refusal(&self) -> Option<&str> {
        match &self.phase {
            Phase::Refused(said) => Some(said),
            _ => None,
        }
    }

    pub fn tick(&mut self, now: Instant) {
        if self.jobs.in_flight() > 0 {
            self.spinner = self.spinner.wrapping_add(1);
        }
        for completion in self.jobs.drain() {
            self.apply(completion);
        }
        self.take_view_request();
        self.reread_on_the_interval(now);
    }
}
