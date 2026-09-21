//! When the pane reads the API again on its own.
//!
//! There is no timer and no background task: the draw loop asks this schedule between draws, so
//! the interval cannot outlive the pane that owns it. The loop also says when the pane is busy,
//! which holds the refresh back while a prompt is open or another screen is on: redrawing the
//! list under a half-typed comment would take the words away.

/// The interval the pane refreshes on when the config names none.
pub const DEFAULT_SECONDS: u64 = 300;

pub struct Schedule {
    every: u64,
    last: u64,
}

impl Schedule {
    /// A schedule that is due at once, so the pane draws its cached rows and then reads.
    pub fn new(every: u64, now: u64) -> Self {
        Self {
            every,
            last: now.saturating_sub(every),
        }
    }

    /// Whether the pane reads now. An interval of zero turns the interval refresh off, leaving
    /// `R` and the read every write makes.
    pub fn due(&self, now: u64, busy: bool) -> bool {
        self.every > 0 && !busy && now.saturating_sub(self.last) >= self.every
    }

    /// Note that a read just happened, whatever asked for it, so a key press and the interval do
    /// not read twice in a row.
    pub fn mark(&mut self, now: u64) {
        self.last = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_schedule_is_due_at_once_so_the_pane_reads_after_its_first_draw() {
        assert!(Schedule::new(300, 1_000).due(1_000, false));
    }

    #[test]
    fn it_comes_due_again_one_interval_after_the_last_read() {
        let mut schedule = Schedule::new(300, 1_000);
        schedule.mark(1_000);

        assert!(!schedule.due(1_299, false));
        assert!(schedule.due(1_300, false));
        assert!(schedule.due(9_000, false));
    }

    #[test]
    fn it_never_fires_while_a_prompt_is_open() {
        let mut schedule = Schedule::new(300, 1_000);
        schedule.mark(1_000);

        assert!(!schedule.due(5_000, true), "a prompt was open");
        assert!(
            schedule.due(5_000, false),
            "and it is due again the moment the prompt closes"
        );
    }

    #[test]
    fn an_interval_of_zero_never_fires() {
        let schedule = Schedule::new(0, 1_000);

        assert!(!schedule.due(1_000_000, false));
    }

    #[test]
    fn a_clock_that_went_backwards_does_not_bring_the_read_forward() {
        let mut schedule = Schedule::new(300, 1_000);
        schedule.mark(5_000);

        assert!(!schedule.due(1_000, false));
    }
}
