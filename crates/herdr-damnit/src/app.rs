mod apply;
mod done;
mod header;
mod keys;
mod screen;

pub use screen::Screen;

#[cfg(test)]
pub(crate) mod tests;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use herdr_damnit_adapters::Config;
use herdr_damnit_application::{JobKind, Jobs, Submitted, argv};
use herdr_damnit_domain::{
    Cursor, DONE_QUERY, DamVersion, Date, Failure, Object, Oid, Stage, Views, message,
};

const BUSY_POLL_WINDOW: Duration = Duration::from_millis(50);

const IDLE_POLL_WINDOW: Duration = Duration::from_millis(200);

#[derive(Debug, PartialEq, Eq)]
pub enum After {
    Stay,
}

pub struct App {
    pub screen: Screen,
    pub views: Views,
    pub list: Cursor,
    pub stage: Stage,
    pub message: String,
    pub spinner: usize,
    pub config: Config,
    pub today: Date,
    pub refusal: Option<String>,
    pub version: Option<DamVersion>,
    pub detail: Option<Object>,
    under_detail: Screen,
    handshake_accepted: bool,
    newer_dam_warning_shown: bool,
    objects: Vec<Object>,
    done_objects: Vec<Object>,
    completion_days: HashMap<Oid, Date>,
    done_list_requested: bool,
    done_log_requested: bool,
    jobs: Jobs,
}

impl App {
    pub fn new(config: Config, jobs: Jobs, today: Date) -> Self {
        Self {
            screen: Screen::List,
            views: Views::new(&config.views()),
            list: Cursor::new(Vec::new()),
            stage: Stage::default(),
            message: String::new(),
            spinner: 0,
            config,
            today,
            refusal: None,
            version: None,
            detail: None,
            under_detail: Screen::List,
            handshake_accepted: false,
            newer_dam_warning_shown: false,
            objects: Vec::new(),
            done_objects: Vec::new(),
            completion_days: HashMap::new(),
            done_list_requested: false,
            done_log_requested: false,
            jobs,
        }
    }

    pub fn poll_window(&self) -> Duration {
        match self.jobs.in_flight() {
            0 => IDLE_POLL_WINDOW,
            _ => BUSY_POLL_WINDOW,
        }
    }

    pub fn submit(&mut self, kind: JobKind, argv: Vec<String>) {
        match self.jobs.submit(kind, argv) {
            Submitted::Started(_) => {}
            Submitted::Refused(said) | Submitted::Failed(said) => self.message = said,
            Submitted::NotInstalled => self.message = message(&Failure::NotInstalled),
        }
    }

    pub fn tick(&mut self, _now: Instant) {
        if self.jobs.in_flight() > 0 {
            self.spinner = self.spinner.wrapping_add(1);
        }
        for completion in self.jobs.drain() {
            self.apply(completion);
        }
    }

    pub fn subject_of(&self, oid: &Oid) -> Option<&str> {
        self.objects
            .iter()
            .chain(&self.done_objects)
            .find(|object| &object.oid == oid)
            .map(|object| object.subject.as_str())
    }

    fn show(&mut self, screen: Screen) -> After {
        self.screen = screen;
        if screen == Screen::Done && self.handshake_accepted {
            if !self.done_list_requested {
                self.done_list_requested = true;
                self.submit(JobKind::ReadDone, argv::list(DONE_QUERY));
            }
            if !self.done_log_requested {
                self.done_log_requested = true;
                self.submit(JobKind::ReadLog, argv::log());
            }
        }
        After::Stay
    }
}
