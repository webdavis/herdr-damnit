use herdr_damnit_application::ListedAgent;

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

const FAKE_HERDR_AT_A_HALF_SPLIT: &str = r#"#!/bin/sh
case "$2" in
  layout) echo '{"result":{"layout":{"splits":[{"ratio":0.5}]}}}' ;;
  resize) echo "$*" >> "$0.log"; echo '{"result":{"resize":{"changed":true}}}' ;;
esac
"#;

fn resize_argv_sent_for_width(name: &str, side: SplitSide, width: f32) -> String {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("herdr-damnit-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch dir");
    let script = dir.join("herdr");
    std::fs::write(&script, FAKE_HERDR_AT_A_HALF_SPLIT).expect("the fake herdr");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("executable");
    let herdr = CliHerdr::new(
        script.to_string_lossy().to_string(),
        "herdr-damnit".to_string(),
    );

    let changed = herdr.resize("w:p1", side, 1.0 - width);

    let sent = std::fs::read_to_string(dir.join("herdr.log")).expect("one resize call");
    std::fs::remove_dir_all(&dir).expect("the scratch dir removed");
    assert_eq!(changed, Ok(true));
    sent.trim().to_string()
}

#[test]
fn a_width_above_half_moves_the_divider_toward_the_calling_pane() {
    assert_eq!(
        resize_argv_sent_for_width("wide-right", SplitSide::Right, 0.7),
        "pane resize --pane w:p1 --direction left --amount 0.19999999"
    );
    assert_eq!(
        resize_argv_sent_for_width("wide-down", SplitSide::Down, 0.7),
        "pane resize --pane w:p1 --direction up --amount 0.19999999"
    );
}

#[test]
fn a_width_below_half_moves_the_divider_away_from_the_calling_pane() {
    assert_eq!(
        resize_argv_sent_for_width("narrow-right", SplitSide::Right, 0.3),
        "pane resize --pane w:p1 --direction right --amount 0.19999999"
    );
    assert_eq!(
        resize_argv_sent_for_width("narrow-down", SplitSide::Down, 0.3),
        "pane resize --pane w:p1 --direction down --amount 0.19999999"
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

#[test]
fn the_agent_verbs_list_send_the_text_as_one_word_and_focus_the_pane() {
    assert_eq!(agents::AGENT_LIST.join(" "), "agent list");
    assert_eq!(
        agents::send_text_args("w1:p2", "a brief\nnote: two words"),
        ["pane", "send-text", "w1:p2", "a brief\nnote: two words"]
    );
    assert_eq!(
        agents::focus_agent_args("w1:p2"),
        ["agent", "focus", "w1:p2"]
    );
}

#[test]
fn an_agent_listing_names_each_pane_its_workspace_kind_name_and_profile() {
    let listed = agents::agents_of_listing(AGENT_LIST_ANSWER).expect("a listing");

    assert_eq!(
        listed,
        vec![
            ListedAgent {
                pane: "w1:p1".to_string(),
                workspace: "w1".to_string(),
                kind: None,
                name: None,
                profile: None,
            },
            ListedAgent {
                pane: "w1:p2".to_string(),
                workspace: "w1".to_string(),
                kind: Some("claude".to_string()),
                name: Some("planner".to_string()),
                profile: Some("work".to_string()),
            },
        ]
    );
}

#[test]
fn a_malformed_agent_listing_is_an_error_that_names_the_verb_and_carries_the_parsers_reason() {
    let not_json = agents::agents_of_listing("not json").expect_err("refuses");
    let reason = not_json
        .strip_prefix("herdr agent list: ")
        .expect("the plugin's own prefix");
    assert!(!reason.is_empty(), "{not_json}");

    let no_result = agents::agents_of_listing("{}").expect_err("refuses");
    let reason = no_result
        .strip_prefix("herdr agent list: ")
        .expect("the plugin's own prefix");
    assert!(reason.contains("`result`"), "{no_result}");
}

const AGENT_LIST_ANSWER: &str = r#"{"result":{"agents":[
  {"pane_id":"w1:p1","workspace_id":"w1","agent":null},
  {"pane_id":"w1:p2","workspace_id":"w1","agent":"claude","name":"planner","display_agent":"work"}
]}}"#;
