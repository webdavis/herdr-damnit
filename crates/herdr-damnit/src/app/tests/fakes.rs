use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use herdr_damnit_application::{
    AgentPanes, Clock, DamRunner, Finished, ListedAgent, RunningJob, SpawnError, Workspace,
};
use herdr_damnit_domain::Date;

pub(crate) const TODAY: &str = "2026-09-20";

pub(crate) fn one_agent() -> Vec<ListedAgent> {
    vec![ListedAgent {
        pane: "w1:p2".to_string(),
        workspace: "w1".to_string(),
        kind: Some("claude".to_string()),
        name: Some("planner".to_string()),
        profile: None,
    }]
}

#[derive(Default)]
pub(crate) struct RecordingHerdr {
    pub(crate) listing: Arc<Mutex<Vec<ListedAgent>>>,
    pub(crate) calls: Arc<Mutex<Vec<String>>>,
}

impl RecordingHerdr {
    fn record(&self, call: String) -> Result<(), String> {
        self.calls.lock().expect("the calls").push(call);
        Ok(())
    }
}

impl AgentPanes for RecordingHerdr {
    fn agents(&self) -> Result<Vec<ListedAgent>, String> {
        self.record("agents".to_string())?;
        Ok(self.listing.lock().expect("the listing").clone())
    }

    fn send_text(&self, pane: &str, text: &str) -> Result<(), String> {
        self.record(format!("send_text {pane} {text}"))
    }

    fn focus_agent(&self, pane: &str) -> Result<(), String> {
        self.record(format!("focus_agent {pane}"))
    }
}

pub(crate) fn here() -> Workspace {
    Workspace {
        workspace: Some("w1".to_string()),
        me: "w1:p9".to_string(),
    }
}

#[derive(Default)]
pub(crate) struct Recorder {
    pub(crate) log: Arc<Mutex<Vec<Vec<String>>>>,
    pub(crate) senders: Arc<Mutex<Vec<Sender<Finished>>>>,
    pub(crate) cancels: Arc<AtomicUsize>,
    pub(crate) missing: Arc<AtomicBool>,
}

impl DamRunner for Recorder {
    fn spawn(&self, argv: &[String]) -> Result<RunningJob, SpawnError> {
        if self.missing.load(Ordering::SeqCst) {
            return Err(SpawnError::NotFound);
        }
        self.log.lock().expect("the log").push(argv.to_vec());
        let (sender, results) = channel();
        self.senders.lock().expect("the senders").push(sender);
        let cancels = Arc::clone(&self.cancels);
        Ok(RunningJob {
            cancel: Box::new(move || {
                cancels.fetch_add(1, Ordering::SeqCst);
            }),
            results,
        })
    }
}

#[derive(Clone)]
pub(crate) struct StoppedClock {
    pub(crate) day: Arc<Mutex<Date>>,
    pub(crate) now: Instant,
}

impl Clock for StoppedClock {
    fn today(&self) -> Date {
        *self.day.lock().expect("the day")
    }

    fn now(&self) -> Instant {
        self.now
    }
}
