use std::cell::RefCell;

use super::*;

fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "herdr-todoist-queue-{name}-{}.json",
        std::process::id()
    ));
    clear(&path);
    path
}

fn close(id: &str) -> Write {
    Write::Close(id.to_string())
}

/// A replay that records what it was handed, answering each write by its id: an id in `refuse`
/// is refused with a message, an id in `offline` reports the network down, and everything else
/// is accepted. No client, no socket, no clock.
struct Sender {
    seen: RefCell<Vec<String>>,
    refuse: Vec<String>,
    offline: Vec<String>,
}

impl Sender {
    fn new() -> Self {
        Self {
            seen: RefCell::new(Vec::new()),
            refuse: Vec::new(),
            offline: Vec::new(),
        }
    }

    fn refusing(mut self, id: &str) -> Self {
        self.refuse.push(id.to_string());
        self
    }

    fn offline_at(mut self, id: &str) -> Self {
        self.offline.push(id.to_string());
        self
    }

    fn answer(&self, write: &Write) -> Result<(), Fault> {
        let id = write.task_id().unwrap_or_default().to_string();
        self.seen.borrow_mut().push(id.clone());
        if self.refuse.contains(&id) {
            return Err(Fault::Refused("Task not found".to_string()));
        }
        if self.offline.contains(&id) {
            return Err(Fault::Offline("network error: dns failure".to_string()));
        }
        Ok(())
    }

    fn seen(&self) -> Vec<String> {
        self.seen.borrow().clone()
    }
}

#[test]
fn a_queued_write_is_on_disk_the_moment_it_is_made_so_it_survives_the_pane_closing() {
    let path = scratch("restart");
    let mut queue = Queue::open(&path);

    queue.push(close("1"));
    queue.push(Write::Add("Pay rent tomorrow".to_string()));

    let reopened = Queue::open(&path);
    assert_eq!(
        reopened.waiting.len(),
        2,
        "the queue did not survive a restart"
    );
    assert_eq!(reopened.task_ids(), vec!["1"]);
}

#[test]
fn a_queue_file_that_cannot_be_read_is_an_empty_queue_rather_than_a_refusal_to_open() {
    let path = scratch("corrupt");
    std::fs::write(&path, "[{\"Close\":").expect("write");

    assert!(Queue::open(&path).is_empty());
}

#[tokio::test]
async fn the_queue_replays_oldest_first_and_empties() {
    let path = scratch("order");
    let mut queue = Queue::open(&path);
    for id in ["1", "2", "3"] {
        queue.push(close(id));
    }
    let sender = Sender::new();

    let replayed = queue
        .replay(async |write: &Write| sender.answer(write))
        .await;

    assert_eq!(sender.seen(), vec!["1", "2", "3"], "out of order");
    assert_eq!(
        replayed,
        Replayed {
            sent: 3,
            dropped: Vec::new(),
            left: 0
        }
    );
    assert!(queue.is_empty());
    assert!(Queue::open(&path).is_empty(), "the file still holds writes");
}

#[tokio::test]
async fn a_write_the_api_refuses_is_dropped_and_the_rest_of_the_queue_still_goes() {
    let path = scratch("refused");
    let mut queue = Queue::open(&path);
    for id in ["1", "2", "3", "4", "5", "6", "7", "8"] {
        queue.push(close(id));
    }
    let sender = Sender::new().refusing("5");

    let replayed = queue
        .replay(async |write: &Write| sender.answer(write))
        .await;

    assert_eq!(
        sender.seen(),
        vec!["1", "2", "3", "4", "5", "6", "7", "8"],
        "the writes behind the refused one were not sent in order"
    );
    assert_eq!(replayed.sent, 7);
    assert_eq!(replayed.dropped, vec!["Task not found".to_string()]);
    assert_eq!(replayed.left, 0);
    assert!(queue.is_empty());
    assert_eq!(
        replayed.status(),
        Some("dropped 1: Task not found  sent 7".to_string())
    );
}

#[tokio::test]
async fn a_network_that_went_down_again_stops_the_replay_and_keeps_what_is_left_in_order() {
    let path = scratch("offline");
    let mut queue = Queue::open(&path);
    for id in ["1", "2", "3", "4"] {
        queue.push(close(id));
    }
    let sender = Sender::new().offline_at("3");

    let replayed = queue
        .replay(async |write: &Write| sender.answer(write))
        .await;

    assert_eq!(
        sender.seen(),
        vec!["1", "2", "3"],
        "it went past the outage"
    );
    assert_eq!(replayed.sent, 2);
    assert_eq!(replayed.left, 2);
    assert_eq!(
        Queue::open(&path).task_ids(),
        vec!["3", "4"],
        "the unsent writes lost their order or their place"
    );
}

#[tokio::test]
async fn a_replay_that_sent_nothing_and_dropped_nothing_says_nothing() {
    let path = scratch("quiet");
    let mut queue = Queue::open(&path);
    let sender = Sender::new();

    let replayed = queue
        .replay(async |write: &Write| sender.answer(write))
        .await;

    assert_eq!(replayed.status(), None);
    assert!(sender.seen().is_empty());
}

#[test]
fn the_status_tail_counts_the_waiting_writes_in_the_columns_a_side_pane_has() {
    let path = scratch("tail");
    let mut queue = Queue::open(&path);

    assert_eq!(queue.waiting_tail(), "");
    queue.push(close("1"));
    queue.push(close("2"));
    assert_eq!(queue.waiting_tail(), " +2");
}

#[test]
fn every_write_the_queue_holds_names_the_task_it_is_about_except_an_added_one() {
    assert_eq!(close("1").task_id(), Some("1"));
    assert_eq!(Write::Delete("2".to_string()).task_id(), Some("2"));
    assert_eq!(Write::Reopen("3".to_string()).task_id(), Some("3"));
    assert_eq!(
        Write::Update {
            id: "4".to_string(),
            change: Change::default()
        }
        .task_id(),
        Some("4")
    );
    assert_eq!(
        Write::Move {
            id: "5".to_string(),
            to: Destination::Project("p1".to_string())
        }
        .task_id(),
        Some("5")
    );
    assert_eq!(Write::Add("write it up".to_string()).task_id(), None);
}

#[test]
fn a_queued_write_keeps_every_field_it_was_made_with_across_a_restart() {
    let path = scratch("fields");
    let mut queue = Queue::open(&path);
    let change = Change {
        content: Some("Pay rent".to_string()),
        labels: Some(vec!["home".to_string()]),
        ..Default::default()
    };
    queue.push(Write::Update {
        id: "1".to_string(),
        change: change.clone(),
    });
    queue.push(Write::Move {
        id: "2".to_string(),
        to: Destination::Section("s9".to_string()),
    });

    let reopened = Queue::open(&path);

    assert_eq!(
        reopened.waiting,
        vec![
            Write::Update {
                id: "1".to_string(),
                change
            },
            Write::Move {
                id: "2".to_string(),
                to: Destination::Section("s9".to_string())
            }
        ]
    );
}
