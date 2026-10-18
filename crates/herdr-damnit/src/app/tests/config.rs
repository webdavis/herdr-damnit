use std::path::PathBuf;

use crate::screens::tests::loaded_with;

use super::*;

const TODAY_VIEW: &str = "icons = \"ascii\"\n[[views]]\nname = \"today\"\nquery = \"due:today\"\n";

const LATER_VIEW: &str = "icons = \"ascii\"\n[[views]]\nname = \"later\"\nquery = \"due:later\"\n";

fn config_file(name: &str, text: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("herdr-damnit-{name}-{}.toml", std::process::id()));
    std::fs::write(&path, text).expect("the config file");
    path
}

fn views_of(harness: &Harness) -> Vec<String> {
    harness
        .app
        .list()
        .views()
        .names()
        .map(str::to_string)
        .collect()
}

#[test]
fn r_reads_the_config_again_and_draws_the_new_views() {
    let path = config_file("reread", TODAY_VIEW);
    let mut harness = loaded_with(Config::load_from(&path).expect("parses"));
    harness.app.watch_config(path.clone());
    harness.press(KeyCode::Char('2'));
    std::fs::write(&path, LATER_VIEW).expect("the changed config");

    harness.press(KeyCode::Char('R'));

    assert_eq!(views_of(&harness), ["open", "later"]);
    assert_eq!(harness.app.list().views().current().name, "open");
    let reads = &harness.lines()[harness.lines().len() - 2..];
    assert_eq!(reads, ["ls !done --json", "status --json"]);
    std::fs::remove_file(path).expect("removed");
}

#[test]
fn r_keeps_the_showing_view_when_the_new_config_still_has_it() {
    let path = config_file("keep", TODAY_VIEW);
    let mut harness = loaded_with(Config::load_from(&path).expect("parses"));
    harness.app.watch_config(path.clone());
    harness.press(KeyCode::Char('2'));
    std::fs::write(
        &path,
        format!("{LATER_VIEW}[[views]]\nname = \"today\"\nquery = \"due:today\"\n"),
    )
    .expect("the changed config");

    harness.press(KeyCode::Char('R'));

    assert_eq!(views_of(&harness), ["open", "later", "today"]);
    assert_eq!(harness.app.list().views().current().name, "today");
    let reads = &harness.lines()[harness.lines().len() - 2..];
    assert_eq!(reads, ["ls due:today --json", "status --json"]);
    std::fs::remove_file(path).expect("removed");
}

#[test]
fn a_config_that_no_longer_parses_is_said_on_the_bottom_line_and_the_views_stay() {
    let path = config_file("broken", TODAY_VIEW);
    let mut harness = loaded_with(Config::load_from(&path).expect("parses"));
    harness.app.watch_config(path.clone());
    std::fs::write(&path, "width = 2\n").expect("the broken config");

    harness.press(KeyCode::Char('R'));

    assert!(
        harness.app.message().contains("share of the tab"),
        "{}",
        harness.app.message()
    );
    assert_eq!(views_of(&harness), ["open", "today"]);
    std::fs::remove_file(path).expect("removed");
}
