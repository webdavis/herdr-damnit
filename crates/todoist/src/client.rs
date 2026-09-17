use serde::Deserialize;

use crate::{Error, Token};

/// The authenticated user, as much of it as the pane needs.
#[derive(Debug, Deserialize)]
pub struct User {
    pub id: String,
    pub email: Option<String>,
    pub full_name: Option<String>,
}

/// A Todoist API client. One instance holds the token and a connection pool; every endpoint is a
/// method that goes through [`Client::get`], so later endpoints add a method and a response type
/// and nothing else.
#[derive(Debug)]
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    token: Token,
}

impl Client {
    /// Build a client against `base_url`, which is [`crate::DEFAULT_BASE_URL`] in production and a
    /// loopback double in tests.
    pub fn new(base_url: impl Into<String>, token: Token) -> Result<Self, Error> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("herdr-todoist/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|error| Error::Network(error.to_string()))?;
        Ok(Self {
            http,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            token,
        })
    }

    /// The authenticated user. This is the request `doctor` makes to prove the token works.
    pub async fn user(&self) -> Result<User, Error> {
        self.get("/user").await
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        let response = self
            .http
            .get(format!("{}{path}", self.base_url))
            .header(reqwest::header::AUTHORIZATION, self.token.header_value())
            .send()
            .await
            .map_err(|error| Error::Network(strip_url(&error.to_string())))?;
        let retry_after = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        if let Some(error) = Error::from_status(response.status().as_u16(), retry_after.as_deref())
        {
            return Err(error);
        }
        response
            .json()
            .await
            .map_err(|error| Error::Malformed(strip_url(&error.to_string())))
    }
}

/// reqwest appends the request URL to its messages; the status line has no room for it and the
/// base URL says nothing a reader needs.
fn strip_url(message: &str) -> String {
    match message.split_once(" for url (") {
        Some((head, _)) => head.to_string(),
        None => message.to_string(),
    }
}
