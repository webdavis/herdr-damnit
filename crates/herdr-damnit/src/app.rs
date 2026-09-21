//! The pane's model and its keys. Nothing here touches a terminal, so every key is tested by the
//! argv it produced and the sentence it left in the status line.

use std::time::{Duration, Instant};

use crossterm::event::KeyEvent;
use herdr_damnit_adapters::{Config, wire};
use herdr_damnit_application::{Completion, JobKind, Jobs, Submitted, SyncKind, argv};
use herdr_damnit_domain::{
    Cursor, Date, Failure, IconSet, Object, RowStyle, Stage, Views, classify, message, rows,
};

/// The poll window while a job is in flight, which is what makes the spinner animate.
const BUSY_WINDOW: Duration = Duration::from_millis(50);

/// The poll window with nothing in flight, so an idle pane costs what it always did.
const IDLE_WINDOW: Duration = Duration::from_millis(200);

const BRAILLE: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
const PLAIN: [&str; 4] = ["|", "/", "-", "\\"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    List,
}

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
    /// The objects of the showing view, which the list rows are rebuilt from.
    objects: Vec<Object>,
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
            objects: Vec::new(),
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
    pub fn key(&mut self, _key: KeyEvent) -> After {
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
        if let Err(said) = self.read(&completion) {
            self.message = said;
            return;
        }
        for kind in completion.follow_up {
            let argv = self.argv_of(&kind);
            self.submit(kind, argv);
        }
    }

    /// Put one successful document into the model. A document that will not parse is the pane's
    /// own failure rather than `dam`'s.
    fn read(&mut self, completion: &Completion) -> Result<(), String> {
        let unreadable = || message(&Failure::Unreadable);
        match &completion.kind {
            JobKind::ReadStatus => {
                self.stage = wire::stage(&completion.finished.stdout).map_err(|_| unreadable())?;
                self.redraw_list();
            }
            JobKind::ReadList => {
                self.objects =
                    wire::objects(&completion.finished.stdout).map_err(|_| unreadable())?;
                self.redraw_list();
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
            JobKind::ReadStatus => argv::status(),
            JobKind::ReadShow(oid) => argv::show(oid),
            JobKind::ReadLog => argv::log(),
            JobKind::Write => Vec::new(),
            JobKind::Exclusive(SyncKind::Commit) => argv::commit(""),
            JobKind::Exclusive(SyncKind::Push) => argv::push(),
            JobKind::Exclusive(SyncKind::Pull) => argv::pull(),
        }
    }

    /// The header's left half. With an exclusive job running it is named; with only reads in
    /// flight, they are counted.
    pub fn header(&self, now: Instant) -> String {
        let Some(elapsed) = self.jobs.elapsed_of_current(now) else {
            return format!("dam  {}", self.views.current().name);
        };
        let reads = self.jobs.in_flight();
        let what = match self.jobs.exclusive() {
            Some(sync) => sync.verb().to_string(),
            None => format!("{reads} read{}", if reads == 1 { "" } else { "s" }),
        };
        format!(
            "dam  {}  {} {what} {}",
            self.views.current().name,
            self.frame(),
            elapsed_text(elapsed)
        )
    }

    fn frame(&self) -> &'static str {
        match self.config.icons() {
            IconSet::NerdFont => BRAILLE[self.spinner % BRAILLE.len()],
            IconSet::Ascii => PLAIN[self.spinner % PLAIN.len()],
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

/// Whole tenths up to ten seconds and whole seconds after that, so the number stops flickering
/// once a job is genuinely slow.
fn elapsed_text(elapsed: Duration) -> String {
    match elapsed.as_secs() < 10 {
        true => format!("{:.1}s", elapsed.as_secs_f32()),
        false => format!("{}s", elapsed.as_secs()),
    }
}

#[cfg(test)]
pub(crate) mod tests;
