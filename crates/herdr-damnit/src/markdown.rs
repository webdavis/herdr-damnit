//! The markdown a task's description and a comment's body are written in, turned into styled
//! lines the pane draws.
//!
//! A deliberately small subset, because a Todoist description is free text a person typed on
//! their phone and the pane it lands in is about 32 columns wide. Block structure is resolved
//! here (headings, lists, quotes, rules, fenced code) and so is inline emphasis, while the
//! WRAPPING is left to the paragraph widget that draws these lines: it breaks on a word boundary
//! and splits a token longer than the pane, which is what keeps a URL or a table row inside the
//! pane instead of running off it.
//!
//! What is shown as written rather than rendered: a table, because 32 columns cannot hold one,
//! and the body of a fenced code block, because reflowing code changes what it says. Both wrap.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

/// Render markdown into lines. `indent` is prefixed to every line, which is how a comment's body
/// sits under its header.
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
            // Code is kept byte for byte, dim so it reads as a block, and never re-flowed by the
            // renderer: only the paragraph widget's own wrap touches it.
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

/// A fence opens or closes a code block. Both spellings the syntax allows are recognised.
fn is_fence(line: &str) -> bool {
    line.starts_with("```") || line.starts_with("~~~")
}

/// One block-level line: its markers turned into what the pane draws, and its text into spans.
fn block(line: &str) -> Vec<Span<'static>> {
    let bare = line.trim_start();
    let leading = &line[..line.len() - bare.len()];
    if bare.is_empty() {
        return Vec::new();
    }
    if is_rule(bare) {
        return vec![Span::styled(
            "---".to_string(),
            Style::new().add_modifier(Modifier::DIM),
        )];
    }
    if let Some(text) = heading(bare) {
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

/// A thematic break: three or more of one of the rule characters, and nothing else.
fn is_rule(line: &str) -> bool {
    ["-", "*", "_"].iter().any(|character| {
        line.len() >= 3 && line.chars().all(|found| found.to_string() == *character)
    })
}

/// An ATX heading's text, with its hashes and its level dropped: a pane this narrow has no room
/// to draw six levels differently, so every heading is one bold line.
fn heading(line: &str) -> Option<&str> {
    let hashes = line.len() - line.trim_start_matches('#').len();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    line[hashes..].strip_prefix(' ').map(str::trim)
}

/// A bullet's text. The marker is normalised to one dash, so a list written with any of the three
/// markers draws the same.
fn bullet(line: &str) -> Option<&str> {
    ["- ", "* ", "+ "]
        .iter()
        .find_map(|marker| line.strip_prefix(marker))
}

/// A numbered item's own marker and text. The number written is kept, because a list that starts
/// at 3 was written that way on purpose.
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

/// Inline markup: bold, italics, code and links. Everything else, `|` table pipes included, is
/// left as it was typed.
fn inline(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut plain = String::new();
    let mut rest = text;
    let mut preceding = None;
    while !rest.is_empty() {
        if let Some((span, tail)) = marked(rest, preceding) {
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
            preceding = Some(character);
        }
        rest = characters.as_str();
    }
    if !plain.is_empty() {
        spans.push(Span::raw(plain));
    }
    spans
}

/// One inline marker at the head of `text`, as the span it draws and what follows it. A link
/// becomes its text followed by its target in brackets, because a pane cannot be clicked and the
/// target is the half a reader has to be able to copy. `preceding` is the character already
/// drawn, which is what tells an underscore inside a word from one opening emphasis.
fn marked(text: &str, preceding: Option<char>) -> Option<(Span<'static>, &str)> {
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
            && (!fence.starts_with('_') || !intraword(preceding, tail))
        {
            return Some((
                Span::styled(inner.to_string(), Style::new().add_modifier(modifier)),
                tail,
            ));
        }
    }
    None
}

/// Whether an underscore run sits inside a word, which the syntax does not read as emphasis: a
/// name like `a_variable_name` is one word, not an italic in the middle of one.
fn intraword(preceding: Option<char>, tail: &str) -> bool {
    let alphanumeric = |character: char| character.is_alphanumeric();
    preceding.is_some_and(alphanumeric) || tail.chars().next().is_some_and(alphanumeric)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text of every rendered line, with its indent, so a case reads as what the pane draws.
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

    /// Every span carrying `modifier`, in order.
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
