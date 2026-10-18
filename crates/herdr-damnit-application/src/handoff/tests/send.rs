use super::*;

#[test]
fn a_refused_send_is_a_refusal_and_a_refused_focus_is_not_because_the_brief_already_arrived() {
    let refused_send = FakeAgents {
        refuse_send: true,
        ..FakeAgents::new()
    };
    assert!(matches!(
        hand_off(&refused_send, &here(), &object(), "", ""),
        HandOff::Refused(message) if message == "herdr refused the send to w1:p2."
    ));

    let refused_focus = FakeAgents {
        refuse_focus: true,
        ..FakeAgents::new()
    };
    assert!(matches!(
        hand_off(&refused_focus, &here(), &object(), "", ""),
        HandOff::Sent { .. }
    ));
}

#[test]
fn a_configured_label_comes_back_as_the_write_the_caller_submits() {
    let agents = FakeAgents::new();
    let HandOff::Sent { label_write, .. } = hand_off(&agents, &here(), &object(), "", "handed-off")
    else {
        panic!("expected a send");
    };

    assert_eq!(
        label_write,
        Some(vec![
            "edit".to_string(),
            "1a2b3c4".to_string(),
            "--label".to_string(),
            "handed-off".to_string(),
            "--json".to_string(),
        ])
    );
}

#[test]
fn an_empty_label_writes_nothing_and_the_status_line_is_the_whole_record() {
    let agents = FakeAgents::new();
    let HandOff::Sent { label_write, .. } = hand_off(&agents, &here(), &object(), "", "  ") else {
        panic!("expected a send");
    };
    assert_eq!(label_write, None);
}

#[test]
fn a_pane_outside_herdr_says_so_rather_than_guessing_a_workspace() {
    let agents = FakeAgents::new();
    let nowhere = Workspace {
        workspace: None,
        me: String::new(),
    };
    let HandOff::Refused(message) = hand_off(&agents, &nowhere, &object(), "", "") else {
        panic!("expected a refusal");
    };
    assert_eq!(message, "no workspace context: run this pane inside herdr.");
    assert!(agents.calls().is_empty());
}
