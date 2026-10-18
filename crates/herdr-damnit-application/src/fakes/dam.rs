use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};

use crate::{DamRunner, Finished, RunningJob, SpawnError};

#[derive(Clone, Default)]
pub struct FakeDam {
    spawned: Arc<Mutex<Vec<Vec<String>>>>,
    answers: Arc<Mutex<Vec<Sender<Finished>>>>,
    cancelled: Arc<Mutex<Vec<usize>>>,
    refusal: Arc<Mutex<Option<SpawnError>>>,
}

impl FakeDam {
    pub fn refuse(&self, error: SpawnError) {
        *self.refusal.lock().expect("the refusal") = Some(error);
    }

    pub fn accept(&self) {
        *self.refusal.lock().expect("the refusal") = None;
    }

    pub fn answer(&self, which: usize, finished: Finished) {
        let _ = self.answers.lock().expect("the answers")[which].send(finished);
    }

    pub fn hang_up(&self) {
        self.answers.lock().expect("the answers").clear();
    }

    pub fn spawned(&self) -> Vec<Vec<String>> {
        self.spawned.lock().expect("the spawned").clone()
    }

    pub fn lines(&self) -> Vec<String> {
        self.spawned().iter().map(|argv| argv.join(" ")).collect()
    }

    pub fn cancelled(&self) -> Vec<usize> {
        self.cancelled.lock().expect("the cancelled").clone()
    }
}

impl DamRunner for FakeDam {
    fn spawn(&self, argv: &[String]) -> Result<RunningJob, SpawnError> {
        if let Some(error) = self.refusal.lock().expect("the refusal").clone() {
            return Err(error);
        }
        let mut spawned = self.spawned.lock().expect("the spawned");
        let which = spawned.len();
        spawned.push(argv.to_vec());
        let (sender, results) = channel();
        self.answers.lock().expect("the answers").push(sender);
        let cancelled = Arc::clone(&self.cancelled);
        Ok(RunningJob {
            cancel: Box::new(move || cancelled.lock().expect("the cancelled").push(which)),
            results,
        })
    }
}
