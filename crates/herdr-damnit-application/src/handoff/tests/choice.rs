use super::*;

fn sent_to(agents: &FakeAgents, here: &Workspace) -> Agent {
    match hand_off(agents, here, &object(), "", "") {
        HandOff::Sent { agent, .. } => agent,
        HandOff::Refused(message) => panic!("expected a send, got {message}"),
    }
}

fn refusal(agents: &FakeAgents, here: &Workspace) -> String {
    match hand_off(agents, here, &object(), "", "") {
        HandOff::Refused(message) => message,
        HandOff::Sent { agent, .. } => panic!("expected a refusal, sent to {}", agent.pane),
    }
}

fn workspace(workspace: &str, me: &str) -> Workspace {
    Workspace {
        workspace: Some(workspace.to_string()),
        me: me.to_string(),
    }
}

#[test]
fn the_brief_goes_to_the_named_agent_of_this_workspace() {
    let agents = FakeAgents::new();
    let agent = sent_to(&agents, &here());

    assert_eq!(agent.pane, "w1:p2");
    assert_eq!(agent.name, "planner");
    assert_eq!(agents.calls(), vec!["agents", "send_text", "focus_agent"]);
}

#[test]
fn an_agent_pane_in_another_workspace_is_not_a_candidate() {
    let message = refusal(&FakeAgents::new(), &workspace("w3", "w3:p1"));
    assert_eq!(message, "no agent pane in this workspace.");
}

#[test]
fn a_pane_herdr_lists_no_agent_for_is_passed_over_for_the_one_it_does() {
    assert_ne!(sent_to(&FakeAgents::new(), &here()).pane, "w1:p1");
}

#[test]
fn a_workspace_with_no_agent_pane_says_so_and_sends_nothing() {
    let agents = FakeAgents::listing(Vec::new());

    assert_eq!(
        refusal(&agents, &here()),
        "no agent pane in this workspace."
    );
    assert_eq!(agents.calls(), vec!["agents"]);
}

#[test]
fn this_pane_is_never_its_own_agent() {
    let message = refusal(&FakeAgents::new(), &workspace("w1", "w1:p2"));
    assert_eq!(message, "no agent pane in this workspace.");
}

#[test]
fn a_listing_herdr_could_not_give_is_refused_with_its_reason_rather_than_guessed_at() {
    let agents = FakeAgents {
        agents: Err("herdr agent list: expected value at line 1 column 1".to_string()),
        ..FakeAgents::new()
    };
    assert_eq!(
        refusal(&agents, &here()),
        "herdr agent list: expected value at line 1 column 1"
    );
}

#[test]
fn an_agent_pane_with_no_name_of_its_own_is_called_by_its_kind() {
    let agents = FakeAgents::listing(vec![agent("w1:p7", "w1", Some("codex"))]);
    assert_eq!(sent_to(&agents, &here()).name, "codex");
}

#[test]
fn the_agent_kind_outranks_the_auth_profile_herdr_lists_as_display_agent() {
    let agents = FakeAgents::listing(vec![ListedAgent {
        profile: Some("work".to_string()),
        ..agent("w1:p7", "w1", Some("codex"))
    }]);
    assert_eq!(sent_to(&agents, &here()).name, "codex");
}

#[test]
fn with_two_agent_panes_in_this_workspace_the_first_herdr_lists_wins() {
    let agents = FakeAgents::listing(vec![
        agent("w1:p3", "w1", Some("codex")),
        agent("w1:p4", "w1", Some("claude")),
    ]);
    assert_eq!(sent_to(&agents, &here()).pane, "w1:p3");
}

#[test]
fn an_agent_pane_that_names_nothing_at_all_is_called_the_agent() {
    let agents = FakeAgents::listing(vec![agent("w1:p7", "w1", Some(""))]);
    assert_eq!(sent_to(&agents, &here()).name, "the agent");
}
