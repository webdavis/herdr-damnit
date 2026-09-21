//! Signalling a running `dam` and the group it spawned its helper into.
//!
//! `SIGINT` rather than `SIGTERM`: `dam` installs a `SIGINT` handler that sets a cancellation flag,
//! its wait loops read it, the helper conversation kills its own child before returning, and `dam`
//! maps a cancelled run to exit 3. It installs no `SIGTERM` handler.

use std::time::{Duration, Instant};

/// How long a `dam` gets to notice the interrupt and reap its helper before it is killed.
const GRACE: Duration = Duration::from_secs(2);

/// How often the grace is checked. `std` has no wait-with-timeout on a process group, so this is
/// a poll.
const TICK: Duration = Duration::from_millis(50);

#[derive(Clone, Copy, Debug)]
pub struct Cancel {
    group: i32,
    grace: Duration,
}

impl Cancel {
    pub fn of(pid: u32) -> Self {
        Self::with_grace(pid, GRACE)
    }

    /// The same signalling with a grace of the caller's choosing, which is how a test drives the
    /// escalation without waiting out the production one.
    pub fn with_grace(pid: u32, grace: Duration) -> Self {
        Self {
            group: pid as i32,
            grace,
        }
    }

    /// `SIGINT` to the group, the grace, then `SIGKILL` to the group.
    pub fn interrupt(self) {
        self.signal(libc::SIGINT);
        let deadline = Instant::now() + self.grace;
        while Instant::now() < deadline {
            if !self.alive() {
                return;
            }
            std::thread::sleep(TICK);
        }
        self.signal(libc::SIGKILL);
    }

    /// Whether the group is still running, which is what a deadline thread asks before signalling.
    pub fn alive_group(self) -> bool {
        self.alive()
    }

    /// A negative pid names the whole process group, which is where the remote helper is.
    fn signal(self, signal: libc::c_int) {
        // Safety: `kill` with a negative pid and a valid signal number has no memory effects; a
        // group that has already gone answers ESRCH, which is the caller's answer either way.
        unsafe { libc::kill(-self.group, signal) };
    }

    /// Signal 0 delivers nothing and only reports whether the group is still there.
    fn alive(self) -> bool {
        // Safety: as above. Signal 0 is the documented existence probe.
        unsafe { libc::kill(-self.group, 0) == 0 }
    }
}
