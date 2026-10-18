use crate::config::{Config, Placement};

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

#[test]
fn a_split_or_zoomed_pane_attaches_to_a_pane_and_a_tab_or_overlay_one_to_the_workspace() {
    for (placement, attaches) in [
        (Placement::Split, true),
        (Placement::Zoomed, true),
        (Placement::Tab, false),
        (Placement::Overlay, false),
    ] {
        assert_eq!(
            placement.attaches_to_a_pane_rather_than_the_workspace(),
            attaches,
            "{placement:?}"
        );
    }
}

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
