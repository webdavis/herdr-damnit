use std::time::Instant;

use super::App;

impl App {
    pub fn header(&self, now: Instant) -> String {
        let showing = self.showing.label(&self.list.views().current().name);
        match self.work.activity(now, self.config.icons()) {
            Some(activity) => format!("dam  {showing}  {activity}"),
            None => format!("dam  {showing}"),
        }
    }
}
