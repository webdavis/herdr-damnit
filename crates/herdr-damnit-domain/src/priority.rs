//! A task's priority the way `dam` numbers it: 1 is the most urgent and 4 is the default, which
//! carries no mark at all.

const HIGHEST: u8 = 1;
const LOWEST: u8 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Priority(u8);

impl Default for Priority {
    fn default() -> Self {
        Self(LOWEST)
    }
}

impl Priority {
    pub fn new(value: u8) -> Option<Self> {
        (HIGHEST..=LOWEST).contains(&value).then_some(Self(value))
    }

    pub fn get(self) -> u8 {
        self.0
    }

    pub fn is_lowest(self) -> bool {
        self.0 == LOWEST
    }

    /// The next priority `p` cycles to: 4, 3, 2, 1 and back to 4.
    pub fn next(self) -> Self {
        match self.0 {
            HIGHEST => Self(LOWEST),
            value => Self(value - 1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dam_numbers_one_as_the_most_urgent_and_four_as_none() {
        assert_eq!(Priority::default().get(), 4);
        assert!(Priority::default().is_lowest());
        assert!(!Priority::new(1).expect("a priority").is_lowest());
    }

    #[test]
    fn nothing_outside_one_to_four_is_a_priority() {
        assert_eq!(Priority::new(0), None);
        assert_eq!(Priority::new(5), None);
        assert_eq!(Priority::new(1).map(Priority::get), Some(1));
        assert_eq!(Priority::new(4).map(Priority::get), Some(4));
    }

    #[test]
    fn the_cycle_runs_four_three_two_one_and_back_to_four() {
        let mut seen = Vec::new();
        let mut priority = Priority::default();
        for _ in 0..5 {
            seen.push(priority.get());
            priority = priority.next();
        }
        assert_eq!(seen, vec![4, 3, 2, 1, 4]);
    }
}
