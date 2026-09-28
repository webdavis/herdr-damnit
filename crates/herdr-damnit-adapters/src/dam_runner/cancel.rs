use std::time::{Duration, Instant};

const DAMS_CANCELLATION_SIGNAL: libc::c_int = libc::SIGINT;

const GRACE_TO_REAP_THE_HELPER_BEFORE_THE_KILL: Duration = Duration::from_secs(2);

const GROUP_EXIT_POLL: Duration = Duration::from_millis(50);

#[derive(Clone, Copy, Debug)]
pub struct Cancel {
    group: i32,
    grace: Duration,
}

impl Cancel {
    pub fn of(pid: u32) -> Self {
        Self::with_grace(pid, GRACE_TO_REAP_THE_HELPER_BEFORE_THE_KILL)
    }

    pub fn with_grace(pid: u32, grace: Duration) -> Self {
        Self {
            group: pid as i32,
            grace,
        }
    }

    pub fn interrupt(self) {
        self.signal_group(DAMS_CANCELLATION_SIGNAL);
        let deadline = Instant::now() + self.grace;
        while Instant::now() < deadline {
            if !self.alive() {
                return;
            }
            std::thread::sleep(GROUP_EXIT_POLL);
        }
        self.signal_group(libc::SIGKILL);
    }

    pub fn alive_group(self) -> bool {
        self.alive()
    }

    fn signal_group(self, signal: libc::c_int) {
        // SAFETY: `kill` takes two plain numbers and writes to none of this program's memory, so it
        // cannot corrupt anything; a group that has already exited just makes the call fail.
        unsafe { libc::kill(-self.group, signal) };
    }

    fn alive(self) -> bool {
        // SAFETY: the same plain-number call as above, and signal 0 sends nothing: it only asks the
        // system whether the group still exists.
        unsafe { libc::kill(-self.group, 0) == 0 }
    }
}
