use crate::screens::tests::{ascii_config, loaded, loaded_with};

use super::non_blocking::fake_dam;
use super::*;

#[test]
fn e_hands_the_loop_the_editor_argv_and_spawns_nothing_itself() {
    let mut harness = loaded();
    harness.select("1a2b3c4");
    let before = harness.lines().len();

    let after = harness.press(KeyCode::Char('e'));

    assert_eq!(
        after,
        After::Editor(vec![
            "edit".to_string(),
            "1a2b3c4".to_string(),
            "-e".to_string()
        ])
    );
    assert_eq!(harness.lines().len(), before, "the key spawned dam itself");
}

#[test]
fn e_with_no_object_under_the_cursor_stays_put() {
    let mut harness = harness();
    assert_eq!(harness.press(KeyCode::Char('e')), After::Stay);
}

#[test]
fn a_finished_editor_leaves_the_pane_reading_again() {
    let log = std::env::temp_dir().join(format!("herdr-damnit-editor-{}", std::process::id()));
    let _ = std::fs::remove_file(&log);
    let mut config = ascii_config();
    config.dam = vec![
        fake_dam().display().to_string(),
        format!("FAKE_DAM_LOG={}", log.display()),
        "FAKE_DAM_EXIT=0".to_string(),
    ];
    let mut harness = loaded_with(config);
    let before = harness.lines().len();

    crate::editor::run_and_reread(&mut harness.app, &argv(&["edit", "1a2b3c4", "-e"]));

    let ran = std::fs::read_to_string(&log).unwrap_or_default();
    let _ = std::fs::remove_file(&log);
    assert_eq!(ran.trim(), r#"["edit","1a2b3c4","-e"]"#);
    let asked = &harness.lines()[before..];
    assert!(asked.contains(&"ls !done --json".to_string()), "{asked:?}");
    assert!(asked.contains(&"status --json".to_string()), "{asked:?}");
}

#[test]
fn a_dam_that_could_not_be_started_still_leaves_the_pane_reading_again() {
    let mut config = ascii_config();
    config.dam = vec!["no-such-dam-anywhere".to_string()];
    let mut harness = loaded_with(config);
    let before = harness.lines().len();

    crate::editor::run_and_reread(&mut harness.app, &argv(&["edit", "x", "-e"]));

    assert_eq!(
        harness.app.message,
        "dam is not on PATH; install it with cargo install damnit, then press R."
    );
    assert!(harness.lines().len() > before, "the pane did not re-read");
}

fn argv(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| word.to_string()).collect()
}
