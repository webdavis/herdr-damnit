//! The connection the pane holds between requests: a built client, or the reason building one
//! failed (no token source, a rejected token_command, an unreachable API).

use todoist::{Client, Error as TodoistError};

use crate::config::Config;

pub enum Connection {
    Ready(Client),
    Failed(String),
}

impl Connection {
    /// Resolve the token and build a client, so the pane opens either connected or showing why
    /// not.
    pub async fn build(config: &Config, base_url: &str) -> Self {
        match try_build(config, base_url).await {
            Ok(client) => Self::Ready(client),
            Err(error) => Self::Failed(error),
        }
    }

    /// Run one request through the held client and report either its answer or the message the
    /// pane shows. A rejected token rebuilds the client once, so a `token_command` whose output
    /// has expired is re-run, while a command that prompts for input is not spawned on every
    /// keypress.
    pub async fn attempt<T, F>(
        &mut self,
        config: &Config,
        base_url: &str,
        request: F,
    ) -> Result<T, String>
    where
        F: AsyncFn(&Client) -> Result<T, TodoistError>,
    {
        match self.once(&request).await {
            Attempt::Answered(value) => Ok(value),
            Attempt::Rejected => {
                *self = Self::build(config, base_url).await;
                match self.once(&request).await {
                    Attempt::Answered(value) => Ok(value),
                    Attempt::Rejected => Err(TodoistError::Unauthorized.to_string()),
                    Attempt::Failed(error) => Err(error),
                }
            }
            Attempt::Failed(error) => Err(error),
        }
    }

    async fn once<T, F>(&self, request: &F) -> Attempt<T>
    where
        F: AsyncFn(&Client) -> Result<T, TodoistError>,
    {
        match self {
            Self::Failed(error) => Attempt::Failed(error.clone()),
            Self::Ready(client) => match request(client).await {
                Ok(value) => Attempt::Answered(value),
                Err(TodoistError::Unauthorized) => Attempt::Rejected,
                Err(error) => Attempt::Failed(error.to_string()),
            },
        }
    }
}

/// One outcome of a request: its answer, a rejected token, or some other failure (a rate limit, a
/// network outage, a build failure carried over from [`Connection::build`]).
enum Attempt<T> {
    Answered(T),
    Rejected,
    Failed(String),
}

async fn try_build(config: &Config, base_url: &str) -> Result<Client, String> {
    let source = config.token_source()?;
    let token = todoist::resolve(&source)
        .await
        .map_err(|error| error.to_string())?;
    Client::new(base_url, token).map_err(|error| error.to_string())
}
