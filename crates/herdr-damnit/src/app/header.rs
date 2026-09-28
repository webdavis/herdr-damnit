use std::time::{Duration, Instant};

use herdr_damnit_domain::IconSet;

use super::App;

const BRAILLE_SPINNER: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
const PLAIN_SPINNER: [&str; 4] = ["|", "/", "-", "\\"];

impl App {
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
            tenths_under_ten_seconds_then_whole_seconds(elapsed)
        )
    }

    fn frame(&self) -> &'static str {
        match self.config.icons() {
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
