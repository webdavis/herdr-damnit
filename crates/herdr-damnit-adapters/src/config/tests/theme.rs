use herdr_damnit_domain::IconSet;

use crate::config::Config;

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

#[test]
fn the_theme_is_catppuccin_mocha_by_name_and_by_default() {
    assert_eq!(
        Config::parse("").expect("parses"),
        Config::parse(r#"theme = "catppuccin-mocha""#).expect("parses")
    );
}

#[test]
fn a_theme_the_pane_does_not_have_is_refused_naming_the_one_it_has() {
    for name in ["dracula", "catppuccin"] {
        let error = Config::parse(&format!("theme = \"{name}\"")).expect_err("refuses");
        assert!(error.contains("catppuccin-mocha"), "{error}");
    }
}
