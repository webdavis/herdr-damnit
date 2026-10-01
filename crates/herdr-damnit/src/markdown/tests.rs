use super::*;

fn texts(source: &str) -> Vec<String> {
    render(source, "")
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect()
        })
        .collect()
}

fn styled(source: &str, modifier: Modifier) -> Vec<String> {
    render(source, "")
        .iter()
        .flat_map(|line| line.spans.clone())
        .filter(|span| span.style.add_modifier.contains(modifier))
        .map(|span| span.content.to_string())
        .collect()
}

#[test]
fn a_heading_loses_its_hashes_and_is_drawn_bold() {
    assert_eq!(texts("## Shopping"), vec!["Shopping"]);
    assert_eq!(styled("## Shopping", Modifier::BOLD), vec!["Shopping"]);
}

#[test]
fn a_line_of_hashes_that_is_not_a_heading_is_left_as_written() {
    assert_eq!(texts("#hashtag"), vec!["#hashtag"]);
    assert_eq!(texts("####### too deep"), vec!["####### too deep"]);
}

#[test]
fn every_bullet_marker_draws_the_same_and_keeps_its_indent() {
    assert_eq!(
        texts("- one\n* two\n  + nested"),
        vec!["- one", "- two", "  - nested"]
    );
}

#[test]
fn a_numbered_list_keeps_the_numbers_it_was_written_with() {
    assert_eq!(texts("3. third\n4) fourth"), vec!["3. third", "4) fourth"]);
}

#[test]
fn a_quote_is_drawn_with_a_bar_and_a_rule_as_dashes() {
    assert_eq!(texts("> quoted\n\n---"), vec!["| quoted", "", "---"]);
}

#[test]
fn bold_italics_and_code_are_styled_and_lose_their_markers() {
    assert_eq!(
        texts("**loud** and _soft_ and `code`"),
        vec!["loud and soft and code"]
    );
    assert_eq!(styled("**loud** and _soft_", Modifier::BOLD), vec!["loud"]);
    assert_eq!(
        styled("**loud** and _soft_", Modifier::ITALIC),
        vec!["soft"]
    );
}

#[test]
fn a_link_draws_its_text_and_its_target_so_the_target_can_be_copied() {
    assert_eq!(
        texts("see [the docs](https://example.invalid/a/very/long/path)"),
        vec!["see the docs <https://example.invalid/a/very/long/path>"]
    );
}

#[test]
fn an_unpaired_marker_is_left_as_written_rather_than_eating_the_line() {
    assert_eq!(texts("2 * 3 = 6"), vec!["2 * 3 = 6"]);
    assert_eq!(texts("a_variable_name"), vec!["a_variable_name"]);
    assert_eq!(texts("[not a link"), vec!["[not a link"]);
}

#[test]
fn a_fenced_block_keeps_its_body_byte_for_byte_and_drops_the_fences() {
    assert_eq!(
        texts("```sh\n  **not bold**  \n```\nafter"),
        vec!["  **not bold**", "after"]
    );
}

#[test]
fn a_table_row_is_left_as_written_because_this_pane_cannot_draw_a_table() {
    assert_eq!(
        texts("| one | two |\n| --- | --- |"),
        vec!["| one | two |", "| --- | --- |"]
    );
}

#[test]
fn an_indent_is_prefixed_to_every_line() {
    assert_eq!(render("one\ntwo", "  ").len(), 2);
    let first: String = render("one", "  ")[0]
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect();
    assert_eq!(first, "  one");
}
