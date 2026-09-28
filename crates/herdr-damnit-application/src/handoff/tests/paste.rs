use super::*;

#[test]
fn the_brief_is_one_bracketed_paste_and_carries_no_return_so_the_operator_submits_it() {
    let herdr = FakeHerdr::new();
    hand_off(&herdr, &here(), &object(), "start here", "");

    let sent = herdr.sent_text();
    assert!(sent.starts_with("\u{1b}[200~"), "{sent:?}");
    assert!(sent.ends_with("\u{1b}[201~"), "{sent:?}");
    assert!(sent.contains("dam task: file taxes"), "{sent:?}");
    assert!(sent.contains("note: start here"), "{sent:?}");
    assert!(
        !sent.trim_end_matches("\u{1b}[201~").ends_with('\n'),
        "{sent:?}"
    );
}

#[test]
fn a_paste_terminator_inside_the_brief_cannot_end_the_frame_early() {
    let framed = as_one_bracketed_paste("before \u{1b}[201~ after");
    assert_eq!(framed.matches("\u{1b}[201~").count(), 1, "{framed:?}");
}

#[test]
fn a_terminator_spliced_together_by_removing_the_one_before_it_is_removed_too() {
    let framed = as_one_bracketed_paste("\u{1b}[20\u{1b}[201~1~");
    assert_eq!(framed.matches("\u{1b}[201~").count(), 1, "{framed:?}");
}
