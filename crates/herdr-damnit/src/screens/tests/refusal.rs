use ratatui::style::Color;

use super::*;
use crate::app::tests::harness_with_missing_dam;

const NOT_ON_PATH: &str = "dam is not on PATH; install it with cargo install damnit, then press R.";

fn refused() -> crate::app::tests::Harness {
    let mut harness = harness_with_missing_dam();
    crate::open::start(&mut harness.app);
    harness
}

#[test]
fn a_narrow_refusal_is_the_header_one_cut_line_and_the_legend() {
    let drawn = render_to_text(&refused().app, 32, 10);
    let rows: Vec<&str> = drawn.lines().collect();

    assert!(rows[0].starts_with("dam"), "{drawn}");
    assert!(rows[1].ends_with(ELLIPSIS), "{drawn}");
    assert!(rows[1].chars().count() <= 32, "{drawn}");
    assert_eq!(rows[9], "q close  R retry", "{drawn}");
    assert!(rows[2..9].iter().all(|row| row.is_empty()), "{drawn}");
}

#[test]
fn a_wide_refusal_is_dams_whole_sentence_in_the_refusal_color() {
    let buffer = drawn(&refused().app, 100, 10);

    assert_eq!(line_of(&buffer, 1).trim_end(), NOT_ON_PATH);
    assert_eq!(buffer[(0, 1)].fg, Color::Rgb(243, 139, 168));
    assert_eq!(buffer[(69, 1)].fg, Color::Rgb(243, 139, 168));
    assert_eq!(line_of(&buffer, 9).trim_end(), "q close  R retry");
}
