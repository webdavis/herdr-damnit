use std::sync::mpsc::TryRecvError;
use std::time::{Duration, Instant};

use crate::{Clock, DamRunner, Finished, RunningJob, SpawnError};

mod kind;

pub use kind::{JobKind, SyncKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JobId(u64);

#[derive(Debug)]
pub enum Submitted {
    Started(JobId),
    Refused(String),
    NotInstalled,
    Failed(String),
}

#[derive(Debug)]
pub struct Completion {
    pub id: JobId,
    pub kind: JobKind,
    pub finished: Finished,
    pub follow_up: Vec<JobKind>,
}

struct Running {
    id: JobId,
    kind: JobKind,
    started: Instant,
    job: RunningJob,
}

pub struct Jobs {
    runner: Box<dyn DamRunner>,
    clock: Box<dyn Clock>,
    running: Vec<Running>,
    next: u64,
}

impl Jobs {
    pub fn new(runner: Box<dyn DamRunner>, clock: Box<dyn Clock>) -> Self {
        Self {
            runner,
            clock,
            running: Vec::new(),
            next: 0,
        }
    }

    pub fn submit(&mut self, kind: JobKind, argv: Vec<String>) -> Submitted {
        if let (JobKind::Exclusive(_), Some(running)) = (&kind, self.exclusive()) {
            return Submitted::Refused(format!(
                "a {} is already running; <C-c> cancels it.",
                running.verb()
            ));
        }
        if kind.supersedes_its_own_kind() {
            self.running.retain(|job| job.kind != kind);
        }
        let job = match self.runner.spawn(&argv) {
            Ok(job) => job,
            Err(SpawnError::NotFound) => return Submitted::NotInstalled,
            Err(SpawnError::Io(error)) => return Submitted::Failed(error),
        };
        self.next += 1;
        let id = JobId(self.next);
        self.running.push(Running {
            id,
            kind,
            started: self.clock.now(),
            job,
        });
        Submitted::Started(id)
    }

    pub fn drain(&mut self) -> Vec<Completion> {
        let mut completions = Vec::new();
        let mut finished = Vec::new();
        for running in &self.running {
            match running.job.results.try_recv() {
                Ok(result) => {
                    finished.push(running.id);
                    completions.push(Completion {
                        id: running.id,
                        kind: running.kind.clone(),
                        follow_up: running.kind.follow_up(),
                        finished: result,
                    });
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => finished.push(running.id),
            }
        }
        self.running.retain(|job| !finished.contains(&job.id));
        completions
    }

    pub fn in_flight(&self) -> usize {
        self.running.len()
    }

    pub fn exclusive(&self) -> Option<SyncKind> {
        self.running.iter().find_map(|job| match job.kind {
            JobKind::Exclusive(sync) => Some(sync),
            _ => None,
        })
    }

    pub fn cancel_current(&mut self) -> bool {
        let Some(running) = self.exclusive_else_oldest() else {
            return false;
        };
        (running.job.cancel)();
        true
    }

    pub fn elapsed_of_current(&self, now: Instant) -> Option<Duration> {
        self.exclusive_else_oldest()
            .map(|running| now.saturating_duration_since(running.started))
    }

    fn exclusive_else_oldest(&self) -> Option<&Running> {
        self.running
            .iter()
            .find(|job| matches!(job.kind, JobKind::Exclusive(_)))
            .or_else(|| self.running.first())
    }
}

#[cfg(test)]
mod tests;
