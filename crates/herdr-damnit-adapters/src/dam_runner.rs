//! The only place in this repository that names `dam` to `std::process::Command`. Every call runs
//! on its own thread and answers on a channel, so the draw loop never waits on one.

use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use herdr_damnit_application::{DamRunner, Finished, RunningJob, SpawnError};

mod cancel;

pub use cancel::Cancel;

pub struct ProcessDamRunner {
    dam: Vec<String>,
}

impl ProcessDamRunner {
    /// The argv the config names, `["dam"]` by default, which a test points at its own fake.
    pub fn new(dam: Vec<String>) -> Self {
        Self { dam }
    }

    /// A run that cancels itself once `deadline` passes. A read carries one; an exclusive job does
    /// not, because `dam`'s own per-remote deadline bounds it.
    pub fn spawn_with_deadline(
        &self,
        argv: &[String],
        deadline: Option<Duration>,
    ) -> Result<RunningJob, SpawnError> {
        let (binary, leading) = self.dam.split_first().ok_or(SpawnError::NotFound)?;
        let mut command = Command::new(binary);
        command
            .args(leading)
            .args(argv)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // Its own process group, so a signal reaches the helper `dam` spawned as well as `dam`.
        command.process_group(0);
        let child = command.spawn().map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => SpawnError::NotFound,
            _ => SpawnError::Io(error.to_string()),
        })?;

        let cancel = Cancel::of(child.id());
        if let Some(deadline) = deadline {
            std::thread::spawn(move || {
                std::thread::sleep(deadline);
                if cancel.alive_group() {
                    cancel.interrupt();
                }
            });
        }
        let (sender, results) = channel();
        let started = Instant::now();
        std::thread::spawn(move || {
            // `wait_with_output` reads both pipes to end of file, so a child writing more than a
            // pipe buffer cannot deadlock against a parent that is not reading.
            let finished = match child.wait_with_output() {
                Ok(output) => Finished {
                    code: output.status.code(),
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    elapsed: started.elapsed(),
                },
                Err(error) => Finished {
                    code: None,
                    stdout: String::new(),
                    stderr: error.to_string(),
                    elapsed: started.elapsed(),
                },
            };
            let _ = sender.send(finished);
        });

        Ok(RunningJob {
            cancel: Box::new(move || cancel.interrupt()),
            results,
        })
    }
}

impl DamRunner for ProcessDamRunner {
    fn spawn(&self, argv: &[String]) -> Result<RunningJob, SpawnError> {
        self.spawn_with_deadline(argv, None)
    }
}
