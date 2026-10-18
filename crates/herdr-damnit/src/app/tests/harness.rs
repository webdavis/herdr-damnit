use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};
use herdr_damnit_adapters::Config;
use herdr_damnit_application::fakes::{FakeAgents, FakeClock, FakeDam};
use herdr_damnit_application::{Finished, Jobs, ListedAgent, SpawnError, Workspace};
use herdr_damnit_domain::Cursor;

use crate::app::{After, App, Screen};
use crate::overlay::Overlay;

const TODAY: &str = "2026-09-20";

pub(crate) fn here() -> Workspace {
    Workspace {
        workspace: Some("w1".to_string()),
        me: "w1:p9".to_string(),
    }
}

fn one_agent() -> Vec<ListedAgent> {
    vec![ListedAgent {
        pane: "w1:p2".to_string(),
        workspace: "w1".to_string(),
        kind: Some("claude".to_string()),
        name: Some("planner".to_string()),
        profile: None,
    }]
}

pub(crate) struct Harness {
    pub(crate) app: App,
    clock: FakeClock,
    dam: FakeDam,
    herdr: FakeAgents,
}

pub(crate) fn harness() -> Harness {
    harness_with(Config::parse("").expect("the default config"))
}

pub(crate) fn harness_with(config: Config) -> Harness {
    harness_with_listing(config, one_agent())
}

pub(crate) fn harness_with_missing_dam() -> Harness {
    let harness = harness();
    harness.dam.refuse(SpawnError::NotFound);
    harness
}

pub(crate) fn harness_with_listing(config: Config, listing: Vec<ListedAgent>) -> Harness {
    let herdr = FakeAgents::listing(listing);
    let dam = FakeDam::default();
    let clock = FakeClock::on(TODAY);
    Harness {
        app: App::new(
            config,
            Jobs::new(Box::new(dam.clone()), Box::new(clock.clone())),
            Box::new(herdr.clone()),
            here(),
            Box::new(clock.clone()),
        ),
        clock,
        dam,
        herdr,
    }
}

impl Harness {
    pub(crate) fn started(&self) -> Instant {
        self.clock.started()
    }

    pub(crate) fn answer(&mut self, starting_with: &[&str], code: i32, stdout: &str, stderr: &str) {
        self.dam.answer(
            starting_with,
            Finished {
                code: Some(code),
                stdout: stdout.to_string(),
                stderr: stderr.to_string(),
            },
        );
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

    pub(crate) fn cursor_mut(&mut self) -> &mut Cursor {
        self.app.cursor_mut()
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
        self.herdr.list(listing);
    }

    pub(crate) fn herdr_calls(&self) -> Vec<String> {
        self.herdr.calls()
    }

    pub(crate) fn sent_anything(&self) -> bool {
        self.herdr_calls()
            .iter()
            .any(|call| call.starts_with("send_text"))
    }

    pub(crate) fn hang_up(&mut self) {
        self.dam.hang_up();
        self.app.tick(Instant::now());
    }

    pub(crate) fn cancels(&self) -> usize {
        self.dam.cancelled().len()
    }

    pub(crate) fn install_dam(&self) {
        self.dam.accept();
    }

    pub(crate) fn turn_the_day_to(&self, date: &str) {
        self.clock.turn_to(date);
    }

    pub(crate) fn log_argv_last(&self) -> Vec<String> {
        self.dam.spawned().last().cloned().unwrap_or_default()
    }

    pub(crate) fn lines(&self) -> Vec<String> {
        self.dam.lines()
    }
}
