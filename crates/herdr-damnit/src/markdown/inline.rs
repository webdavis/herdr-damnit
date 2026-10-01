use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

pub(super) fn inline(text: &str) -> Vec<Span<'static>> {
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
            && (!fence.starts_with('_')
                || !underscore_sits_inside_a_word(preceding_character, tail))
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
