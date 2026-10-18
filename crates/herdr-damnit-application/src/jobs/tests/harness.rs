use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::{Clock, DamRunner, Finished, Jobs, Request, RunningJob, SpawnError, Submitted};

pub(super) struct TestClock;

impl Clock for TestClock {
    fn today(&self) -> herdr_damnit_domain::Date {
        herdr_damnit_domain::parse_date("2026-09-20").expect("a date")
    }

    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[derive(Default)]
pub(super) struct Recorder {
    pub(super) log: Arc<Mutex<Vec<Vec<String>>>>,
    pub(super) senders: Arc<Mutex<Vec<Sender<Finished>>>>,
    pub(super) refuse_spawn: bool,
    pub(super) io_error: Option<String>,
}

impl DamRunner for Recorder {
    fn spawn(&self, argv: &[String]) -> Result<RunningJob, SpawnError> {
        if self.refuse_spawn {
            return Err(SpawnError::NotFound);
        }
        if let Some(error) = &self.io_error {
            return Err(SpawnError::Io(error.clone()));
        }
        self.log.lock().expect("the log").push(argv.to_vec());
        let (sender, results) = channel();
        self.senders.lock().expect("the senders").push(sender);
        Ok(RunningJob {
            cancel: Box::new(|| {}),
            results,
        })
    }
}

pub(super) fn ok(stdout: &str) -> Finished {
    Finished {
        code: Some(0),
        stdout: stdout.to_string(),
        stderr: String::new(),
    }
}

pub(super) struct Harness {
    pub(super) jobs: Jobs,
    log: Arc<Mutex<Vec<Vec<String>>>>,
    senders: Arc<Mutex<Vec<Sender<Finished>>>>,
}

pub(super) fn harness() -> Harness {
    let recorder = Recorder::default();
    let log = Arc::clone(&recorder.log);
    let senders = Arc::clone(&recorder.senders);
    Harness {
        jobs: Jobs::new(Box::new(recorder), Box::new(TestClock)),
        log,
        senders,
    }
}

impl Harness {
    pub(super) fn submit(&mut self, request: Request) -> Submitted {
        self.jobs.submit(request)
    }

    pub(super) fn answer(&self, which: usize, finished: Finished) {
        let _ = self.senders.lock().expect("the senders")[which].send(finished);
    }

    pub(super) fn lines(&self) -> Vec<String> {
        self.log
            .lock()
            .expect("the log")
            .iter()
            .map(|argv| argv.join(" "))
            .collect()
    }
}
