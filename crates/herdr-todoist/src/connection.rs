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
        self.attempt_fault(config, base_url, request)
            .await
            .map_err(Fault::into_message)
    }

    /// The same attempt, keeping whether the failure was the network being down, which is what
    /// decides between queueing a write and reporting it refused.
    pub async fn attempt_fault<T, F>(
        &mut self,
        config: &Config,
        base_url: &str,
        request: F,
    ) -> Result<T, Fault>
    where
        F: AsyncFn(&Client) -> Result<T, TodoistError>,
    {
        match self.once(&request).await {
            Attempt::Answered(value) => Ok(value),
            Attempt::Rejected => {
                *self = Self::build(config, base_url).await;
                match self.once(&request).await {
                    Attempt::Answered(value) => Ok(value),
                    Attempt::Rejected => {
                        Err(Fault::Unavailable(TodoistError::Unauthorized.to_string()))
                    }
                    Attempt::Failed(fault) => Err(fault),
                }
            }
            Attempt::Failed(fault) => Err(fault),
        }
    }

    async fn once<T, F>(&self, request: &F) -> Attempt<T>
    where
        F: AsyncFn(&Client) -> Result<T, TodoistError>,
    {
        match self {
            // A connection that never built failed on the token, not on the network, so a write
            // is not queued for a network that is not the problem. It is still worth retrying,
            // an unlocked password manager or a fixed token_command can resolve it on its own,
            // so a queued write waits it out rather than being dropped.
            Self::Failed(error) => Attempt::Failed(Fault::Unavailable(error.clone())),
            Self::Ready(client) => match request(client).await {
                Ok(value) => Attempt::Answered(value),
                Err(TodoistError::Unauthorized) => Attempt::Rejected,
                Err(error) => Attempt::Failed(Fault::of(error)),
            },
        }
    }
}

/// One outcome of a request: its answer, a rejected token, or some other failure (a rate limit, a
/// network outage, a build failure carried over from [`Connection::build`]).
enum Attempt<T> {
    Answered(T),
    Rejected,
    Failed(Fault),
}

/// Why a request failed: the network was down, the API is having a moment but may answer the
/// same request later, or it answered and said no to this one in particular. The first two are
/// worth waiting out, so a queued write survives them; only the third drops it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    Offline(String),
    Unavailable(String),
    Refused(String),
}

impl Fault {
    fn of(error: TodoistError) -> Self {
        match error {
            TodoistError::Network(_) => Self::Offline(error.to_string()),
            TodoistError::RateLimited
            | TodoistError::RateLimitedAfter(_)
            | TodoistError::Malformed(_) => Self::Unavailable(error.to_string()),
            TodoistError::Status(status) if (500..600).contains(&status) => {
                Self::Unavailable(error.to_string())
            }
            _ => Self::Refused(error.to_string()),
        }
    }

    /// The message the status line shows, whichever kind of failure it was.
    pub fn into_message(self) -> String {
        match self {
            Self::Offline(message) | Self::Unavailable(message) | Self::Refused(message) => message,
        }
    }
}

async fn try_build(config: &Config, base_url: &str) -> Result<Client, String> {
    let source = config.token_source()?;
    let token = todoist::resolve(&source)
        .await
        .map_err(|error| error.to_string())?;
    Client::new(base_url, token).map_err(|error| error.to_string())
}
