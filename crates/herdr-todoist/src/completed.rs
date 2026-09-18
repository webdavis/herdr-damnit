//! The completed list: the newest page drawn before any deeper window is read, and `u` to reopen
//! the task under the cursor.

use todoist::Client;

use crate::config::Config;
use crate::connection::Connection;
use crate::cursor::List;
use crate::history::History;

/// The completed screen: the history read so far, the cursor over it, and what the status line
/// says about both.
pub struct Completed {
    history: History,
    list: List,
    status: String,
}

impl Completed {
    /// Open the list on its newest page. One request's worth is drawn, so a history years long
    /// costs the same first screen as a history a week long.
    pub async fn open(connection: &mut Connection, config: &Config, base_url: &str) -> Self {
        let mut completed = Self {
            history: History::today(),
            list: List::new(Vec::new()),
            status: String::new(),
        };
        completed.status = completed.read(connection, config, base_url).await;
        completed
    }

    pub fn list(&self) -> &List {
        &self.list
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn up(&mut self) {
        self.list.move_cursor(-1);
    }

    /// Down one row, and the next page once the cursor is already on the last one. The page is
    /// asked for by the key press, so sitting at the bottom asks for nothing until a key is
    /// pressed again, and a walk that has reached its floor asks for nothing at all.
    pub async fn down(&mut self, connection: &mut Connection, config: &Config, base_url: &str) {
        if self.list.move_cursor(1) {
            return;
        }
        self.status = self.read(connection, config, base_url).await;
    }

    /// Reopen the task under the cursor. It stops being completed, so its row leaves at once and
    /// the cursor takes the row below it. A refusal leaves every row on screen and reports the
    /// API's own message.
    pub async fn reopen(&mut self, connection: &mut Connection, config: &Config, base_url: &str) {
        let Some(id) = self.list.selected_id().map(str::to_string) else {
            return;
        };
        let reopened = connection
            .attempt(config, base_url, async |client: &Client| {
                client.reopen(&id).await
            })
            .await;
        self.status = match reopened {
            Ok(()) => {
                self.history.forget(&id);
                self.list.refresh(self.history.rows());
                format!("reopened  {}", self.count())
            }
            Err(error) => error,
        };
    }

    /// Read pages until one brings a row or the walk reaches its floor. An empty window is walked
    /// through here rather than one key press at a time, so a history that starts further back
    /// than the newest window still draws a first screen.
    async fn read(
        &mut self,
        connection: &mut Connection,
        config: &Config,
        base_url: &str,
    ) -> String {
        while let Some(request) = self.history.request() {
            let page = connection
                .attempt(config, base_url, async |client: &Client| {
                    client
                        .completed(&request.since, &request.until, request.cursor.as_deref())
                        .await
                })
                .await;
            match page {
                Ok(page) => {
                    let held = self.history.len();
                    self.history.accept(page);
                    if self.history.len() > held {
                        break;
                    }
                }
                Err(error) => return error,
            }
        }
        self.list.refresh(self.history.rows());
        self.count()
    }

    /// What the status line says about the history: how much of it is on screen, and, at the
    /// bottom, how far back the API was read and why it stops there.
    fn count(&self) -> String {
        let count = format!("{} completed tasks", self.history.len());
        if self.history.spent() {
            return format!(
                "{count}  bottom of history: the API reads three months at a time, read back to {}",
                self.history.floor()
            );
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;

    /// A loopback double answering each request with the next body, the last one repeating, and
    /// counting what it was asked for.
    struct Double {
        base_url: String,
        requests: Arc<Mutex<usize>>,
    }

    impl Double {
        fn requests(&self) -> usize {
            *self.requests.lock().expect("lock")
        }
    }

    async fn serve(bodies: &'static [&'static str]) -> Double {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let base_url = format!("http://{}", listener.local_addr().expect("addr"));
        let requests = Arc::new(Mutex::new(0usize));
        let served = Arc::clone(&requests);
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut buffer = [0u8; 2048];
                let _ = socket.read(&mut buffer).await;
                let body = {
                    let mut count = served.lock().expect("lock");
                    let body = bodies[(*count).min(bodies.len() - 1)];
                    *count += 1;
                    body
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });
        Double { base_url, requests }
    }

    /// A double that refuses every request the way the API refuses reopening an open task.
    async fn serve_refusal() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let base_url = format!("http://{}", listener.local_addr().expect("addr"));
        let body = r#"{"error":"Invalid argument value",
             "error_extra":{"explanation":"Task is not completed"}}"#;
        let response = format!(
            "HTTP/1.1 400 Bad Request\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut buffer = [0u8; 2048];
                let _ = socket.read(&mut buffer).await;
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });
        base_url
    }

    fn config() -> Config {
        Config::parse(r#"token_command = ["sh", "-c", "printf test-token"]"#).expect("parses")
    }

    const EMPTY: &str = r#"{"items":[],"next_cursor":null}"#;
    const NEWEST: &str = r#"{"items":[{"id":"1","content":"newest",
        "completed_at":"2026-09-17T08:00:00Z"}],"next_cursor":null}"#;
    const OLDER: &str = r#"{"items":[{"id":"2","content":"older",
        "completed_at":"2026-05-02T08:00:00Z"}],"next_cursor":null}"#;

    fn texts(completed: &Completed) -> Vec<String> {
        completed
            .list()
            .rows()
            .iter()
            .map(|row| row.text().to_string())
            .collect()
    }

    #[tokio::test]
    async fn the_first_screen_is_one_page_and_the_bottom_asks_for_the_next_one_once() {
        let double = serve(&[NEWEST, OLDER]).await;
        let config = config();
        let mut connection = Connection::build(&config, &double.base_url).await;

        let mut completed = Completed::open(&mut connection, &config, &double.base_url).await;
        assert_eq!(texts(&completed), vec!["2026-09-17  newest"]);
        assert_eq!(
            double.requests(),
            1,
            "the first screen read more than a page"
        );
        assert_eq!(completed.status(), "1 completed tasks");

        completed
            .down(&mut connection, &config, &double.base_url)
            .await;
        assert_eq!(
            texts(&completed),
            vec!["2026-09-17  newest", "2026-05-02  older"]
        );
        assert_eq!(double.requests(), 2);

        completed.up();
        completed
            .down(&mut connection, &config, &double.base_url)
            .await;
        assert_eq!(
            double.requests(),
            2,
            "a move onto a row that is there asked the API for another page"
        );
    }

    #[tokio::test]
    async fn the_bottom_of_history_says_how_far_back_the_api_was_read() {
        let double = serve(&[EMPTY]).await;
        let config = config();
        let mut connection = Connection::build(&config, &double.base_url).await;

        let mut completed = Completed::open(&mut connection, &config, &double.base_url).await;
        let walked = double.requests();
        assert!(texts(&completed).is_empty());
        assert!(
            completed.status().starts_with(
                "0 completed tasks  bottom of history: the API reads three months at a time, \
                 read back to "
            ),
            "{}",
            completed.status()
        );

        completed
            .down(&mut connection, &config, &double.base_url)
            .await;

        assert_eq!(
            double.requests(),
            walked,
            "a walk that reached its floor asked the API for more"
        );
    }

    #[tokio::test]
    async fn a_reopened_task_leaves_the_list_and_the_cursor_takes_the_row_below() {
        let double = serve(&[
            r#"{"items":[{"id":"1","content":"newest","completed_at":"2026-09-17T08:00:00Z"},
                 {"id":"2","content":"older","completed_at":"2026-09-16T08:00:00Z"}],
                 "next_cursor":null}"#,
        ])
        .await;
        let config = config();
        let mut connection = Connection::build(&config, &double.base_url).await;
        let mut completed = Completed::open(&mut connection, &config, &double.base_url).await;

        completed
            .reopen(&mut connection, &config, &double.base_url)
            .await;

        assert_eq!(texts(&completed), vec!["2026-09-16  older"]);
        assert_eq!(completed.list().selected_id(), Some("2"));
        assert_eq!(completed.status(), "reopened  1 completed tasks");
    }

    #[tokio::test]
    async fn a_refused_reopen_reports_the_api_s_message_and_keeps_every_row() {
        let double = serve(&[NEWEST]).await;
        let config = config();
        let mut connection = Connection::build(&config, &double.base_url).await;
        let mut completed = Completed::open(&mut connection, &config, &double.base_url).await;
        let refusing = serve_refusal().await;
        let mut refusing_connection = Connection::build(&config, &refusing).await;

        completed
            .reopen(&mut refusing_connection, &config, &refusing)
            .await;

        assert_eq!(
            completed.status(),
            "Invalid argument value: Task is not completed"
        );
        assert_eq!(texts(&completed), vec!["2026-09-17  newest"]);
    }
}
