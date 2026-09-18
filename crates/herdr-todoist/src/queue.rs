//! The writes made while the network was down, kept in order until it comes back.
//!
//! The queue is a file under the plugin's own state directory, so a task completed on a train is
//! still waiting to be sent after the pane is closed and opened again. It is replayed one write
//! at a time, oldest first: a write the API refuses is dropped with its message and the rest go
//! on, and a write that fails because the network is still down stays at the head with everything
//! behind it untouched.

#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use todoist::{Change, Client, Destination, Error as TodoistError};

use crate::connection::Fault;

/// One write the pane makes on the task under the cursor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Write {
    Close(String),
    Reopen(String),
    Delete(String),
    Update { id: String, change: Change },
    Move { id: String, to: Destination },
    Add(String),
}

impl Write {
    pub async fn send(&self, client: &Client) -> Result<(), TodoistError> {
        match self {
            Self::Close(id) => client.close(id).await,
            Self::Reopen(id) => client.reopen(id).await,
            Self::Delete(id) => client.delete(id).await,
            Self::Update { id, change } => client.update(id, change).await,
            Self::Move { id, to } => client.move_task(id, to).await,
            Self::Add(text) => client.quick_add(text).await.map(|_| ()),
        }
    }

    /// The task this write is about, which is the row that carries the waiting mark. An added
    /// task has no row yet: it is not on screen until the queue has been sent.
    pub fn task_id(&self) -> Option<&str> {
        match self {
            Self::Close(id) | Self::Reopen(id) | Self::Delete(id) => Some(id),
            Self::Update { id, .. } | Self::Move { id, .. } => Some(id),
            Self::Add(_) => None,
        }
    }
}

/// The waiting writes and the file that holds them.
pub struct Queue {
    path: PathBuf,
    waiting: Vec<Write>,
}

/// What became of a write the pane asked for: the API took it, or it is waiting for the network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sent {
    Done,
    Queued,
}

/// What one replay did. `dropped` carries the API's own words for each write it refused, and
/// `left` is what is still waiting because the network went down again mid-replay.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Replayed {
    pub sent: usize,
    pub dropped: Vec<String>,
    pub left: usize,
}

impl Replayed {
    /// What the status line says about the replay, or `None` when there was nothing to replay.
    /// A refusal leads, because it is the one outcome that lost a change the operator made.
    pub fn status(&self) -> Option<String> {
        match (self.sent, self.dropped.first()) {
            (0, None) => None,
            (sent, None) => Some(format!("sent {sent}")),
            (sent, Some(first)) => Some(format!(
                "dropped {}: {first}  sent {sent}",
                self.dropped.len()
            )),
        }
    }
}

impl Queue {
    /// Read the queue, or start an empty one when the file is missing or unreadable.
    pub fn open(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let waiting = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self { path, waiting }
    }

    /// The queue under the plugin's own state directory, beside the cache.
    pub fn in_state_dir() -> Self {
        Self::open(crate::state::state_dir().join("queue.json"))
    }

    /// Put a write at the back of the queue and write the file, so the pane exiting now still
    /// sends it later.
    pub fn push(&mut self, write: Write) {
        self.waiting.push(write);
        self.save();
    }

    /// The file the waiting writes live in.
    #[cfg(test)]
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_empty(&self) -> bool {
        self.waiting.is_empty()
    }

    /// What the status line says about the writes still waiting, in the fewest columns that say
    /// it: the same mark their rows carry, and how many there are. Nothing waiting says nothing.
    pub fn waiting_tail(&self) -> String {
        match self.waiting.len() {
            0 => String::new(),
            waiting => format!(" {}{waiting}", crate::list::WAITING),
        }
    }

    /// The tasks with a write waiting, which is what the list marks.
    pub fn task_ids(&self) -> Vec<&str> {
        self.waiting.iter().filter_map(Write::task_id).collect()
    }

    /// Send the waiting writes, oldest first, stopping at the first one the network refused to
    /// carry. The file is rewritten after every write, so a pane that exits mid-replay leaves
    /// exactly what is still unsent.
    pub async fn replay<F>(&mut self, mut send: F) -> Replayed
    where
        F: AsyncFnMut(&Write) -> Result<(), Fault>,
    {
        let mut replayed = Replayed::default();
        while let Some(write) = self.waiting.first() {
            match send(write).await {
                Ok(()) => replayed.sent += 1,
                Err(Fault::Refused(message)) => replayed.dropped.push(message),
                Err(Fault::Offline(_)) => break,
            }
            self.waiting.remove(0);
            self.save();
        }
        replayed.left = self.waiting.len();
        replayed
    }

    fn save(&self) {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string(&self.waiting) {
            let _ = std::fs::write(&self.path, text);
        }
    }
}

/// Delete the file, which is how a test starts from an empty queue.
#[cfg(test)]
pub(crate) fn clear(path: &Path) {
    let _ = std::fs::remove_file(path);
}

#[cfg(test)]
mod tests;
