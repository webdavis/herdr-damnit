//! The status line's left half: what the pane is doing, the spinner while it does it, and how
//! long it has been doing it for.

use std::time::{Duration, Instant};

use herdr_damnit_domain::IconSet;

use super::App;

const BRAILLE: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
const PLAIN: [&str; 4] = ["|", "/", "-", "\\"];

impl App {
    /// The header's left half. With an exclusive job running it is named; with only reads in
    /// flight, they are counted.
    pub fn header(&self, now: Instant) -> String {
        let showing = self
            .screen
            .label()
            .unwrap_or_else(|| self.views.current().name.as_str());
        let Some(elapsed) = self.jobs.elapsed_of_current(now) else {
            return format!("dam  {showing}");
        };
        let reads = self.jobs.in_flight();
        let what = match self.jobs.exclusive() {
            Some(sync) => sync.verb().to_string(),
            None => format!("{reads} read{}", if reads == 1 { "" } else { "s" }),
        };
        format!(
            "dam  {showing}  {} {what} {}",
            self.frame(),
            elapsed_text(elapsed)
        )
    }

    fn frame(&self) -> &'static str {
        match self.config.icons() {
            IconSet::NerdFont => BRAILLE[self.spinner % BRAILLE.len()],
            IconSet::Ascii => PLAIN[self.spinner % PLAIN.len()],
        }
    }
}

/// Whole tenths up to ten seconds and whole seconds after that, so the number stops flickering
/// once a job is genuinely slow.
fn elapsed_text(elapsed: Duration) -> String {
    match elapsed.as_secs() < 10 {
        true => format!("{:.1}s", elapsed.as_secs_f32()),
        false => format!("{}s", elapsed.as_secs()),
    }
}
