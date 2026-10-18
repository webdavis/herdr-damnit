use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};

use crate::{DamRunner, Finished, RunningJob, SpawnError};

type UnansweredJob = (Vec<String>, Sender<Finished>);

#[derive(Clone, Default)]
pub struct FakeDam {
    spawned: Arc<Mutex<Vec<Vec<String>>>>,
    unanswered: Arc<Mutex<Vec<UnansweredJob>>>,
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

    pub fn answer(&self, starting_with: &[&str], finished: Finished) {
        let mut unanswered = self.unanswered.lock().expect("the unanswered");
        let latest = unanswered
            .iter()
            .rposition(|(argv, _)| {
                let leading = argv.iter().map(String::as_str).take(starting_with.len());
                leading.eq(starting_with.iter().copied())
            })
            .unwrap_or_else(|| panic!("no unanswered job starts with {starting_with:?}"));
        let _ = unanswered.remove(latest).1.send(finished);
    }

    pub fn hang_up(&self) {
        self.unanswered.lock().expect("the unanswered").clear();
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
        self.unanswered
            .lock()
            .expect("the unanswered")
            .push((argv.to_vec(), sender));
        let cancelled = Arc::clone(&self.cancelled);
        Ok(RunningJob {
            cancel: Box::new(move || cancelled.lock().expect("the cancelled").push(which)),
            results,
        })
    }
}
