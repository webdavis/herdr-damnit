use std::sync::{Arc, Mutex};

use crate::{AgentPanes, ListedAgent};

#[derive(Clone)]
pub struct FakeAgents {
    listing: Arc<Mutex<Result<Vec<ListedAgent>, String>>>,
    refuse_send: bool,
    refuse_focus: bool,
    calls: Arc<Mutex<Vec<String>>>,
    sent: Arc<Mutex<String>>,
}

impl FakeAgents {
    pub fn listing(agents: Vec<ListedAgent>) -> Self {
        Self {
            listing: Arc::new(Mutex::new(Ok(agents))),
            refuse_send: false,
            refuse_focus: false,
            calls: Arc::default(),
            sent: Arc::default(),
        }
    }

    pub fn failing(reason: &str) -> Self {
        let agents = Self::listing(Vec::new());
        *agents.listing.lock().expect("the listing") = Err(reason.to_string());
        agents
    }

    pub fn refusing_send(self) -> Self {
        Self {
            refuse_send: true,
            ..self
        }
    }

    pub fn refusing_focus(self) -> Self {
        Self {
            refuse_focus: true,
            ..self
        }
    }

    pub fn list(&self, agents: Vec<ListedAgent>) {
        *self.listing.lock().expect("the listing") = Ok(agents);
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().expect("the calls").clone()
    }

    pub fn sent_text(&self) -> String {
        self.sent.lock().expect("the text").clone()
    }

    fn record(&self, call: String, refused: bool) -> Result<(), String> {
        self.calls.lock().expect("the calls").push(call);
        match refused {
            true => Err("refused".to_string()),
            false => Ok(()),
        }
    }
}

impl AgentPanes for FakeAgents {
    fn agents(&self) -> Result<Vec<ListedAgent>, String> {
        self.calls
            .lock()
            .expect("the calls")
            .push("agents".to_string());
        self.listing.lock().expect("the listing").clone()
    }

    fn send_text(&self, pane: &str, text: &str) -> Result<(), String> {
        *self.sent.lock().expect("the text") = text.to_string();
        self.record(format!("send_text {pane} {text}"), self.refuse_send)
    }

    fn focus_agent(&self, pane: &str) -> Result<(), String> {
        self.record(format!("focus_agent {pane}"), self.refuse_focus)
    }
}
