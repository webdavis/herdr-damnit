//! The comment a person is typing: several lines, drawn in the pane rather than in an editor.
//!
//! This is the one thing the quick edits' single-line [`crate::prompt::Input`] cannot be, so it
//! is its own widget in the same prompt: `<CR>` opens a line rather than sending, and the caret
//! walks the text it has already typed.

/// A multi-line draft. The caret is a line and a column within it, so `<CR>` splits a line and a
/// backspace at the start of one joins it to the line above, which is what a hand expects.
#[derive(Debug)]
pub struct Draft {
    lines: Vec<String>,
    line: usize,
    column: usize,
}

impl Default for Draft {
    fn default() -> Self {
        Self {
            lines: vec![String::new()],
            line: 0,
            column: 0,
        }
    }
}

impl Draft {
    pub fn new() -> Self {
        Self::default()
    }

    /// Type one character. A control character is dropped rather than stored, so the key that
    /// sends the comment can never end up inside it.
    pub fn push(&mut self, character: char) {
        if character.is_control() {
            return;
        }
        let line = &mut self.lines[self.line];
        let at = byte_at(line, self.column);
        line.insert(at, character);
        self.column += 1;
    }

    /// Open a line at the caret, carrying whatever followed it onto the new line.
    pub fn newline(&mut self) {
        let at = byte_at(&self.lines[self.line], self.column);
        let tail = self.lines[self.line].split_off(at);
        self.line += 1;
        self.lines.insert(self.line, tail);
        self.column = 0;
    }

    /// Take back one character, joining this line to the one above when the caret is at its start.
    pub fn backspace(&mut self) {
        if self.column > 0 {
            let line = &mut self.lines[self.line];
            let at = byte_at(line, self.column - 1);
            line.remove(at);
            self.column -= 1;
            return;
        }
        if self.line == 0 {
            return;
        }
        let joined = self.lines.remove(self.line);
        self.line -= 1;
        self.column = self.lines[self.line].chars().count();
        self.lines[self.line].push_str(&joined);
    }

    /// The lines as typed, with the caret marked on the line it is on, which is what the pane
    /// draws.
    pub fn drawn(&self) -> Vec<String> {
        self.lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                if index == self.line {
                    let at = byte_at(line, self.column);
                    format!(" {}_{}", &line[..at], &line[at..])
                } else {
                    format!(" {line}")
                }
            })
            .collect()
    }

    /// Which drawn line the caret is on, so a draft taller than its box scrolls to it.
    pub fn caret_line(&self) -> usize {
        self.line
    }

    /// The comment the draft sends: the lines joined by newlines, which is what the API stores.
    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    /// A draft holding nothing but whitespace asks for nothing, so sending it is not a write.
    pub fn is_blank(&self) -> bool {
        self.lines.iter().all(|line| line.trim().is_empty())
    }
}

/// The byte offset of the `column`th character, which is where an insert or a remove goes: a
/// column counts characters and a `String` is indexed by bytes.
fn byte_at(line: &str, column: usize) -> usize {
    line.char_indices()
        .nth(column)
        .map_or(line.len(), |(at, _)| at)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(text: &str) -> Draft {
        let mut draft = Draft::new();
        for character in text.chars() {
            if character == '\n' {
                draft.newline();
            } else {
                draft.push(character);
            }
        }
        draft
    }

    #[test]
    fn a_new_draft_asks_for_nothing_and_draws_one_empty_line() {
        let draft = Draft::new();

        assert!(draft.is_blank());
        assert_eq!(draft.drawn(), vec![" _"]);
        assert_eq!(draft.text(), "");
    }

    #[test]
    fn several_lines_are_kept_in_order_and_sent_joined_by_newlines() {
        let draft = typed("first\nsecond\nthird");

        assert_eq!(draft.text(), "first\nsecond\nthird");
        assert_eq!(draft.drawn(), vec![" first", " second", " third_"]);
        assert_eq!(draft.caret_line(), 2);
        assert!(!draft.is_blank());
    }

    #[test]
    fn a_draft_of_nothing_but_blank_lines_asks_for_nothing() {
        assert!(typed("  \n\n \t").is_blank());
    }

    #[test]
    fn a_backspace_at_the_start_of_a_line_joins_it_to_the_one_above() {
        let mut draft = typed("first\nsecond");

        // Six take back "second" and leave the caret at the start of an empty second line; the
        // seventh is the one that joins the two lines.
        for _ in 0..6 {
            draft.backspace();
        }
        assert_eq!(draft.text(), "first\n");
        assert_eq!(draft.caret_line(), 1);

        draft.backspace();
        assert_eq!(draft.text(), "first");
        assert_eq!(draft.caret_line(), 0);
    }

    #[test]
    fn a_backspace_on_the_first_empty_line_leaves_the_draft_alone() {
        let mut draft = Draft::new();

        draft.backspace();

        assert_eq!(draft.text(), "");
        assert_eq!(draft.caret_line(), 0);
    }

    #[test]
    fn a_newline_in_the_middle_of_a_line_carries_the_rest_of_it_down() {
        let mut draft = typed("onetwo");
        for _ in 0..3 {
            draft.backspace();
        }
        draft.newline();

        assert_eq!(draft.text(), "one\n");
        assert_eq!(draft.drawn(), vec![" one", " _"]);
    }

    #[test]
    fn a_control_character_is_never_stored_so_the_send_key_cannot_land_in_the_text() {
        let mut draft = Draft::new();

        draft.push('\u{4}');
        draft.push('a');

        assert_eq!(draft.text(), "a");
    }

    #[test]
    fn a_multibyte_character_is_typed_and_taken_back_whole() {
        let mut draft = typed("café");

        assert_eq!(draft.text(), "café");
        draft.backspace();
        assert_eq!(draft.text(), "caf");
    }
}
