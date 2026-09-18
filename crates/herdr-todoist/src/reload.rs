//! Reading the open list. The pane holds one [`Screen`] worth of state, and every read of the API
//! goes through it, so a failure always lands in the status line with the rows left on screen.
//!
//! A read is also where the local copy of the view is written and where the queue of writes made
//! offline is sent: a read that reached the API is the proof that the network is back.

use todoist::{Client, Error as TodoistError, Project, Section, Task};

use crate::cache::{self, Cache};
use crate::config::Config;
use crate::connection::{Connection, Fault};
use crate::cursor::List;
use crate::icons::Marks;
use crate::list::{self, Row};
use crate::queue::{Queue, Replayed, Sent, Write};
use crate::views::Views;

/// Everything a key press acts on: the held connection, the rows on screen, the views, the local
/// copy of each view and the writes still waiting for the network.
pub struct Screen<'a> {
    pub connection: &'a mut Connection,
    pub config: &'a Config,
    pub base_url: &'a str,
    pub list: &'a mut List,
    pub views: &'a mut Views,
    pub cache: &'a mut Cache,
    pub queue: &'a mut Queue,
}

/// Which view the rows belong to, which is what decides where the cursor lands: a refresh of the
/// showing view keeps the cursor near its task, a switch to another view only keeps the task
/// itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reload {
    SameView,
    OtherView,
}

impl Screen<'_> {
    /// Run one request through the held connection, reporting either its answer or the message
    /// the status line shows.
    pub async fn request<T, F>(&mut self, request: F) -> Result<T, String>
    where
        F: AsyncFn(&Client) -> Result<T, TodoistError>,
    {
        self.connection
            .attempt(self.config, self.base_url, request)
            .await
    }

    async fn request_fault<T, F>(&mut self, request: F) -> Result<T, Fault>
    where
        F: AsyncFn(&Client) -> Result<T, TodoistError>,
    {
        self.connection
            .attempt_fault(self.config, self.base_url, request)
            .await
    }

    /// Send one write, or queue it when the network is down. A queued write is not drawn onto its
    /// row, which would put something on screen the server has not agreed to: the row keeps
    /// saying what the API last said and takes the waiting mark beside it.
    pub async fn write(&mut self, write: Write) -> Result<Sent, String> {
        match self
            .request_fault(async |client: &Client| write.send(client).await)
            .await
        {
            Ok(()) => Ok(Sent::Done),
            Err(Fault::Offline(_)) => {
                self.queue.push(write);
                list::mark_waiting(self.list.rows_mut(), &self.queue.task_ids());
                Ok(Sent::Queued)
            }
            Err(fault) => Err(fault.into_message()),
        }
    }

    /// Add a task, or queue the line when the network is down. Reports the task Todoist made of
    /// the line, and `None` when the line is waiting.
    pub async fn add(&mut self, text: &str) -> Result<Option<Task>, String> {
        match self
            .request_fault(async |client: &Client| client.quick_add(text).await)
            .await
        {
            Ok(task) => Ok(Some(task)),
            Err(Fault::Offline(_)) => {
                self.queue.push(Write::Add(text.to_string()));
                Ok(None)
            }
            Err(fault) => Err(fault.into_message()),
        }
    }

    /// Read the showing view and report the status line. A failure leaves the rows already on
    /// screen alone and says so in the status line, as the cache's age when the network is down.
    pub async fn read(&mut self, reload: Reload) -> String {
        let replayed = self.replay().await;
        let view = self.views.current().name.clone();
        let filter = self.views.current().filter.clone();
        // The day the due marks are read against is taken once per read, so every row of one
        // drawing agrees about what today is.
        let marks = Marks::today(self.config.icons);
        let now = cache::now();
        let read = self
            .request_fault(async |client: &Client| fetch(client, filter.as_deref()).await)
            .await;
        let said = match read {
            Ok((tasks, projects, sections)) => {
                let mut rows = list::build(&tasks, &projects, &sections, &marks);
                self.cache.save(&view, tasks, projects, sections, now);
                list::mark_waiting(&mut rows, &self.queue.task_ids());
                match reload {
                    Reload::SameView => self.list.refresh(rows),
                    Reload::OtherView => self.list.switch(rows),
                }
                format!(
                    "{} open tasks{}",
                    self.list.task_count(),
                    self.queue.waiting_tail()
                )
            }
            Err(Fault::Offline(message)) => {
                if reload == Reload::OtherView {
                    self.draw_cached(&view, &marks);
                }
                match self.cache.age(now) {
                    Some(age) => format!(
                        "stale {}{}",
                        cache::age_text(age),
                        self.queue.waiting_tail()
                    ),
                    None => message,
                }
            }
            Err(Fault::Refused(message)) => message,
        };
        match replayed.status() {
            Some(replay) => format!("{replay}  {said}"),
            None => said,
        }
    }

    /// Draw a view's local copy, which is what a switch to another view falls back on when the
    /// network is down: the pane's own name for the view it is showing has to be the truth. A
    /// view with no local copy leaves the rows that are on screen, and the age still names them.
    fn draw_cached(&mut self, view: &str, marks: &Marks) {
        let Some(snapshot) = self.cache.load(view) else {
            return;
        };
        let mut rows = list::build(
            &snapshot.tasks,
            &snapshot.projects,
            &snapshot.sections,
            marks,
        );
        list::mark_waiting(&mut rows, &self.queue.task_ids());
        self.list.switch(rows);
    }

    /// Send what the queue holds, oldest first. Nothing waiting is no requests at all, so the
    /// common path costs nothing.
    async fn replay(&mut self) -> Replayed {
        if self.queue.is_empty() {
            return Replayed::default();
        }
        let Screen {
            connection,
            config,
            base_url,
            queue,
            ..
        } = self;
        queue
            .replay(async |write: &Write| {
                connection
                    .attempt_fault(config, base_url, async |client: &Client| {
                        write.send(client).await
                    })
                    .await
            })
            .await
    }

    /// Read the showing view after a switch to it.
    pub async fn show(&mut self) -> String {
        self.read(Reload::OtherView).await
    }

    /// Read the showing view again, which is what follows every write: the rows the pane draws
    /// come from the server rather than from a guess at what the write did.
    pub async fn refresh(&mut self) -> String {
        self.read(Reload::SameView).await
    }

    /// The task under the cursor, or `None` when the cursor is on nothing.
    pub fn selected(&self) -> Option<&crate::list::TaskRow> {
        self.list
            .rows()
            .get(self.list.selected())
            .and_then(Row::task)
    }
}

