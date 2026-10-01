use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent};
use herdr_damnit_adapters::Config;
use herdr_damnit_application::{
    Clock, DamRunner, Finished, Herdr, Jobs, RunningJob, SpawnError, Workspace,
};
use herdr_damnit_domain::{Date, parse_date};

use crate::app::{After, App};

pub(crate) const TODAY: &str = "2026-09-20";

pub(crate) const ONE_AGENT: &str = r#"{"result":{"agents":[
  {"pane_id":"w1:p2","workspace_id":"w1","agent":"claude","name":"planner"}]}}"#;

pub(crate) const NO_AGENT: &str = r#"{"result":{"agents":[]}}"#;

#[derive(Default)]
pub(crate) struct RecordingHerdr {
    listing: String,
    calls: Arc<Mutex<Vec<String>>>,
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
}

impl DamRunner for Recorder {
    fn spawn(&self, argv: &[String]) -> Result<RunningJob, SpawnError> {
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

struct StoppedClock {
    today: Date,
    now: Instant,
}

impl Clock for StoppedClock {
    fn today(&self) -> Date {
        self.today
    }

    fn now(&self) -> Instant {
        self.now
    }
}

struct MissingDam;

impl DamRunner for MissingDam {
    fn spawn(&self, _argv: &[String]) -> Result<RunningJob, SpawnError> {
        Err(SpawnError::NotFound)
    }
}

pub(crate) struct Harness {
    pub(crate) app: App,
    started: Instant,
    log: Arc<Mutex<Vec<Vec<String>>>>,
    senders: Arc<Mutex<Vec<Sender<Finished>>>>,
    cancels: Arc<AtomicUsize>,
    herdr_calls: Arc<Mutex<Vec<String>>>,
}

pub(crate) fn harness() -> Harness {
    harness_with(Config::parse("").expect("the default config"))
}

pub(crate) fn harness_with(config: Config) -> Harness {
    harness_with_listing(config, ONE_AGENT)
}

pub(crate) fn harness_with_missing_dam() -> Harness {
    let today = parse_date(TODAY).expect("a date");
    let now = Instant::now();
    Harness {
        app: App::new(
            Config::parse("").expect("the default config"),
            Jobs::new(Box::new(MissingDam), Box::new(StoppedClock { today, now })),
            Box::new(RecordingHerdr::default()),
            here(),
            today,
        ),
        started: now,
        log: Arc::default(),
        senders: Arc::default(),
        cancels: Arc::default(),
        herdr_calls: Arc::default(),
    }
}

pub(crate) fn harness_with_listing(config: Config, listing: &str) -> Harness {
    let herdr = RecordingHerdr {
        listing: listing.to_string(),
        calls: Arc::default(),
    };
    let herdr_calls = Arc::clone(&herdr.calls);
    let recorder = Recorder::default();
    let log = Arc::clone(&recorder.log);
    let senders = Arc::clone(&recorder.senders);
    let cancels = Arc::clone(&recorder.cancels);
    let today = parse_date(TODAY).expect("a date");
    let started = Instant::now();
    Harness {
        app: App::new(
            config,
            Jobs::new(
                Box::new(recorder),
                Box::new(StoppedClock {
                    today,
                    now: started,
                }),
            ),
            Box::new(herdr),
            here(),
            today,
        ),
        started,
        log,
        senders,
        cancels,
        herdr_calls,
    }
}

impl Harness {
    pub(crate) fn started(&self) -> Instant {
        self.started
    }

    pub(crate) fn answer(&mut self, which: usize, code: i32, stdout: &str, stderr: &str) {
        let _ = self.senders.lock().expect("the senders")[which].send(Finished {
            code: Some(code),
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
            elapsed: Duration::from_millis(9),
        });
        self.app.tick(Instant::now());
    }

    pub(crate) fn press(&mut self, code: KeyCode) -> After {
        self.app.key(KeyEvent::from(code))
    }

    pub(crate) fn last(&self) -> String {
        self.lines().last().cloned().unwrap_or_default()
    }

    pub(crate) fn select(&mut self, oid: &str) {
        let cursor = self.app.cursor_mut();
        cursor.move_by(isize::MIN);
        while cursor.selected_oid().map(|found| found.as_str()) != Some(oid) {
            assert!(cursor.move_by(1), "{oid} is not on the showing screen");
        }
    }

    pub(crate) fn type_line(&mut self, text: &str) {
        for character in text.chars() {
            self.press(KeyCode::Char(character));
        }
    }

    pub(crate) fn herdr_calls(&self) -> Vec<String> {
        self.herdr_calls.lock().expect("the calls").clone()
    }

    pub(crate) fn cancels(&self) -> usize {
        self.cancels.load(Ordering::SeqCst)
    }

    pub(crate) fn log_argv_last(&self) -> Vec<String> {
        self.log
            .lock()
            .expect("the log")
            .last()
            .cloned()
            .unwrap_or_default()
    }

    pub(crate) fn lines(&self) -> Vec<String> {
        self.log
            .lock()
            .expect("the log")
            .iter()
            .map(|argv| argv.join(" "))
            .collect()
    }
}
