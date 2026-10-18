use std::path::Path;
use std::time::Duration;

pub const PATIENCE: Duration = Duration::from_secs(20);

pub const POLL: Duration = Duration::from_millis(5);

pub fn logged(log: &Path) -> usize {
    std::fs::read_to_string(log)
        .map(|text| text.lines().filter(|line| !line.trim().is_empty()).count())
        .unwrap_or(0)
}

pub fn interrupts(log: &Path) -> usize {
    std::fs::read_to_string(log)
        .expect("a log")
        .matches("SIGINT")
        .count()
}
