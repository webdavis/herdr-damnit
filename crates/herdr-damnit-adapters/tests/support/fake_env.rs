static FAKE_ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

const FAKE_DAM_KNOBS: [&str; 7] = [
    "FAKE_DAM_FIXTURE_DIR",
    "FAKE_DAM_FIXTURE",
    "FAKE_DAM_EXIT",
    "FAKE_DAM_STDERR",
    "FAKE_DAM_SLEEP_MS",
    "FAKE_DAM_LOG",
    "FAKE_DAM_IGNORE_SIGINT",
];

pub struct FakeEnv {
    _held_for_the_whole_test: std::sync::MutexGuard<'static, ()>,
}

impl FakeEnv {
    pub fn lock_cleared() -> Self {
        let held = FAKE_ENV
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for knob in FAKE_DAM_KNOBS {
            // SAFETY: this test now holds the lock every change to the environment here takes, so
            // no other test thread can be reading these settings while they are removed.
            unsafe { std::env::remove_var(knob) };
        }
        Self {
            _held_for_the_whole_test: held,
        }
    }

    pub fn set(&self, knob: &str, value: impl AsRef<std::ffi::OsStr>) {
        // SAFETY: `self` holds the lock every change to the environment here takes, so no other
        // test thread can be reading these settings while this one changes.
        unsafe { std::env::set_var(knob, value) };
    }
}
