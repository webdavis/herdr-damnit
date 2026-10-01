use std::path::{Path, PathBuf};

use crossterm::event::KeyModifiers;
use herdr_damnit_adapters::{ProcessDamRunner, SystemClock};

use super::*;
use crate::screens::render_to_text;

const FAKE_SLEEP: &str = "FAKE_DAM_SLEEP_MS=600";
const GIVE_UP: Duration = Duration::from_secs(3);
const WIDTH: u16 = 60;

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("herdr-damnit-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let fixtures =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../herdr-damnit-adapters/tests/fixtures");
        for (fixture, answers) in [
            ("push-ok", "push"),
            ("status-clean", "status"),
            ("ls", "ls"),
        ] {
            std::fs::copy(
                fixtures.join(format!("{fixture}.json")),
                dir.join(format!("{answers}.json")),
            )
            .expect("a fixture");
        }
        Self(dir)
    }

    fn log(&self) -> Vec<String> {
        std::fs::read_to_string(self.0.join("argv.jsonl"))
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fake_dam() -> PathBuf {
    let test_binary = std::env::current_exe().expect("the test binary");
    let profile = test_binary
        .parent()
        .and_then(Path::parent)
        .expect("the profile directory");
    let fake = profile.join("fake-dam");
    assert!(
        fake.exists(),
        "{} is missing: cargo test --workspace builds it",
        fake.display()
    );
    fake
}

fn app(scratch: &Scratch) -> App {
    let mut config = Config::parse("icons = \"ascii\"\n").expect("parses");
    config.dam = vec![
        fake_dam().display().to_string(),
        format!("FAKE_DAM_FIXTURE_DIR={}", scratch.0.display()),
        format!("FAKE_DAM_LOG={}", scratch.0.join("argv.jsonl").display()),
        FAKE_SLEEP.to_string(),
    ];
    let runner = ProcessDamRunner::new(config.dam.clone());
    App::new(
        config,
        Jobs::new(Box::new(runner), Box::new(SystemClock)),
        parse_date(TODAY).expect("a date"),
    )
}

/// Drives the loop the way `loop_::run` does, without a terminal, until the push is over. The
/// key goes in once the fake has logged the push, so it always lands mid-push.
fn drive(app: &mut App, scratch: &Scratch, mut key: Option<KeyEvent>) -> Vec<String> {
    let started = Instant::now();
    let mut frames = Vec::new();
    loop {
        frames.push(render_to_text(app, WIDTH, 8));
        app.tick(Instant::now());
        let pushing = app.header(Instant::now()).contains(" push ");
        if !pushing || started.elapsed() > GIVE_UP {
            frames.push(render_to_text(app, WIDTH, 8));
            return frames;
        }
        if scratch.log().iter().any(|line| line.contains("\"push\""))
            && let Some(key) = key.take()
        {
            app.key(key);
        }
        std::thread::sleep(app.poll_window());
    }
}

fn pushing(scratch: &Scratch) -> App {
    let mut app = app(scratch);
    app.key(KeyEvent::from(KeyCode::Char('P')));
    app
}

#[test]
fn the_pane_renders_while_a_push_runs_and_shows_the_summary_after_it() {
    let scratch = Scratch::new("renders-mid-push");
    let mut app = pushing(&scratch);

    let frames = drive(&mut app, &scratch, None);

    assert!(frames.len() >= 6, "it rendered {} frames", frames.len());
    let mid = &frames[frames.len() / 2];
    assert!(
        mid.lines()
            .next()
            .is_some_and(|header| header.contains("push")),
        "{mid}"
    );
    let last = frames.last().expect("a frame");
    assert!(last.contains("example: 3 sent, 3 ok"), "{last}");
    assert!(
        !last
            .lines()
            .next()
            .is_some_and(|header| header.contains("push")),
        "{last}"
    );
}

#[test]
fn a_key_pressed_mid_push_takes_effect_while_the_push_runs() {
    let scratch = Scratch::new("tab-mid-push");
    let mut app = pushing(&scratch);

    let frames = drive(&mut app, &scratch, Some(KeyEvent::from(KeyCode::Tab)));

    assert!(
        frames
            .iter()
            .any(|frame| frame.starts_with("dam  status") && frame.contains("push")),
        "the tab never took effect while the push ran"
    );
}

#[test]
fn a_second_push_mid_push_is_refused_and_the_fake_records_exactly_one() {
    let scratch = Scratch::new("second-push");
    let mut app = pushing(&scratch);

    drive(&mut app, &scratch, Some(KeyEvent::from(KeyCode::Char('P'))));

    let pushes = scratch
        .log()
        .iter()
        .filter(|line| line.contains("\"push\""))
        .count();
    assert_eq!(pushes, 1, "{:?}", scratch.log());
}

#[test]
fn control_c_mid_push_interrupts_the_fake_and_says_cancelled() {
    let scratch = Scratch::new("cancel-push");
    let mut app = pushing(&scratch);

    let control_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    let frames = drive(&mut app, &scratch, Some(control_c));

    assert!(
        scratch.log().iter().any(|line| line.contains("SIGINT")),
        "{:?}",
        scratch.log()
    );
    assert!(frames.last().expect("a frame").contains("cancelled"));
}
