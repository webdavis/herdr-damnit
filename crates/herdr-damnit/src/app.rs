mod agent;
mod apply;
mod boxes;
mod done;
mod edits;
mod handshake;
mod header;
mod keys;
mod list;
mod overlay_keys;
mod pending;
mod pickers;
mod reach;
mod rereads;
mod screen;
mod showing;
mod status;
mod views;
mod work;

pub use crate::overlay::Overlay;
pub use done::DoneScreen;
pub(crate) use handshake::status_refusal;
pub use list::ListScreen;
use pending::Pending;
use reach::AgentReach;
use rereads::Rereads;
pub use screen::Screen;
pub use showing::Showing;
pub use status::StatusScreen;
use work::Work;

#[cfg(test)]
pub(crate) mod tests;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use herdr_damnit_adapters::Config;
use herdr_damnit_application::{
    AgentPanes, Clock, JobId, Jobs, Phase, Request, Submitted, Workspace,
};
use herdr_damnit_domain::{Failure, Views, message};

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
    status: StatusScreen,
    done: DoneScreen,
    message: String,
    config: Config,
    phase: Phase,
    agents: AgentReach,
    rereads: Rereads,
    work: Work,
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
            status: StatusScreen::new(config.icons()),
            done: DoneScreen::new(),
            message: String::new(),
            config,
            phase: Phase::Starting,
            agents: AgentReach::new(agents, here),
            rereads: Rereads::new(),
            work: Work::new(jobs),
        }
    }

    pub fn poll_window(&self) -> Duration {
        match self.work.in_flight() {
            0 => IDLE_POLL_WINDOW,
            _ => BUSY_POLL_WINDOW,
        }
    }

    pub fn submit(&mut self, request: Request) -> Option<JobId> {
        match self.work.submit(request) {
            Submitted::Started(id) => return Some(id),
            Submitted::Refused(said) | Submitted::Failed(said) => self.message = said,
            Submitted::NotInstalled => self.phase = Phase::Refused(message(&Failure::NotInstalled)),
        }
        None
    }

    pub fn watch_view_requests(&mut self, path: PathBuf) {
        self.rereads.watch(path);
    }

    pub fn watch_config(&mut self, path: PathBuf) {
        self.rereads.watch_config(path);
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn say(&mut self, said: String) {
        self.message = said;
    }

    pub fn list(&self) -> &ListScreen {
        &self.list
    }

    pub fn status(&self) -> &StatusScreen {
        &self.status
    }

    pub fn done(&self) -> &DoneScreen {
        &self.done
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
        for completion in self.work.turn() {
            self.apply(completion);
        }
        self.take_view_request();
        self.reread_on_the_interval(now);
    }
}
