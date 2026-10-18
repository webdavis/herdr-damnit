use super::*;

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
