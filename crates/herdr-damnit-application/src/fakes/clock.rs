use std::sync::{Arc, Mutex};
use std::time::Instant;

use herdr_damnit_domain::{Date, parse_date};

use crate::Clock;

#[derive(Clone)]
pub struct FakeClock {
    day: Arc<Mutex<Date>>,
    started: Instant,
}

impl FakeClock {
    pub fn on(day: &str) -> Self {
        Self {
            day: Arc::new(Mutex::new(parse_date(day).expect("a date"))),
            started: Instant::now(),
        }
    }

    pub fn turn_to(&self, day: &str) {
        *self.day.lock().expect("the day") = parse_date(day).expect("a date");
    }

    pub fn started(&self) -> Instant {
        self.started
    }
}

impl Clock for FakeClock {
    fn today(&self) -> Date {
        *self.day.lock().expect("the day")
    }

    fn now(&self) -> Instant {
        self.started
    }
}
