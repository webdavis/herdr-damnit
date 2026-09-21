use super::*;

use herdr_damnit_domain::View;

#[test]
fn an_empty_file_is_the_defaults() {
    let config = Config::parse("").expect("parses");
    assert_eq!(config.dam, vec!["dam".to_string()]);
    assert_eq!(config.placement, Placement::Split);
    assert_eq!(config.side, Side::Right);
    assert_eq!(config.width, None);
    assert_eq!(config.default_view, None);
    assert!(!config.auto_open);
    assert_eq!(config.theme, None);
    assert_eq!(config.icons(), IconSet::NerdFont);
    assert_eq!(config.handoff_label, "handed-off");
    assert_eq!(config.refresh_seconds(), DEFAULT_REFRESH_SECONDS);
    assert!(config.views().is_empty());
}

#[test]
fn the_dam_argv_is_read_as_argv_so_a_path_with_a_space_stays_one_word() {
    let config = Config::parse(r#"dam = ["/opt/My Tools/dam"]"#).expect("parses");
    assert_eq!(config.dam, vec!["/opt/My Tools/dam".to_string()]);
}

#[test]
fn a_dam_argv_carries_its_own_leading_arguments() {
    let config = Config::parse(r#"dam = ["nix", "run", "nixpkgs#damnit", "--"]"#).expect("parses");
    assert_eq!(config.dam.len(), 4);
    assert_eq!(config.dam[0], "nix");
}

#[test]
fn an_empty_dam_argv_is_refused_because_nothing_could_be_spawned() {
    let error = Config::parse("dam = []").expect_err("refuses");
    assert!(error.contains("dam"), "{error}");
}

/// A blank first word could spawn nothing either, and `Command::new("")` fails at action time
/// rather than at load time, which is the wrong place to learn it.
#[test]
fn a_blank_dam_binary_is_refused_the_same_way_an_empty_argv_is() {
    let error = Config::parse(r#"dam = ["  "]"#).expect_err("refuses");
    assert!(error.contains("dam"), "{error}");
}

#[test]
fn a_view_carries_a_query_in_dams_grammar() {
    let config = Config::parse(
        "[[views]]\nname = \"today\"\nquery = \"!done & (due:today | overdue)\"\n\
         [[views]]\nname = \"deep\"\nquery = \"!done & effort:deep\"\n",
    )
    .expect("parses");

    assert_eq!(
        config.views(),
        vec![
            View {
                name: "today".to_string(),
                query: "!done & (due:today | overdue)".to_string(),
            },
            View {
                name: "deep".to_string(),
                query: "!done & effort:deep".to_string(),
            },
        ]
    );
}

#[test]
fn the_old_filter_key_is_a_parse_error_naming_the_field_rather_than_an_ignored_key() {
    let error =
        Config::parse("[[views]]\nname = \"today\"\nfilter = \"today\"\n").expect_err("refuses");
    assert!(
        error.contains("filter") || error.contains("query"),
        "{error}"
    );
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
fn two_views_with_one_name_are_a_config_error() {
    let error = Config::parse(
        "[[views]]\nname = \"today\"\nquery = \"!done\"\n\
         [[views]]\nname = \"today\"\nquery = \"done\"\n",
    )
    .expect_err("refuses");
    assert!(error.contains("two views are named 'today'"), "{error}");
}

#[test]
fn a_view_named_after_the_panes_own_open_list_is_a_config_error() {
    let error =
        Config::parse("[[views]]\nname = \"open\"\nquery = \"!done\"\n").expect_err("refuses");
    assert!(error.contains("cannot be named 'open'"), "{error}");
}

#[test]
fn a_view_missing_either_half_is_a_config_error_naming_the_field() {
    assert!(
        Config::parse("[[views]]\nname = \"today\"\n")
            .expect_err("refuses")
            .contains("query")
    );
    assert!(
        Config::parse("[[views]]\nquery = \"!done\"\n")
            .expect_err("refuses")
            .contains("name")
    );
    assert!(
        Config::parse("[[views]]\nname = \"\"\nquery = \"!done\"\n")
            .expect_err("refuses")
            .contains("needs a name")
    );
    assert!(
        Config::parse("[[views]]\nname = \"today\"\nquery = \"\"\n")
            .expect_err("refuses")
            .contains("empty query")
    );
}

#[test]
fn an_opening_view_no_view_carries_is_refused_and_names_the_views() {
    let error =
        Config::parse("default_view = \"work\"\n[[views]]\nname = \"today\"\nquery = \"!done\"\n")
            .expect_err("refuses");
    assert!(
        error.contains("default_view 'work' is not a view"),
        "{error}"
    );
    assert!(error.contains("open, today"), "{error}");
}

/// The open list is a view the config never declares, so opening on it is legal.
#[test]
fn the_panes_own_open_list_is_an_opening_view() {
    assert_eq!(
        Config::parse(r#"default_view = "open""#)
            .expect("parses")
            .default_view
            .as_deref(),
        Some("open")
    );
}

#[test]
fn a_width_outside_the_tab_is_refused_by_the_number_it_was_given() {
    for width in ["0.0", "0", "1.0", "1.5", "-0.2"] {
        let error = Config::parse(&format!("width = {width}")).expect_err("refuses");
        assert!(error.contains("share of the tab"), "{width}: {error}");
    }
    assert_eq!(
        Config::parse("width = 0.3").expect("parses").width,
        Some(0.3)
    );
}

#[test]
fn a_placement_and_a_side_are_taken_by_name_and_an_unknown_one_names_the_alternatives() {
    let error = Config::parse(r#"placement = "popup""#).expect_err("refuses");
    for word in ["overlay", "split", "tab", "zoomed"] {
        assert!(error.contains(word), "{error}");
    }
    let error = Config::parse(r#"side = "left""#).expect_err("refuses");
    assert!(error.contains("right") && error.contains("down"), "{error}");
}

/// `herdr plugin pane open --direction` splits rightward or downward only, and a same-tab
/// `herdr pane move` cannot reposition a pane afterward, so a side maps straight onto a direction.
#[test]
fn every_side_maps_onto_the_direction_herdr_splits_in() {
    for (text, direction) in [("right", "right"), ("down", "down")] {
        let config = Config::parse(&format!("side = \"{text}\"")).expect("parses");
        assert_eq!(config.side.split_direction(), direction);
    }
}

#[test]
fn every_placement_carries_the_word_herdr_takes_for_it() {
    for (text, word) in [
        ("overlay", "overlay"),
        ("split", "split"),
        ("tab", "tab"),
        ("zoomed", "zoomed"),
    ] {
        let config = Config::parse(&format!("placement = \"{text}\"")).expect("parses");
        assert_eq!(config.placement.as_str(), word);
    }
}

#[test]
fn the_marks_are_nerd_font_glyphs_until_the_plain_set_is_asked_for() {
    assert_eq!(
        Config::parse(r#"icons = "ascii""#).expect("parses").icons(),
        IconSet::Ascii
    );
    assert_eq!(
        Config::parse(r#"icons = "nerd-font""#)
            .expect("parses")
            .icons(),
        IconSet::NerdFont
    );
    assert!(
        Config::parse(r#"icons = "emoji""#)
            .expect_err("refuses")
            .contains("icons")
    );
}

/// Zero turns the interval off, leaving `R` and the read that follows every write, so it is a
/// legal value rather than a missing one.
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

/// The theme vocabulary belongs to the crate that paints, so the parse knows nothing about it and
/// the composition root checks the name against the palettes it has.
#[test]
fn a_theme_is_checked_against_the_names_the_caller_knows_rather_than_a_list_of_its_own() {
    let config = Config::parse(r#"theme = "gruvbox""#).expect("parses");
    assert!(config.check_theme_against(&["gruvbox", "nord"]).is_ok());

    let error = config
        .check_theme_against(&["nord", "onedark"])
        .expect_err("refuses");
    assert!(error.contains("unknown theme 'gruvbox'"), "{error}");
    assert!(error.contains("nord, onedark"), "{error}");
}

#[test]
fn a_config_naming_no_theme_passes_every_vocabulary() {
    assert!(
        Config::parse("")
            .expect("parses")
            .check_theme_against(&[])
            .is_ok()
    );
}

#[test]
fn something_that_is_not_toml_is_a_parse_error() {
    assert!(Config::parse("this is not toml").is_err());
}
