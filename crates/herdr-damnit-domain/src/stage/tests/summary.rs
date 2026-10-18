use super::*;

#[test]
fn the_summary_counts_what_the_status_line_carries() {
    assert_eq!(
        full().summary(),
        "2 staged  1 changed  3 unpushed  1 notice"
    );
    assert_eq!(
        full().unpushed_commits(),
        3,
        "two remotes, one and two commits"
    );
}

#[test]
fn the_summary_leaves_out_every_zero_count() {
    let staged_only = Stage {
        staged: full().staged,
        ..Stage::default()
    };
    assert_eq!(staged_only.summary(), "2 staged");
}

#[test]
fn a_clean_stage_summarizes_to_nothing() {
    assert_eq!(Stage::default().summary(), "");
}

#[test]
fn two_notices_are_spelled_in_the_plural() {
    let notices = Stage {
        notices: [full().notices, full().notices].concat(),
        ..Stage::default()
    };
    assert_eq!(notices.summary(), "2 notices");
}
