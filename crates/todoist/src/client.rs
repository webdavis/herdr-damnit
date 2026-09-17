use serde::Deserialize;

use crate::model::Page;
use crate::{Error, Project, Section, Task, Token};

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
        self.get("/user", &[]).await
    }

    /// Every open task. The endpoint lists active tasks, so a completed one never arrives here.
    pub async fn tasks(&self) -> Result<Vec<Task>, Error> {
        self.collect("/tasks", &[]).await
    }

    /// The open tasks a filter query selects, in Todoist's own filter language, the one the app's
    /// Filters feature uses. A malformed query is refused by the API with its own message.
    pub async fn tasks_matching(&self, query: &str) -> Result<Vec<Task>, Error> {
        self.collect("/tasks/filter", &[("query", query.to_string())])
            .await
    }

    pub async fn projects(&self) -> Result<Vec<Project>, Error> {
        self.collect("/projects", &[]).await
    }

    pub async fn sections(&self) -> Result<Vec<Section>, Error> {
        self.collect("/sections", &[]).await
    }

    /// Read a list endpoint to its end, following `next_cursor`. A repeated cursor ends the walk
    /// rather than looping forever.
    async fn collect<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        extra: &[(&str, String)],
    ) -> Result<Vec<T>, Error> {
        let mut collected = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let mut query = vec![("limit", PAGE_LIMIT.to_string())];
            query.extend(extra.iter().map(|(key, value)| (*key, value.clone())));
            if let Some(cursor) = &cursor {
                query.push(("cursor", cursor.clone()));
            }
            let page: Page<T> = self.get(path, &query).await?;
            collected.extend(page.results);
            match page.next_cursor {
                Some(next) if Some(&next) != cursor.as_ref() => cursor = Some(next),
                _ => return Ok(collected),
            }
        }
    }

    async fn get<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T, Error> {
        let mut url = reqwest::Url::parse(&format!("{}{path}", self.base_url))
            .map_err(|error| Error::Network(error.to_string()))?;
        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query);
        }
        let response = self
            .http
            .get(url)
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
            let body = response.text().await.unwrap_or_default();
            return Err(error.with_api_message(&body));
        }
        response
            .json()
            .await
            .map_err(|error| Error::Malformed(strip_url(&error.to_string())))
    }
}

/// Pages are read at the API's documented maximum, so a few hundred open tasks is one request.
const PAGE_LIMIT: u16 = 200;

/// reqwest appends the request URL to its messages; the status line has no room for it and the
/// base URL says nothing a reader needs.
fn strip_url(message: &str) -> String {
    match message.split_once(" for url (") {
        Some((head, _)) => head.to_string(),
        None => message.to_string(),
    }
}
