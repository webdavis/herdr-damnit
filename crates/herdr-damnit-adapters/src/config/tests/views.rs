use herdr_damnit_domain::View;

use crate::config::Config;

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
