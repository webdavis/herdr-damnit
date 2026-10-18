use std::collections::HashMap;
use std::time::{Duration, Instant};

use herdr_damnit_application::{Completion, JobId, Jobs, Request, Submitted, SyncKind};
use herdr_damnit_domain::IconSet;

use super::Pending;

const BRAILLE_SPINNER: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
const PLAIN_SPINNER: [&str; 4] = ["|", "/", "-", "\\"];

pub(super) struct Work {
    jobs: Jobs,
    pending: HashMap<JobId, Pending>,
    spinner: usize,
}

impl Work {
    pub(super) fn new(jobs: Jobs) -> Self {
        Self {
            jobs,
            pending: HashMap::new(),
            spinner: 0,
        }
    }

    pub(super) fn submit(&mut self, request: Request) -> Submitted {
        self.jobs.submit(request)
    }

    pub(super) fn pend(&mut self, id: JobId, pending: Pending) {
        self.pending.insert(id, pending);
    }

    pub(super) fn settle(&mut self, id: &JobId) -> Option<Pending> {
        self.pending.remove(id)
    }

    #[cfg(test)]
    pub(super) fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub(super) fn in_flight(&self) -> usize {
        self.jobs.in_flight()
    }

    pub(super) fn exclusive(&self) -> Option<SyncKind> {
        self.jobs.exclusive()
    }

    pub(super) fn cancel_current(&mut self) {
        self.jobs.cancel_current();
    }

    pub(super) fn turn(&mut self) -> Vec<Completion> {
        if self.jobs.in_flight() > 0 {
            self.spinner = self.spinner.wrapping_add(1);
        }
        self.jobs.drain()
    }

    pub(super) fn activity(&self, now: Instant, icons: IconSet) -> Option<String> {
        let elapsed = self.jobs.elapsed_of_current(now)?;
        let reads = self.jobs.in_flight();
        let what = match self.jobs.exclusive() {
            Some(sync) => sync.verb().to_string(),
            None => format!("{reads} read{}", if reads == 1 { "" } else { "s" }),
        };
        Some(format!(
            "{} {what} {}",
            self.frame(icons),
            tenths_under_ten_seconds_then_whole_seconds(elapsed)
        ))
    }

    fn frame(&self, icons: IconSet) -> &'static str {
        match icons {
            IconSet::NerdFont => BRAILLE_SPINNER[self.spinner % BRAILLE_SPINNER.len()],
            IconSet::Ascii => PLAIN_SPINNER[self.spinner % PLAIN_SPINNER.len()],
        }
    }
}

fn tenths_under_ten_seconds_then_whole_seconds(elapsed: Duration) -> String {
    match elapsed.as_secs() < 10 {
        true => format!("{:.1}s", elapsed.as_secs_f32()),
        false => format!("{}s", elapsed.as_secs()),
    }
}
