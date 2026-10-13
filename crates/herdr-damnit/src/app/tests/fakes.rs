use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use herdr_damnit_application::{
    Clock, DamRunner, Finished, Herdr, RunningJob, SpawnError, Workspace,
};
use herdr_damnit_domain::Date;

pub(crate) const TODAY: &str = "2026-09-20";

pub(crate) const ONE_AGENT: &str = r#"{"result":{"agents":[
  {"pane_id":"w1:p2","workspace_id":"w1","agent":"claude","name":"planner"}]}}"#;

pub(crate) const NO_AGENT: &str = r#"{"result":{"agents":[]}}"#;

#[derive(Default)]
pub(crate) struct RecordingHerdr {
    pub(crate) listing: String,
    pub(crate) calls: Arc<Mutex<Vec<String>>>,
}

impl Herdr for RecordingHerdr {
    fn call(&self, args: &[&str]) -> Result<String, String> {
        self.calls.lock().expect("the calls").push(args.join(" "));
        match args {
            ["agent", "list"] => Ok(self.listing.clone()),
            _ => Ok(String::new()),
        }
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

pub(crate) struct StoppedClock {
    pub(crate) today: Date,
    pub(crate) now: Instant,
}

impl Clock for StoppedClock {
    fn today(&self) -> Date {
        self.today
    }

    fn now(&self) -> Instant {
        self.now
    }
}