/// The rows the pane opens on and the status line that goes with them, taken from the local copy
/// of the view so the first draw happens before any request. No readable copy is an empty list
/// and an empty status line, which the first read fills in.
pub fn opening(
    cache: &mut Cache,
    queue: &Queue,
    view: &str,
    config: &Config,
    now: u64,
) -> (Vec<Row>, String) {
    let Some(snapshot) = cache.load(view) else {
        return (Vec::new(), String::new());
    };
    let marks = Marks::today(config.icons);
    let mut rows = list::build(
        &snapshot.tasks,
        &snapshot.projects,
        &snapshot.sections,
        &marks,
    );
    list::mark_waiting(&mut rows, &queue.task_ids());
    let age = cache.age(now).unwrap_or_default();
    (
        rows,
        format!("stale {}{}", cache::age_text(age), queue.waiting_tail()),
    )
}

/// The three lists the view is built from, read at once. `filter` is the showing view's query,
/// absent on the unfiltered list.
async fn fetch(
    client: &Client,
    filter: Option<&str>,
) -> Result<(Vec<Task>, Vec<Project>, Vec<Section>), TodoistError> {
    let tasks = async {
        match filter {
            Some(query) => client.tasks_matching(query).await,
            None => client.tasks().await,
        }
    };
    tokio::try_join!(tasks, client.projects(), client.sections())
}

#[cfg(test)]
pub(crate) mod tests;
