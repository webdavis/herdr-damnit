use super::*;

fn priority(value: u8) -> Priority {
    Priority::new(value).expect("a priority")
}

#[test]
fn priority_one_is_the_urgent_one_and_four_carries_no_mark() {
    assert_eq!(
        Mark::of_priority(priority(1)).map(Mark::slot),
        Some(Slot::Red)
    );
    assert_eq!(
        Mark::of_priority(priority(2)).map(Mark::slot),
        Some(Slot::Orange)
    );
    assert_eq!(
        Mark::of_priority(priority(3)).map(Mark::slot),
        Some(Slot::Blue)
    );
    assert_eq!(Mark::of_priority(priority(4)), None);
}

#[test]
fn the_plain_set_tells_the_three_priorities_apart_by_character() {
    let plain = |value: u8| {
        Mark::of_priority(priority(value))
            .expect("a mark")
            .glyph(IconSet::Ascii)
    };
    assert_eq!(plain(1), "!");
    assert_eq!(plain(2), "^");
    assert_eq!(plain(3), "-");
}

#[test]
fn the_four_staging_marks_have_their_own_characters_and_colours() {
    for (mark, glyph, slot) in [
        (Mark::Working, "*", Slot::Yellow),
        (Mark::Staged, "+", Slot::Green),
        (Mark::Unpushed, "^", Slot::Cyan),
        (Mark::Conflict, "x", Slot::Red),
    ] {
        assert_eq!(mark.glyph(IconSet::Ascii), glyph);
        assert_eq!(mark.slot(), slot);
    }
}

#[test]
fn a_notice_is_a_decision_waiting_so_it_draws_below_red_and_the_same_bang_in_either_set() {
    assert_eq!(Mark::Notice.glyph(IconSet::Ascii), "!");
    assert_eq!(Mark::Notice.glyph(IconSet::NerdFont), "!");
    assert_eq!(Mark::Notice.slot(), Slot::Orange);
}

#[test]
fn a_label_count_is_the_sigil_and_the_number() {
    assert_eq!(Mark::Labels(2).glyph(IconSet::Ascii), "@2");
    assert_eq!(Mark::Labels(0).glyph(IconSet::Ascii), "@0");
    assert_eq!(Mark::Labels(2).glyph(IconSet::NerdFont), "\u{f02c}2");
}

#[test]
fn every_nerd_font_mark_draws_the_font_awesome_codepoint_the_spec_names() {
    for (mark, glyph) in [
        (Mark::Priority(priority(1)), "\u{f024}"),
        (Mark::Overdue, "\u{f071}"),
        (Mark::Today, "\u{f017}"),
        (Mark::Upcoming, "\u{f073}"),
        (Mark::Recurring, "\u{f021}"),
        (Mark::Working, "\u{f040}"),
        (Mark::Staged, "\u{f067}"),
        (Mark::Unpushed, "\u{f062}"),
        (Mark::Conflict, "\u{f00d}"),
    ] {
        assert_eq!(mark.glyph(IconSet::NerdFont), glyph, "{mark:?}");
    }
}

#[test]
fn a_due_state_takes_its_own_mark_except_when_there_is_no_date() {
    assert_eq!(Mark::of_due(DueState::Overdue), Some(Mark::Overdue));
    assert_eq!(Mark::of_due(DueState::Today), Some(Mark::Today));
    assert_eq!(Mark::of_due(DueState::Upcoming), Some(Mark::Upcoming));
    assert_eq!(Mark::of_due(DueState::None), None);
}

#[test]
fn every_nerd_font_glyph_is_one_character_so_a_row_is_measured_by_counting_marks() {
    for mark in [
        Mark::Priority(priority(1)),
        Mark::Overdue,
        Mark::Today,
        Mark::Upcoming,
        Mark::Recurring,
        Mark::Working,
        Mark::Staged,
        Mark::Unpushed,
        Mark::Conflict,
        Mark::Notice,
    ] {
        let glyph = mark.glyph(IconSet::NerdFont);
        assert_eq!(glyph.chars().count(), 1, "{mark:?} drew {glyph:?}");
    }
}
