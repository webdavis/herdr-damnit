use super::*;

#[test]
fn every_command_the_plugin_manifest_runs_is_one_this_binary_knows() {
    let manifest = include_str!("../../../herdr-plugin.toml");
    let commands: Vec<&str> = manifest
        .lines()
        .filter_map(|line| {
            line.split_once("bin/herdr-damnit\\\" ")?
                .1
                .split('"')
                .next()
        })
        .collect();

    assert!(commands.contains(&"status"), "{commands:?}");
    for command in commands {
        let known = match command.split_once(' ') {
            Some(("view", number)) => number.parse::<usize>().is_ok(),
            _ => action(command).is_some(),
        };
        assert!(
            known,
            "the manifest runs `herdr-damnit {command}`, which is unknown"
        );
    }
}

#[test]
fn the_config_funnel_rejects_a_theme_the_pane_cannot_draw() {
    let error = load_with_theme_check(|| Config::parse("theme = 'unknown'"))
        .expect_err("the theme is unknown");

    assert!(error.contains("unknown theme 'unknown'"), "{error}");
}
