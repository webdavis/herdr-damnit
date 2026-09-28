mod done;
mod header;
mod screen;

pub use screen::Screen;

#[cfg(test)]
pub(crate) mod tests;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent};
use herdr_damnit_adapters::{Config, wire};
use herdr_damnit_application::{
    Completion, Handshake, JobKind, Jobs, Submitted, SyncKind, argv, check_status, check_version,
};
use herdr_damnit_domain::{
    Cursor, DONE_QUERY, DamVersion, Date, Failure, Object, Oid, RowStyle, Stage, Views, classify,
    message, rows,
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

    pub fn key(&mut self, key: KeyEvent) -> After {
        match key.code {
            KeyCode::Tab => self.show(self.screen.next()),
            KeyCode::BackTab => self.show(self.screen.previous()),
            _ => After::Stay,
        }
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

    fn apply(&mut self, completion: Completion) {
        if completion.finished.code != Some(0) {
            self.message = message(&classify(
                completion.finished.code,
                wire::error_document(&completion.finished.stderr),
                &completion.finished.stderr,
            ));
            return;
        }
        if self.applied_as_handshake(&completion) {
            return;
        }
        if let Err(said) = self.read_into_model(&completion) {
            self.message = said;
            return;
        }
        for kind in completion.follow_up {
            let argv = self.follow_up_argv(&kind);
            self.submit(kind, argv);
        }
    }

    fn applied_as_handshake(&mut self, completion: &Completion) -> bool {
        match completion.kind {
            JobKind::Version => {
                match check_version(&completion.finished.stdout) {
                    Handshake::Refuse(said) => self.refusal = Some(said),
                    Handshake::Ready { version, warning } => {
                        self.version = Some(version);
                        if let (Some(warning), false) = (warning, self.newer_dam_warning_shown) {
                            self.message = warning;
                            self.newer_dam_warning_shown = true;
                        }
                        self.submit(JobKind::Handshake, argv::status());
                    }
                }
                true
            }
            JobKind::Handshake => {
                match check_status(&completion.finished.stdout) {
                    Err(said) => self.refusal = Some(said),
                    Ok(()) => {
                        if let Err(said) = self.read_status(&completion.finished.stdout) {
                            self.message = said;
                            return true;
                        }
                        let query = self.views.current().query.clone();
                        self.submit(JobKind::ReadList, argv::list(&query));
                        self.handshake_accepted = true;
                        self.show(self.screen);
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn read_into_model(&mut self, completion: &Completion) -> Result<(), String> {
        let unreadable = || message(&Failure::Unreadable);
        match &completion.kind {
            JobKind::ReadStatus => self.read_status(&completion.finished.stdout)?,
            JobKind::ReadList => {
                self.objects =
                    wire::objects(&completion.finished.stdout).map_err(|_| unreadable())?;
                self.redraw_list();
            }
            JobKind::ReadDone => {
                self.done_objects =
                    wire::objects(&completion.finished.stdout).map_err(|_| unreadable())?;
            }
            JobKind::ReadLog => {
                self.completion_days =
                    wire::completions(&completion.finished.stdout).map_err(|_| unreadable())?;
            }
            JobKind::Exclusive(SyncKind::Push) => {
                self.message =
                    wire::push_summary(&completion.finished.stdout).map_err(|_| unreadable())?;
            }
            JobKind::Exclusive(SyncKind::Pull) => {
                self.message =
                    wire::pull_summary(&completion.finished.stdout).map_err(|_| unreadable())?;
            }
            _ => {}
        }
        Ok(())
    }

    fn read_status(&mut self, stdout: &str) -> Result<(), String> {
        self.stage = wire::stage(stdout).map_err(|_| message(&Failure::Unreadable))?;
        self.redraw_list();
        Ok(())
    }

    fn redraw_list(&mut self) {
        let style = RowStyle {
            icons: self.config.icons(),
            today: self.today,
        };
        self.list.replace(rows(&self.objects, &self.stage, style));
    }

    fn follow_up_argv(&self, kind: &JobKind) -> Vec<String> {
        match kind {
            JobKind::ReadList => argv::list(&self.views.current().query),
            JobKind::ReadDone => argv::list(DONE_QUERY),
            JobKind::ReadStatus => argv::status(),
            JobKind::ReadShow(oid) => argv::show(oid),
            JobKind::ReadLog => argv::log(),
            JobKind::Version => argv::version(),
            JobKind::Handshake => argv::status(),
            JobKind::Write => Vec::new(),
            JobKind::Exclusive(SyncKind::Commit) => argv::commit(""),
            JobKind::Exclusive(SyncKind::Push) => argv::push(),
            JobKind::Exclusive(SyncKind::Pull) => argv::pull(),
        }
    }
}
