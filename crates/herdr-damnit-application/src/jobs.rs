//! The jobs in flight. One `dam` call per job, at most one exclusive job at a time, at most one
//! read of each kind, and a completion that re-reads rather than patching the model.

use std::sync::mpsc::TryRecvError;
use std::time::{Duration, Instant};

use herdr_damnit_domain::Oid;

use crate::{DamRunner, Finished, RunningJob, SpawnError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncKind {
    Commit,
    Push,
    Pull,
}

impl SyncKind {
    pub fn verb(self) -> &'static str {
        match self {
            Self::Commit => "commit",
            Self::Push => "push",
            Self::Pull => "pull",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobKind {
    ReadList,
    ReadStatus,
    ReadShow(Oid),
    ReadLog,
    Write,
    Exclusive(SyncKind),
}

impl JobKind {
    /// Whether a newer job of this kind supersedes an older one. Two writes are two different
    /// intentions and both run; two reads of one kind are the same question asked twice.
    fn supersedes_its_own_kind(&self) -> bool {
        matches!(
            self,
            Self::ReadList | Self::ReadStatus | Self::ReadShow(_) | Self::ReadLog
        )
    }

    /// The reads a completion of this kind enqueues. The rows come from the store rather than from
    /// a guess at what the write did.
    fn follow_up(&self) -> Vec<JobKind> {
        match self {
            Self::Write | Self::Exclusive(_) => vec![Self::ReadStatus, Self::ReadList],
            _ => Vec::new(),
        }
    }
}

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
    running: Vec<Running>,
    next: u64,
}

impl Jobs {
    pub fn new(runner: Box<dyn DamRunner>) -> Self {
        Self {
            runner,
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
            started: Instant::now(),
            job,
        });
        Submitted::Started(id)
    }

    /// Every job that has answered since the last call, without blocking on the ones that have not.
    pub fn drain(&mut self) -> Vec<Completion> {
        let mut completions = Vec::new();
        let mut finished = Vec::new();
        for running in &self.running {
            match running.job.results.try_recv() {
                Ok(result) => {
                    finished.push(running.id);
                    completions.push(Completion {
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

    /// Cancel the job the header names: the exclusive one when there is one, the oldest otherwise.
    pub fn cancel_current(&mut self) -> bool {
        let Some(running) = self.current() else {
            return false;
        };
        (running.job.cancel)();
        true
    }

    pub fn elapsed_of_current(&self, now: Instant) -> Option<Duration> {
        self.current()
            .map(|running| now.saturating_duration_since(running.started))
    }

    fn current(&self) -> Option<&Running> {
        self.running
            .iter()
            .find(|job| matches!(job.kind, JobKind::Exclusive(_)))
            .or_else(|| self.running.first())
    }
}

#[cfg(test)]
mod tests;
