//! Signalling a running `dam` and the group it spawned its helper into.

#[derive(Clone, Copy, Debug)]
pub struct Cancel {
    group: i32,
}

impl Cancel {
    pub fn of(pid: u32) -> Self {
        Self { group: pid as i32 }
    }

    pub fn interrupt(self) {
        unsafe { libc::kill(-self.group, libc::SIGINT) };
    }
}
