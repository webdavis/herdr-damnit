//! The pane's model and its keys. Nothing here touches a terminal, so every key is tested by the
//! argv it produced and the sentence it left in the status line.

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

/// The poll window while a job is in flight, which is what makes the spinner animate.
const BUSY_WINDOW: Duration = Duration::from_millis(50);

/// The poll window with nothing in flight, so an idle pane costs what it always did.
const IDLE_WINDOW: Duration = Duration::from_millis(200);

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
    /// Why the pane will not draw rows at all. A refusal is the whole screen, because every screen
    /// under it would be drawn from a `dam` this pane does not agree with.
    pub refusal: Option<String>,
    /// The version the handshake read, once it has.
    pub version: Option<DamVersion>,
    /// Whether the handshake's warning has already reached the status line, so a newer `dam` is
    /// named once at open rather than on every read.
    warned: bool,
    /// The objects of the showing view, which the list rows are rebuilt from.
    objects: Vec<Object>,
    /// The completed objects the Done screen draws.
    done_objects: Vec<Object>,
    /// The day each completed object was completed on, from the commit that flipped it.
    completed: HashMap<Oid, Date>,
    /// Whether the Done screen's two reads have been made, so entering it a second time costs
    /// nothing.
    read_done: bool,
    read_log: bool,
    jobs: Jobs,
}

impl App {
    pub fn new(config: Config, jobs: Jobs, today: Date) -> Self {
        Self {
            screen: Screen::List,
            views: Views::new(&config.views()),
            list: Cursor::new(Vec::new()),
            stage: empty_stage(),
            message: String::new(),
            spinner: 0,
            config,
            today,
            refusal: None,
            version: None,
            warned: false,
            objects: Vec::new(),
            done_objects: Vec::new(),
            completed: HashMap::new(),
            read_done: false,
            read_log: false,
            jobs,
        }
    }

    /// 50 ms while any job is in flight, 200 ms when none is.
    pub fn poll_window(&self) -> Duration {
        match self.jobs.in_flight() {
            0 => IDLE_WINDOW,
            _ => BUSY_WINDOW,
        }
    }

    pub fn submit(&mut self, kind: JobKind, argv: Vec<String>) {
        match self.jobs.submit(kind, argv) {
            Submitted::Started(_) => {}
            Submitted::Refused(said) | Submitted::Failed(said) => self.message = said,
            Submitted::NotInstalled => self.message = message(&Failure::NotInstalled),
        }
    }

    /// Drain the job channel, apply every completion, step the spinner. Never blocks.
    pub fn tick(&mut self, _now: Instant) {
        if self.jobs.in_flight() > 0 {
            self.spinner = self.spinner.wrapping_add(1);
        }
        for completion in self.jobs.drain() {
            self.apply(completion);
        }
    }

    /// Handle one key. Never blocks and never spawns a thread of its own.
    pub fn key(&mut self, key: KeyEvent) -> After {
        match key.code {
            KeyCode::Tab => self.show(self.screen.next()),
            KeyCode::BackTab => self.show(self.screen.previous()),
            _ => After::Stay,
        }
    }

    /// Draw another screen, reading what it needs the first time it is entered.
    fn show(&mut self, screen: Screen) -> After {
        self.screen = screen;
        if screen == Screen::Done {
            if !self.read_done {
                self.read_done = true;
                self.submit(JobKind::ReadDone, argv::list(DONE_QUERY));
            }
            if !self.read_log {
                self.read_log = true;
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
        if self.handshook(&completion) {
            return;
        }
        if let Err(said) = self.read(&completion) {
            self.message = said;
            return;
        }
        for kind in completion.follow_up {
            let argv = self.argv_of(&kind);
            self.submit(kind, argv);
        }
    }

    /// The handshake's own two completions, which decide whether the pane draws at all. Reports
    /// whether this completion was one of them.
    fn handshook(&mut self, completion: &Completion) -> bool {
        match completion.kind {
            JobKind::Version => {
                match check_version(&completion.finished.stdout) {
                    Handshake::Refuse(said) => self.refusal = Some(said),
                    Handshake::Ready { version, warning } => {
                        self.version = Some(version);
                        if let (Some(warning), false) = (warning, self.warned) {
                            self.message = warning;
                            self.warned = true;
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
                    }
                }
                true
            }
            _ => false,
        }
    }

    /// Put one successful document into the model. A document that will not parse is the pane's
    /// own failure rather than `dam`'s.
    fn read(&mut self, completion: &Completion) -> Result<(), String> {
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
                self.completed =
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

    /// Put one `dam status --json` into the model and rebuild the rows its marks belong to.
    fn read_status(&mut self, stdout: &str) -> Result<(), String> {
        self.stage = wire::stage(stdout).map_err(|_| message(&Failure::Unreadable))?;
        self.redraw_list();
        Ok(())
    }

    /// Rebuild the list rows from the objects and the staging marks, keeping the cursor on the
    /// object it was on.
    fn redraw_list(&mut self) {
        let style = RowStyle {
            icons: self.config.icons(),
            today: self.today,
        };
        self.list.replace(rows(&self.objects, &self.stage, style));
    }

    /// The argv of a read the job table asked for on its own, after a write or a sync.
    fn argv_of(&self, kind: &JobKind) -> Vec<String> {
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

/// The staging model of a pane that has not read `dam status` yet: nothing staged and nothing
/// changed, so the first rows carry no staging marks.
fn empty_stage() -> Stage {
    Stage {
        staged: Vec::new(),
        unstaged: Vec::new(),
        unpushed: Vec::new(),
        conflicts: Vec::new(),
        notices: Vec::new(),
    }
}

mod done;
mod header;
mod screen;

pub use screen::Screen;

#[cfg(test)]
pub(crate) mod tests;
