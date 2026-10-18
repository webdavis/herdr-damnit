use std::borrow::Cow;

use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    List,
    Status,
    Done,
    Detail,
    Picker,
    LineBox,
    NoteBox,
    Confirm(char),
    Refusal,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Need {
    Always,
    IfRoom,
}

#[derive(Clone)]
struct Entry {
    key: Cow<'static, str>,
    verb: &'static str,
    need: Need,
}

const fn if_room(key: &'static str, verb: &'static str) -> Entry {
    Entry {
        key: Cow::Borrowed(key),
        verb,
        need: Need::IfRoom,
    }
}

const fn always(key: &'static str, verb: &'static str) -> Entry {
    Entry {
        key: Cow::Borrowed(key),
        verb,
        need: Need::Always,
    }
}

const MOVE: Entry = if_room("j/k", "move");
const CLOSE: Entry = always("q", "close");
const CANCEL: Entry = always("esc", "cancel");

const LIST: [Entry; 4] = [MOVE, if_room("space", "stage"), if_room("x", "done"), CLOSE];
const STATUS: [Entry; 4] = [
    MOVE,
    if_room("space", "stage"),
    if_room("c", "commit"),
    CLOSE,
];
const DONE: [Entry; 4] = [MOVE, if_room("enter", "open"), if_room("v", "view"), CLOSE];
const DETAIL: [Entry; 2] = [if_room("esc", "back"), CLOSE];
const PICKER: [Entry; 3] = [MOVE, if_room("enter", "take"), CANCEL];
const LINE_BOX: [Entry; 2] = [if_room("enter", "send"), CANCEL];
const NOTE_BOX: [Entry; 3] = [
    if_room("enter", "new line"),
    always("ctrl+d", "send"),
    CANCEL,
];

const REFUSAL: [Entry; 2] = [CLOSE, always("R", "retry")];

const GAP: &str = "  ";

pub fn fitted(mode: Mode, width: usize) -> String {
    let mut entries = entries(mode);
    while joined(&entries).width() > width {
        match entries.iter().rposition(|entry| entry.need == Need::IfRoom) {
            Some(nearest_the_close) => entries.remove(nearest_the_close),
            None => break,
        };
    }
    joined(&entries)
}

fn entries(mode: Mode) -> Vec<Entry> {
    match mode {
        Mode::List => LIST.to_vec(),
        Mode::Status => STATUS.to_vec(),
        Mode::Done => DONE.to_vec(),
        Mode::Detail => DETAIL.to_vec(),
        Mode::Picker => PICKER.to_vec(),
        Mode::LineBox => LINE_BOX.to_vec(),
        Mode::NoteBox => NOTE_BOX.to_vec(),
        Mode::Refusal => REFUSAL.to_vec(),
        Mode::Confirm(key) => vec![
            Entry {
                key: Cow::Owned(key.to_string()),
                verb: "confirm",
                need: Need::IfRoom,
            },
            CANCEL,
        ],
    }
}

fn joined(entries: &[Entry]) -> String {
    entries
        .iter()
        .map(|entry| format!("{} {}", entry.key, entry.verb))
        .collect::<Vec<_>>()
        .join(GAP)
}

#[cfg(test)]
mod tests;
