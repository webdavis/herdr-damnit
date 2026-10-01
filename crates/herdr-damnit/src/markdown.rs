use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

mod inline;

use inline::inline;

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

#[cfg(test)]
mod tests;
