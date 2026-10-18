use std::path::PathBuf;
use std::time::{Duration, Instant};

use herdr_damnit_adapters::{Config, state};

pub(super) struct Rereads {
    view_request: Option<PathBuf>,
    config: Option<PathBuf>,
    last: Option<Instant>,
}

impl Rereads {
    pub(super) fn new() -> Self {
        Self {
            view_request: None,
            config: None,
            last: None,
        }
    }

    pub(super) fn watch(&mut self, view_request: PathBuf) {
        self.view_request = Some(view_request);
    }

    pub(super) fn watch_config(&mut self, config: PathBuf) {
        self.config = Some(config);
    }

    pub(super) fn config(&self) -> Option<Result<Config, String>> {
        self.config.as_deref().map(Config::load_from)
    }

    pub(super) fn requested_view(&self) -> Option<String> {
        self.view_request
            .as_deref()
            .and_then(state::take_requested_view)
    }

    pub(super) fn forget(&mut self) {
        self.last = None;
    }

    pub(super) fn restart(&mut self, now: Instant) {
        self.last = Some(now);
    }

    pub(super) fn due(&mut self, now: Instant, interval: Duration, held_back: bool) -> bool {
        let Some(since) = self.last else {
            self.last = Some(now);
            return false;
        };
        now.saturating_duration_since(since) >= interval && !held_back
    }
}
