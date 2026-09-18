//! Reading the open list. The pane holds one [`Screen`] worth of state, and every read of the API
//! goes through it, so a failure always lands in the status line with the rows left on screen.

use todoist::{Client, Error as TodoistError};

use crate::config::Config;
use crate::connection::Connection;
use crate::cursor::List;
use crate::icons::Marks;
use crate::list::{self, Row};
use crate::views::Views;

/// Everything a key press acts on: the held connection, the rows on screen and the views.
pub struct Screen<'a> {
    pub connection: &'a mut Connection,
    pub config: &'a Config,
    pub base_url: &'a str,
    pub list: &'a mut List,
    pub views: &'a mut Views,
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

    /// Read the showing view and report the status line. A failure leaves the rows already on
    /// screen alone and says so in the status line.
    pub async fn read(&mut self, reload: Reload) -> String {
        let filter = self.views.current().filter.clone();
        // The day the due marks are read against is taken once per read, so every row of one
        // drawing agrees about what today is.
        let marks = Marks::today(self.config.icons);
        let rows = self
            .request(async |client: &Client| fetch(client, filter.as_deref(), &marks).await)
            .await;
        match rows {
            Ok(rows) => {
                match reload {
                    Reload::SameView => self.list.refresh(rows),
                    Reload::OtherView => self.list.switch(rows),
                }
                format!("{} open tasks", self.list.task_count())
            }
            Err(error) => error,
        }
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

/// The three lists the view is built from, read at once. `filter` is the showing view's query,
/// absent on the unfiltered list.
async fn fetch(
    client: &Client,
    filter: Option<&str>,
    marks: &Marks,
) -> Result<Vec<Row>, TodoistError> {
    let tasks = async {
        match filter {
            Some(query) => client.tasks_matching(query).await,
            None => client.tasks().await,
        }
    };
    let (tasks, projects, sections) =
        tokio::try_join!(tasks, client.projects(), client.sections())?;
    Ok(list::build(&tasks, &projects, &sections, marks))
}

#[cfg(test)]
pub(crate) mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;
    use crate::list::tests::task;

    /// One empty page, which every list endpoint parses.
    pub(crate) const EMPTY_PAGE: &str = r#"{"results":[],"next_cursor":null}"#;

    /// The document Todoist answers a malformed filter query with.
    const REFUSED_FILTER: &str = r#"{"error_tag":"INVALID_ARGUMENT_VALUE","error_code":20,
        "error":"Invalid argument value","http_code":400,
        "error_extra":{"argument":"filter","explanation":"Unable to parse the filter query"}}"#;

    pub(crate) fn config_with_token_command(script: &str) -> Config {
        Config::parse(&format!(r#"token_command = ["sh", "-c", "{script}"]"#)).expect("parses")
    }

    pub(crate) fn list_of_one() -> List {
        let project = serde_json::from_str(r#"{"id":"p1","name":"First"}"#).expect("project");
        List::new(list::build(
            &[task(r#"{"id":"1","content":"keep me","project_id":"p1"}"#)],
            &[project],
            &[],
            &crate::list::tests::marks(),
        ))
    }

    /// A loopback double answering every request the same way, until the listener is dropped.
    pub(crate) async fn serve_forever(status: &'static str, body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let base_url = format!("http://{}", listener.local_addr().expect("addr"));
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut buffer = [0u8; 1024];
                let _ = socket.read(&mut buffer).await;
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });
        base_url
    }

    /// A screen over the given rows, reading a double at `base_url`.
    pub(crate) fn screen<'a>(
        connection: &'a mut Connection,
        config: &'a Config,
        base_url: &'a str,
        list: &'a mut List,
        views: &'a mut Views,
    ) -> Screen<'a> {
        Screen {
            connection,
            config,
            base_url,
            list,
            views,
        }
    }

    #[tokio::test]
    async fn a_failed_connection_reports_its_error_without_probing_the_network() {
        let mut connection = Connection::Failed("no token source: neither key is set".to_string());
        let config = Config::default();
        let mut list = List::new(Vec::new());
        let mut views = Views::new(&[]);
        let mut screen = screen(
            &mut connection,
            &config,
            "http://127.0.0.1:1",
            &mut list,
            &mut views,
        );

        assert_eq!(
            screen.refresh().await,
            "no token source: neither key is set"
        );
    }

    #[tokio::test]
    async fn a_refresh_reuses_the_client_instead_of_rerunning_token_command() {
        let counter = std::env::temp_dir().join(format!(
            "herdr-todoist-token-command-runs-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&counter);
        let base_url = serve_forever("200 OK", EMPTY_PAGE).await;
        let config = config_with_token_command(&format!(
            "printf x >> {} && printf test-token",
            counter.display()
        ));
        let mut connection = Connection::build(&config, &base_url).await;
        let mut list = List::new(Vec::new());
        let mut views = Views::new(&[]);
        let mut screen = screen(&mut connection, &config, &base_url, &mut list, &mut views);

        let first = screen.refresh().await;
        let second = screen.refresh().await;

        assert_eq!(first, "0 open tasks");
        assert_eq!(second, "0 open tasks");
        let runs = std::fs::read_to_string(&counter).expect("counter file");
        let _ = std::fs::remove_file(&counter);
        assert_eq!(runs, "x", "token_command ran more than once: {runs:?}");
    }

    #[tokio::test]
    async fn a_rejected_token_is_reported_after_rebuilding_once() {
        let base_url = serve_forever("401 Unauthorized", "{}").await;
        let config = config_with_token_command("printf test-token");
        let mut connection = Connection::build(&config, &base_url).await;
        let mut list = List::new(Vec::new());
        let mut views = Views::new(&[]);
        let mut screen = screen(&mut connection, &config, &base_url, &mut list, &mut views);

        assert_eq!(
            screen.refresh().await,
            "unauthorized: the token was rejected"
        );
    }

    #[tokio::test]
    async fn a_rate_limited_refresh_keeps_the_last_good_list_on_screen() {
        let base_url = serve_forever("429 Too Many Requests", "{}").await;
        let config = config_with_token_command("printf test-token");
        let mut connection = Connection::build(&config, &base_url).await;
        let mut list = list_of_one();
        let mut views = Views::new(&[]);
        let mut screen = screen(&mut connection, &config, &base_url, &mut list, &mut views);

        let message = screen.refresh().await;

        assert_eq!(message, "rate limited, retry shortly");
        assert_eq!(list.task_count(), 1);
        assert_eq!(list.selected_id(), Some("1"));
    }

    #[tokio::test]
    async fn a_refused_filter_reports_the_api_s_own_message_and_keeps_the_rows_on_screen() {
        let base_url = serve_forever("400 Bad Request", REFUSED_FILTER).await;
        let config = config_with_token_command("printf test-token");
        let mut connection = Connection::build(&config, &base_url).await;
        let mut list = list_of_one();
        let mut views = Views::new(&[crate::config::View {
            name: "work".to_string(),
            filter: "#Work &".to_string(),
        }]);
        views.select(1);
        let mut screen = screen(&mut connection, &config, &base_url, &mut list, &mut views);

        let message = screen.show().await;

        assert_eq!(
            message,
            "Invalid argument value: Unable to parse the filter query"
        );
        assert_eq!(list.task_count(), 1);
        assert_eq!(list.selected_id(), Some("1"));
    }

    #[tokio::test]
    async fn a_filter_matching_nothing_empties_the_list_instead_of_reporting_a_failure() {
        let base_url = serve_forever("200 OK", EMPTY_PAGE).await;
        let config = config_with_token_command("printf test-token");
        let mut connection = Connection::build(&config, &base_url).await;
        let mut list = list_of_one();
        let mut views = Views::new(&[crate::config::View {
            name: "nobody".to_string(),
            filter: "today & @nobody".to_string(),
        }]);
        views.select(1);
        let mut screen = screen(&mut connection, &config, &base_url, &mut list, &mut views);

        let message = screen.show().await;

        assert_eq!(message, "0 open tasks");
        assert_eq!(list.task_count(), 0);
    }

    #[tokio::test]
    async fn a_network_failure_keeps_the_last_good_list_on_screen() {
        let config = config_with_token_command("printf test-token");
        let base_url = {
            let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
            let address = listener.local_addr().expect("addr");
            format!("http://{address}")
        };
        let mut connection = Connection::build(&config, &base_url).await;
        let mut list = list_of_one();
        let mut views = Views::new(&[]);
        let mut screen = screen(&mut connection, &config, &base_url, &mut list, &mut views);

        let message = screen.refresh().await;

        assert!(message.starts_with("network error:"), "{message}");
        assert_eq!(list.task_count(), 1);
    }
}
