use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};
use herdr_damnit_adapters::Config;
use herdr_damnit_application::{Finished, Jobs, ListedAgent};
use herdr_damnit_domain::{Date, parse_date};

use super::fakes::{Recorder, RecordingHerdr, StoppedClock, TODAY, here, one_agent};
use crate::app::{After, App, Screen};
use crate::overlay::Overlay;

pub(crate) struct Harness {
    pub(crate) app: App,
    started: Instant,
    day: Arc<Mutex<Date>>,
    log: Arc<Mutex<Vec<Vec<String>>>>,
    senders: Arc<Mutex<Vec<Sender<Finished>>>>,
    cancels: Arc<AtomicUsize>,
    missing: Arc<AtomicBool>,
    herdr_calls: Arc<Mutex<Vec<String>>>,
    agents: Arc<Mutex<Vec<ListedAgent>>>,
}

pub(crate) fn harness() -> Harness {
    harness_with(Config::parse("").expect("the default config"))
}

pub(crate) fn harness_with(config: Config) -> Harness {
    harness_with_listing(config, one_agent())
}

pub(crate) fn harness_with_missing_dam() -> Harness {
    let harness = harness();
    harness.missing.store(true, Ordering::SeqCst);
    harness
}

pub(crate) fn harness_with_listing(config: Config, listing: Vec<ListedAgent>) -> Harness {
    let herdr = RecordingHerdr {
        listing: Arc::new(Mutex::new(listing)),
        calls: Arc::default(),
    };
    let herdr_calls = Arc::clone(&herdr.calls);
    let agents = Arc::clone(&herdr.listing);
    let recorder = Recorder::default();
    let log = Arc::clone(&recorder.log);
    let senders = Arc::clone(&recorder.senders);
    let cancels = Arc::clone(&recorder.cancels);
    let missing = Arc::clone(&recorder.missing);
    let clock = StoppedClock {
        day: Arc::new(Mutex::new(parse_date(TODAY).expect("a date"))),
        now: Instant::now(),
    };
    let day = Arc::clone(&clock.day);
    let started = clock.now;
    Harness {
        app: App::new(
            config,
            Jobs::new(Box::new(recorder), Box::new(clock.clone())),
            Box::new(herdr),
            here(),
            Box::new(clock),
        ),
        started,
        day,
        log,
        senders,
        cancels,
        missing,
        herdr_calls,
        agents,
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

    pub(crate) fn put_on(&mut self, screen: Screen) {
        self.app.showing.show(screen);
    }

    pub(crate) fn open(&mut self, overlay: Overlay) {
        self.app.showing.open(overlay);
    }

    pub(crate) fn type_line(&mut self, text: &str) {
        for character in text.chars() {
            self.press(KeyCode::Char(character));
        }
    }

    pub(crate) fn list_agents(&self, listing: Vec<ListedAgent>) {
        *self.agents.lock().expect("the listing") = listing;
    }

    pub(crate) fn herdr_calls(&self) -> Vec<String> {
        self.herdr_calls.lock().expect("the calls").clone()
    }

    pub(crate) fn sent_anything(&self) -> bool {
        self.herdr_calls()
            .iter()
            .any(|call| call.starts_with("send_text"))
    }

    pub(crate) fn cancels(&self) -> usize {
        self.cancels.load(Ordering::SeqCst)
    }

    pub(crate) fn install_dam(&self) {
        self.missing.store(false, Ordering::SeqCst);
    }

    pub(crate) fn turn_the_day_to(&self, date: &str) {
        *self.day.lock().expect("the day") = parse_date(date).expect("a date");
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
