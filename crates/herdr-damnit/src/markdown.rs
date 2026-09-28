use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

pub fn render(source: &str, indent: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut fenced = false;
    for raw in source.lines() {
        let trimmed = raw.trim_end();
        if is_fence(trimmed.trim_start()) {
            fenced = !fenced;
            continue;
        }
        if fenced {
            lines.push(prefixed(
                indent,
                vec![Span::styled(
                    trimmed.to_string(),
                    Style::new().add_modifier(Modifier::DIM),
                )],
            ));
            continue;
        }
        lines.push(prefixed(indent, block(trimmed)));
    }
    lines
}

fn prefixed(indent: &str, spans: Vec<Span<'static>>) -> Line<'static> {
    if indent.is_empty() {
        return Line::from(spans);
    }
    let mut all = vec![Span::raw(indent.to_string())];
    all.extend(spans);
    Line::from(all)
}

fn is_fence(line: &str) -> bool {
    line.starts_with("```") || line.starts_with("~~~")
}

fn block(line: &str) -> Vec<Span<'static>> {
    let bare = line.trim_start();
    let leading = &line[..line.len() - bare.len()];
    if bare.is_empty() {
        return Vec::new();
    }
    if is_thematic_break(bare) {
        return vec![Span::styled(
            "---".to_string(),
            Style::new().add_modifier(Modifier::DIM),
        )];
    }
    if let Some(text) = heading_text_whatever_its_level(bare) {
        return vec![Span::styled(
            text.to_string(),
            Style::new().add_modifier(Modifier::BOLD),
        )];
    }
    if let Some(text) = bare.strip_prefix("> ").or_else(|| bare.strip_prefix(">")) {
        let mut spans = vec![Span::raw(format!("{leading}| "))];
        spans.extend(inline(text.trim_start()));
        return spans;
    }
    if let Some(text) = bullet(bare) {
        let mut spans = vec![Span::raw(format!("{leading}- "))];
        spans.extend(inline(text));
        return spans;
    }
    if let Some((marker, text)) = numbered(bare) {
        let mut spans = vec![Span::raw(format!("{leading}{marker} "))];
        spans.extend(inline(text));
        return spans;
    }
    let mut spans = vec![Span::raw(leading.to_string())];
    spans.extend(inline(bare));
    spans
}

fn is_thematic_break(line: &str) -> bool {
    ["-", "*", "_"].iter().any(|character| {
        line.len() >= 3 && line.chars().all(|found| found.to_string() == *character)
    })
}

fn heading_text_whatever_its_level(line: &str) -> Option<&str> {
    let hashes = line.len() - line.trim_start_matches('#').len();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    line[hashes..].strip_prefix(' ').map(str::trim)
}

fn bullet(line: &str) -> Option<&str> {
    ["- ", "* ", "+ "]
        .iter()
        .find_map(|marker| line.strip_prefix(marker))
}

fn numbered(line: &str) -> Option<(&str, &str)> {
    let digits = line.len()
        - line
            .trim_start_matches(|found: char| found.is_ascii_digit())
            .len();
    if digits == 0 {
        return None;
    }
    let rest = &line[digits..];
    let text = rest
        .strip_prefix(". ")
        .or_else(|| rest.strip_prefix(") "))?;
    Some((&line[..digits + 1], text))
}

fn inline(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut plain = String::new();
    let mut rest = text;
    let mut preceding_character = None;
    while !rest.is_empty() {
        if let Some((span, tail)) = marked(rest, preceding_character) {
            if !plain.is_empty() {
                spans.push(Span::raw(std::mem::take(&mut plain)));
            }
            spans.push(span);
            rest = tail;
            continue;
        }
        let mut characters = rest.chars();
        if let Some(character) = characters.next() {
            plain.push(character);
            preceding_character = Some(character);
        }
        rest = characters.as_str();
    }
    if !plain.is_empty() {
        spans.push(Span::raw(plain));
    }
    spans
}

fn marked(text: &str, preceding_character: Option<char>) -> Option<(Span<'static>, &str)> {
    if let Some(rest) = text.strip_prefix('[')
        && let Some((label, after)) = rest.split_once("](")
        && let Some((target, tail)) = after.split_once(')')
    {
        return Some((
            Span::styled(
                format!("{label} <{target}>"),
                Style::new().add_modifier(Modifier::UNDERLINED),
            ),
            tail,
        ));
    }
    for (fence, modifier) in [
        ("**", Modifier::BOLD),
        ("__", Modifier::BOLD),
        ("`", Modifier::DIM),
        ("*", Modifier::ITALIC),
        ("_", Modifier::ITALIC),
    ] {
        if let Some(rest) = text.strip_prefix(fence)
            && let Some((inner, tail)) = rest.split_once(fence)
            && !inner.is_empty()
            && (!fence.starts_with('_') || !underscore_sits_inside_a_word(preceding_character, tail))
        {
            return Some((
                Span::styled(inner.to_string(), Style::new().add_modifier(modifier)),
                tail,
            ));
        }
    }
    None
}

fn underscore_sits_inside_a_word(preceding_character: Option<char>, tail: &str) -> bool {
    let alphanumeric = |character: char| character.is_alphanumeric();
    preceding_character.is_some_and(alphanumeric) || tail.chars().next().is_some_and(alphanumeric)
}

#[cfg(test)]
mod tests {
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
}
