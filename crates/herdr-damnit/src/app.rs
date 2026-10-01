mod apply;
mod boxes;
mod done;
mod edits;
mod handshake;
mod header;
mod keys;
mod overlay_keys;
mod pickers;
mod screen;
mod views;

use crate::overlay::LineBox;
pub use crate::overlay::Overlay;
pub use screen::Screen;

#[cfg(test)]
pub(crate) mod tests;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use herdr_damnit_adapters::Config;
use herdr_damnit_application::{JobId, JobKind, Jobs, Submitted};
use herdr_damnit_domain::{Cursor, DamVersion, Date, Failure, Object, Oid, Stage, Views, message};

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
    pub status: Cursor,
    pub done: Cursor,
    pub overlay: Option<Overlay>,
    pub view_request: Option<PathBuf>,
    pub stage: Stage,
    pub message: String,
    pub spinner: usize,
    pub config: Config,
    pub today: Date,
    pub refusal: Option<String>,
    pub version: Option<DamVersion>,
    pub detail: Option<Object>,
    under_detail: Screen,
    last_reread: Option<Instant>,
    awaiting: Option<(JobId, LineBox)>,
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
        let mut views = Views::new(&config.views());
        if let Some(name) = &config.default_view {
            views.select_named(name);
        }
        Self {
            screen: Screen::List,
            views,
            list: Cursor::new(Vec::new()),
            status: Cursor::new(Vec::new()),
            done: Cursor::new(Vec::new()),
            overlay: None,
            view_request: None,
            stage: Stage::default(),
            message: String::new(),
            spinner: 0,
            config,
            today,
            refusal: None,
            version: None,
            detail: None,
            under_detail: Screen::List,
            last_reread: None,
            awaiting: None,
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

    pub fn submit(&mut self, kind: JobKind, argv: Vec<String>) -> Option<JobId> {
        match self.jobs.submit(kind, argv) {
            Submitted::Started(id) => return Some(id),
            Submitted::Refused(said) | Submitted::Failed(said) => self.message = said,
            Submitted::NotInstalled => self.message = message(&Failure::NotInstalled),
        }
        None
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

    pub fn subject_of(&self, oid: &Oid) -> Option<&str> {
        self.object(oid).map(|object| object.subject.as_str())
    }

    fn object(&self, oid: &Oid) -> Option<&Object> {
        self.objects
            .iter()
            .chain(&self.done_objects)
            .find(|object| &object.oid == oid)
    }
}
