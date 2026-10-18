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
    held_for_the_whole_test: EnvLock,
}

impl FakeEnv {
    pub fn lock_cleared() -> Self {
        let held = FAKE_ENV
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for knob in FAKE_DAM_KNOBS {
            remove_while_holding_the_env_lock(&held, knob);
        }
        Self {
            held_for_the_whole_test: held,
        }
    }

    pub fn set(&self, knob: &str, value: impl AsRef<std::ffi::OsStr>) {
        set_while_holding_the_env_lock(&self.held_for_the_whole_test, knob, value);
    }
}

type EnvLock = std::sync::MutexGuard<'static, ()>;

fn remove_while_holding_the_env_lock(_: &EnvLock, knob: &str) {
    unsafe { std::env::remove_var(knob) };
}

fn set_while_holding_the_env_lock(_: &EnvLock, knob: &str, value: impl AsRef<std::ffi::OsStr>) {
    unsafe { std::env::set_var(knob, value) };
}
