//! Fixtures the integration tests of this crate share.

use std::path::{Path, PathBuf};

/// A directory of one test's own, removed when the value drops. Every temp path a test writes goes
/// inside it, so a run leaves nothing behind in the system temp directory, and a panicking test
/// cleans up on the way out because `Drop` runs while the stack unwinds.
pub struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    /// `name` distinguishes the scratches of one test binary from each other; the process id
    /// distinguishes concurrent binaries.
    pub fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("herdr-damnit-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        Self { dir }
    }

    // Each test binary compiles its own copy of this module, so a method only one of them calls
    // reads as dead code in the others.
    #[allow(dead_code)]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// A path inside this scratch. Nothing outside this test writes there.
    pub fn file(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
