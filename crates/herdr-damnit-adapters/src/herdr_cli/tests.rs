use super::*;

fn opening(placement: PanePlacement, neighbor: &str) -> PaneOpen {
    PaneOpen {
        workspace: "w".to_string(),
        neighbor: neighbor.to_string(),
        placement,
        focus: PaneFocus::Take,
    }
}

#[test]
fn a_split_opens_beside_the_calling_pane_in_the_configured_direction() {
    let open = opening(PanePlacement::Split(SplitSide::Down), "w:p1");

    assert_eq!(
        open_args("herdr-damnit", &open).join(" "),
        "plugin pane open --plugin herdr-damnit --entrypoint pane --placement split --focus \
         --target-pane w:p1 --direction down"
    );
}

#[test]
fn a_zoomed_pane_attaches_to_the_caller_and_a_tab_to_the_workspace() {
    let zoomed = opening(PanePlacement::Zoomed, "w:p1");
    let tab = opening(PanePlacement::Tab, "w:p1");

    assert!(open_args("herdr-damnit", &zoomed).ends_with(&["--target-pane", "w:p1"]));
    assert!(open_args("herdr-damnit", &tab).ends_with(&["--workspace", "w"]));
}

#[test]
fn without_a_calling_pane_a_split_opens_in_the_workspace_without_the_focus() {
    let mut open = opening(PanePlacement::Split(SplitSide::Right), "");
    open.focus = PaneFocus::Leave;

    assert_eq!(
        open_args("herdr-damnit", &open).join(" "),
        "plugin pane open --plugin herdr-damnit --entrypoint pane --placement split --no-focus \
         --workspace w"
    );
}

#[test]
fn the_opened_pane_id_is_read_from_the_open_envelope() {
    let output = r#"{"result":{"plugin_pane":{"pane":{"pane_id":"w:p7","tab_id":"w:t1"}}}}"#;
    assert_eq!(pane_id_of_open(output), Some("w:p7".to_string()));
    assert_eq!(pane_id_of_open("{}"), None);
}

#[test]
fn a_resize_carries_the_pane_the_direction_and_the_rendered_amount() {
    assert_eq!(
        resize_args("w:p1", "right", 0.2).join(" "),
        "pane resize --pane w:p1 --direction right --amount 0.2"
    );
}

#[test]
fn the_current_ratio_is_read_from_the_layouts_leading_split() {
    let output = r#"{"result":{"layout":{"splits":[{"ratio":0.5}]}}}"#;
    assert_eq!(ratio_of_layout(output), Some(0.5));
    assert_eq!(ratio_of_layout("{}"), None);
}

#[test]
fn the_resize_outcome_is_read_from_its_own_changed_flag() {
    let changed = r#"{"result":{"resize":{"changed":true}}}"#;
    let unchanged = r#"{"result":{"resize":{"changed":false,"reason":"unchanged"}}}"#;
    assert!(resize_changed(changed));
    assert!(!resize_changed(unchanged));
    assert!(!resize_changed("{}"));
}
