use crate::config::Config;

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
