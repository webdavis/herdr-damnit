use serde::{Deserialize, Serialize};

use crate::model::{Items, Page};
use crate::{Comment, CompletedTask, Error, Label, Project, Section, Task, Token};

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

    /// One page of the tasks completed in `[since, until)`, both ISO 8601 timestamps, newest
    /// first being no promise of the endpoint: the caller orders what it reads. The API caps the
    /// window at three months and pages it by cursor, so this makes exactly ONE request and hands
    /// back the cursor for the next: a caller drawing a screen at a time never reads a history it
    /// has not been asked for.
    pub async fn completed(
        &self,
        since: &str,
        until: &str,
        cursor: Option<&str>,
    ) -> Result<CompletedPage, Error> {
        let mut query = vec![
            ("since", since.to_string()),
            ("until", until.to_string()),
            ("limit", COMPLETED_PAGE_LIMIT.to_string()),
        ];
        if let Some(cursor) = cursor {
            query.push(("cursor", cursor.to_string()));
        }
        let page: Items<CompletedTask> = self
            .get("/tasks/completed/by_completion_date", &query)
            .await?;
        Ok(CompletedPage {
            tasks: page.items,
            next_cursor: page.next_cursor,
        })
    }

    /// Complete the task. Its answer carries nothing the pane reads.
    pub async fn close(&self, id: &str) -> Result<(), Error> {
        self.post(&format!("/tasks/{id}/close"), None).await
    }

    /// Delete the task and every subtask under it. There is no undo, which is why the pane asks
    /// before calling this.
    pub async fn delete(&self, id: &str) -> Result<(), Error> {
        self.send(reqwest::Method::DELETE, &format!("/tasks/{id}"), &[], None)
            .await?;
        Ok(())
    }

    /// Change the fields [`Change`] names and leave every other field of the task alone.
    pub async fn update(&self, id: &str, change: &Change) -> Result<(), Error> {
        self.post(&format!("/tasks/{id}"), Some(json(change)?))
            .await
    }

    /// Move the task to another project or section.
    pub async fn move_task(&self, id: &str, to: &Destination) -> Result<(), Error> {
        self.post(&format!("/tasks/{id}/move"), Some(json(to)?))
            .await
    }

    /// Add a task from a line of Quick Add syntax. Todoist parses the text and answers with the
    /// task it made of it, which is what the pane reports rather than its own reading of the line.
    pub async fn quick_add(&self, text: &str) -> Result<Task, Error> {
        let body = json(&QuickAdd { text })?;
        let response = self
            .send(reqwest::Method::POST, "/tasks/quick", &[], Some(body))
            .await?;
        response
            .json()
            .await
            .map_err(|error| Error::Malformed(strip_url(&error.to_string())))
    }

    /// Every comment on the task, in whatever order the endpoint answers in: the vendor schema
    /// promises no ordering, so the caller orders what it reads. The endpoint takes exactly one
    /// of `task_id` and `project_id`, and paginates like every other list.
    pub async fn comments(&self, task_id: &str) -> Result<Vec<Comment>, Error> {
        self.collect("/comments", &[("task_id", task_id.to_string())])
            .await
    }

    /// Add a comment to the task. Its answer is the comment the API made, which the pane does not
    /// read: it re-reads the thread instead, so what is drawn is the server's own ordering.
    pub async fn add_comment(&self, task_id: &str, content: &str) -> Result<(), Error> {
        self.post("/comments", Some(json(&NewComment { task_id, content })?))
            .await
    }

    /// Every label the account has, which is what the label picker offers.
    pub async fn labels(&self) -> Result<Vec<Label>, Error> {
        self.collect("/labels", &[]).await
    }

    /// Reopen a completed task. Its answer carries nothing the pane reads, so only a refusal,
    /// which arrives in the API's own words, is worth reporting.
    pub async fn reopen(&self, id: &str) -> Result<(), Error> {
        self.post(&format!("/tasks/{id}/reopen"), None).await
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
        let response = self.send(reqwest::Method::GET, path, query, None).await?;
        response
            .json()
            .await
            .map_err(|error| Error::Malformed(strip_url(&error.to_string())))
    }

    /// A write whose answer the pane does not read. The body is left unparsed, so an empty answer
    /// and a document are both a success.
    async fn post(&self, path: &str, body: Option<String>) -> Result<(), Error> {
        self.send(reqwest::Method::POST, path, &[], body).await?;
        Ok(())
    }

    /// One authenticated request, with every failure the API can report mapped to an [`Error`]
    /// carrying the API's own message.
    async fn send(
        &self,
        method: reqwest::Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<String>,
    ) -> Result<reqwest::Response, Error> {
        let mut url = reqwest::Url::parse(&format!("{}{path}", self.base_url))
            .map_err(|error| Error::Network(error.to_string()))?;
        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query);
        }
        let mut request = self
            .http
            .request(method, url)
            .header(reqwest::header::AUTHORIZATION, self.token.header_value());
        if let Some(body) = body {
            request = request
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(body);
        }
        let response = request
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
        Ok(response)
    }
}

/// The fields of a task an update writes. Every one left `None` is left out of the request, which
/// is how the API is told to keep that field unchanged.
#[derive(Debug, Default, Serialize)]
pub struct Change {
    /// 1 to 4, where 4 is the app's p1 and 1 its p4.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,
    /// A due date in Todoist's own natural language, which the API parses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_string: Option<String>,
    /// The task's whole label set, names not ids.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
}

/// Where a move puts the task. The API takes one id, so a section move names the section alone and
/// the task follows it into that section's project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Destination {
    #[serde(rename = "project_id")]
    Project(String),
    #[serde(rename = "section_id")]
    Section(String),
}

#[derive(Serialize)]
struct QuickAdd<'a> {
    text: &'a str,
}

/// A comment to create. `task_id` and `content` are the only two fields the pane sends: an
/// attachment is a file upload, which a task pane is not the place for.
#[derive(Serialize)]
struct NewComment<'a> {
    task_id: &'a str,
    content: &'a str,
}

fn json<T: Serialize>(body: &T) -> Result<String, Error> {
    serde_json::to_string(body).map_err(|error| Error::Malformed(error.to_string()))
}

/// One page of completed tasks, with the cursor that reads the page after it.
#[derive(Debug)]
pub struct CompletedPage {
    pub tasks: Vec<CompletedTask>,
    pub next_cursor: Option<String>,
}

/// Pages are read at the API's documented maximum, so a few hundred open tasks is one request.
const PAGE_LIMIT: u16 = 200;

/// A completed page is the endpoint's own default size, which is about a screen of history.
const COMPLETED_PAGE_LIMIT: u16 = 50;

/// reqwest appends the request URL to its messages; the status line has no room for it and the
/// base URL says nothing a reader needs.
fn strip_url(message: &str) -> String {
    match message.split_once(" for url (") {
        Some((head, _)) => head.to_string(),
        None => message.to_string(),
    }
}
