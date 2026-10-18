use herdr_damnit_domain::{Failure, Oid, message};

#[derive(Default)]
pub(super) struct Message(String);

impl Message {
    pub(super) fn none() -> Self {
        Self::default()
    }

    pub(super) fn said(text: String) -> Self {
        Self(text)
    }

    pub(super) fn failed(failure: &Failure) -> Self {
        Self(message(failure))
    }

    pub(super) fn made(oid: &Oid) -> Self {
        Self(format!("made {}", oid.short()))
    }

    pub(super) fn sent_to(agent: &str) -> Self {
        Self(format!("sent to {agent}"))
    }

    pub(super) fn label_refused(agent: &str, failure: &Failure) -> Self {
        Self(format!(
            "sent to {agent}, label refused: {}.",
            message(failure)
        ))
    }

    pub(super) fn nothing_staged() -> Self {
        Self("nothing staged; press <Space> on a row or A to stage everything.".to_string())
    }

    pub(super) fn no_such_view(count: usize) -> Self {
        Self(format!("the config has {count} views."))
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}
