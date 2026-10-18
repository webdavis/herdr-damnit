use super::*;

mod argv;
mod placement;
mod theme;
mod views;

#[test]
fn an_empty_file_is_the_defaults() {
    let config = Config::parse("").expect("parses");
    assert_eq!(config.dam, vec!["dam".to_string()]);
    assert_eq!(config.placement, Placement::Split);
    assert_eq!(config.side, Side::Right);
    assert_eq!(config.width, None);
    assert_eq!(config.default_view, None);
    assert!(!config.auto_open);
    assert_eq!(config.icons(), IconSet::NerdFont);
    assert_eq!(config.handoff_label, "handed-off");
    assert_eq!(config.refresh_seconds(), DEFAULT_REFRESH_SECONDS);
    assert!(config.views().is_empty());
}

#[test]
fn a_stale_token_key_is_a_parse_error_because_the_token_is_dams_business_now() {
    for text in [
        r#"token_command = ["security", "find-generic-password"]"#,
        r#"token_env = "SOME_TOKEN""#,
        r#"editor = ["nvim"]"#,
    ] {
        let error = Config::parse(text).expect_err("refuses");
        assert!(error.contains("unknown field"), "{text}: {error}");
    }
}

#[test]
fn the_handoff_label_is_configurable_and_an_empty_one_turns_the_record_off() {
    assert_eq!(
        Config::parse(r#"handoff_label = "sent""#)
            .expect("parses")
            .handoff_label,
        "sent"
    );
    assert_eq!(
        Config::parse(r#"handoff_label = """#)
            .expect("parses")
            .handoff_label,
        ""
    );
}

#[test]
fn a_refresh_interval_of_zero_turns_the_interval_off_rather_than_taking_the_default() {
    assert_eq!(
        Config::parse("refresh_seconds = 0")
            .expect("parses")
            .refresh_seconds(),
        0
    );
    assert_eq!(
        Config::parse("refresh_seconds = 45")
            .expect("parses")
            .refresh_seconds(),
        45
    );
}

#[test]
fn something_that_is_not_toml_is_a_parse_error() {
    assert!(Config::parse("this is not toml").is_err());
}
