use super::*;

const EVERY_MODE: [Mode; 9] = [
    Mode::List,
    Mode::Status,
    Mode::Done,
    Mode::Detail,
    Mode::Picker,
    Mode::LineBox,
    Mode::NoteBox,
    Mode::Confirm('d'),
    Mode::Confirm('!'),
];

#[test]
fn each_mode_names_the_keys_that_work_in_it_with_a_verb() {
    for (mode, expected) in [
        (Mode::List, "j/k move  space stage  x done  q close"),
        (Mode::Status, "j/k move  space stage  c commit  q close"),
        (Mode::Done, "j/k move  enter open  v view  q close"),
        (Mode::Detail, "esc back  q close"),
        (Mode::Picker, "j/k move  enter take  esc cancel"),
        (Mode::LineBox, "enter send  esc cancel"),
        (Mode::NoteBox, "enter new line  ctrl+d send  esc cancel"),
        (Mode::Confirm('d'), "d confirm  esc cancel"),
        (Mode::Confirm('!'), "! confirm  esc cancel"),
    ] {
        assert_eq!(fitted(mode, 70), expected, "{mode:?}");
    }
}

#[test]
fn a_narrow_pane_drops_whole_entries_before_the_close_key() {
    assert_eq!(fitted(Mode::List, 32), "j/k move  space stage  q close");
    assert_eq!(fitted(Mode::Done, 32), "j/k move  enter open  q close");
    assert_eq!(fitted(Mode::Status, 32), "j/k move  space stage  q close");
}

#[test]
fn a_narrow_note_box_keeps_the_send_key() {
    assert_eq!(fitted(Mode::NoteBox, 32), "ctrl+d send  esc cancel");
}

#[test]
fn every_legend_at_32_columns_ends_with_its_close_key_and_cuts_no_word() {
    for mode in EVERY_MODE {
        let line = fitted(mode, 32);
        let close = fitted(mode, 70)
            .rsplit("  ")
            .next()
            .unwrap_or_default()
            .to_string();
        assert!(line.ends_with(&close), "{mode:?}: {line}");
        assert!(!line.contains('\u{2026}'), "{mode:?}: {line}");
        assert!(line.chars().count() <= 32, "{mode:?}: {line}");
    }
}
