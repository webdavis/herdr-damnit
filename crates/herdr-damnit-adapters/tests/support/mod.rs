use std::path::{Path, PathBuf};

#[allow(
    dead_code,
    reason = "each test binary compiles its own copy of this module, and the fake's own tests set its environment per command instead"
)]
pub mod fake_env;

pub struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    pub fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("herdr-damnit-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        Self { dir }
    }

    #[allow(
        dead_code,
        reason = "each test binary compiles its own copy of this module and only one calls this"
    )]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn file(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
