use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use herdr_damnit_application::{DamRunner, Finished, RunningJob, SpawnError};

mod cancel;

pub use cancel::Cancel;

const A_NEW_GROUP_THAT_DAMS_HELPER_JOINS_TOO: i32 = 0;

pub struct ProcessDamRunner {
    dam: Vec<String>,
}

impl ProcessDamRunner {
    pub fn new(dam: Vec<String>) -> Self {
        Self { dam }
    }

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
        command.process_group(A_NEW_GROUP_THAT_DAMS_HELPER_JOINS_TOO);
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
            let _ = sender.send(wait_draining_both_pipes_so_a_full_one_cannot_deadlock(
                child, started,
            ));
        });

        Ok(RunningJob {
            cancel: Box::new(move || cancel.interrupt()),
            results,
        })
    }
}

fn wait_draining_both_pipes_so_a_full_one_cannot_deadlock(
    child: std::process::Child,
    started: Instant,
) -> Finished {
    match child.wait_with_output() {
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
    }
}

impl DamRunner for ProcessDamRunner {
    fn spawn(&self, argv: &[String]) -> Result<RunningJob, SpawnError> {
        self.spawn_with_deadline(argv, None)
    }
}
