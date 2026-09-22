//! The wall clock, behind a port so every date rule is tested against a literal day.

use std::time::Instant;

use herdr_damnit_application::Clock;
use herdr_damnit_domain::Date;

pub struct SystemClock;

impl Clock for SystemClock {
    fn today(&self) -> Date {
        jiff::Zoned::now().date()
    }

    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_system_clock_answers_a_real_day_and_a_monotonic_instant() {
        let clock = SystemClock;
        assert!(clock.today().year() >= 2026);
        let first = clock.now();
        assert!(clock.now() >= first);
    }
}
