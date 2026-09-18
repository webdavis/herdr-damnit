//! The task detail: one task's description rendered as markdown, its comment thread under it,
//! and the multi-line box `c` adds a comment in.
//!
//! A third screen rather than a tenth view, the way task 107's completed list is: the numbered
//! views are the operator's own filter queries and this screen has no filter, no grouping and
//! keys of its own, so `view:1` to `view:9` and `default_view` are untouched by it.

use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use todoist::{Client, Comment};

use crate::config::Config;
use crate::connection::Connection;
use crate::markdown;
use crate::prompt::Prompt;

/// One task's detail: what the list already knew about it, the thread read from the API, the
/// scroll offset, and the comment box when one is open.
pub struct Detail {
    id: String,
    content: String,
    description: String,
    comments: Vec<Comment>,
    status: String,
    scroll: u16,
    prompt: Option<Prompt>,
}

/// What a key press on the detail screen leaves behind.
#[derive(Debug, PartialEq, Eq)]
pub enum After {
    Stay,
    /// Back to the list the detail was opened from, with its cursor where it was.
    Back,
    Quit,
}

impl Detail {
    /// Open the detail and read the thread. The title and the description come from the row the
    /// cursor was on, so the screen draws at once and only the comments are waited for.
    pub async fn open(
        connection: &mut Connection,
        config: &Config,
        base_url: &str,
        id: &str,
        content: &str,
        description: &str,
    ) -> Self {
        let mut detail = Self {
            id: id.to_string(),
            content: content.trim().to_string(),
            description: description.to_string(),
            comments: Vec::new(),
            status: String::new(),
            scroll: 0,
            prompt: None,
        };
        detail.status = detail.read(connection, config, base_url).await;
        detail
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn prompt(&self) -> Option<&Prompt> {
        self.prompt.as_ref()
    }

    pub fn scroll(&self) -> u16 {
        self.scroll
    }

    /// The whole screen as lines: the task, its description, then the thread oldest first.
    pub fn lines(&self) -> Vec<Line<'static>> {
        let mut lines = vec![Line::styled(
            self.content.clone(),
            Style::new().add_modifier(Modifier::BOLD),
        )];
        lines.push(Line::raw(""));
        if self.description.trim().is_empty() {
            lines.push(dim("no description"));
        } else {
            lines.extend(markdown::render(&self.description, ""));
        }
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            format!("comments ({})", self.comments.len()),
            Style::new().add_modifier(Modifier::BOLD),
        ));
        if self.comments.is_empty() {
            lines.push(dim("no comments"));
            return lines;
        }
        for comment in &self.comments {
            lines.push(dim(posted(comment)));
            lines.extend(markdown::render(&comment.content, "  "));
            if let Some(attachment) = comment.attachment() {
                // An attachment is named, never fetched: a file a task list downloads on a key
                // press is not something this pane does.
                lines.push(dim(format!("  [attached] {attachment}")));
            }
            lines.push(Line::raw(""));
        }
        lines
    }

    /// One key press, with the comment box taking every key while it is open.
    pub async fn key(
        &mut self,
        key: crossterm::event::KeyCode,
        connection: &mut Connection,
        config: &Config,
        base_url: &str,
    ) -> After {
        use crossterm::event::KeyCode;
        if self.prompt.is_some() {
            self.compose(key, connection, config, base_url).await;
            return After::Stay;
        }
        match key {
            KeyCode::Char('q') => After::Quit,
            KeyCode::Esc | KeyCode::Enter | KeyCode::Tab | KeyCode::BackTab => After::Back,
            KeyCode::Char('j') | KeyCode::Down => {
                // Clamped to the screen's own line count, so a short task cannot scroll past its
                // last line into blank space.
                let last = self.lines().len().saturating_sub(1) as u16;
                self.scroll = self.scroll.saturating_add(1).min(last);
                After::Stay
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
                After::Stay
            }
            KeyCode::Char('c') => {
                self.prompt = Some(Prompt::comment());
                After::Stay
            }
            KeyCode::Char('r' | 'R') => {
                self.status = self.read(connection, config, base_url).await;
                After::Stay
            }
            _ => After::Stay,
        }
    }

    /// One key press into the open comment box. `<CR>` opens a line, [`crate::prompt::SEND`]
    /// posts, and `<Esc>` throws the draft away.
    async fn compose(
        &mut self,
        key: crossterm::event::KeyCode,
        connection: &mut Connection,
        config: &Config,
        base_url: &str,
    ) {
        use crossterm::event::KeyCode;
        if key == crate::prompt::SEND {
            self.post(connection, config, base_url).await;
            return;
        }
        let Some(draft) = self.prompt.as_mut().and_then(Prompt::draft_mut) else {
            return;
        };
        match key {
            KeyCode::Esc => self.prompt = None,
            KeyCode::Enter => draft.newline(),
            KeyCode::Backspace => draft.backspace(),
            KeyCode::Char(character) => draft.push(character),
            _ => {}
        }
    }

    /// Post the draft. The thread is read again afterwards rather than the comment being added to
    /// what is on screen, so the order and the timestamp drawn are the server's own. A REFUSAL
    /// leaves the box open with every line still in it: a person just typed several.
    async fn post(&mut self, connection: &mut Connection, config: &Config, base_url: &str) {
        let Some(draft) = self.prompt.as_ref().and_then(|prompt| match prompt {
            Prompt::Comment { draft } => Some(draft),
            _ => None,
        }) else {
            return;
        };
        if draft.is_blank() {
            self.prompt = None;
            return;
        }
        let text = draft.text();
        let id = self.id.clone();
        let posted = connection
            .attempt(config, base_url, async |client: &Client| {
                client.add_comment(&id, &text).await
            })
            .await;
        self.status = match posted {
            Ok(()) => {
                self.prompt = None;
                format!("posted  {}", self.read(connection, config, base_url).await)
            }
            Err(error) => error,
        };
    }

    /// Read the thread, oldest first. A failure leaves whatever is on screen alone and says so.
    async fn read(
        &mut self,
        connection: &mut Connection,
        config: &Config,
        base_url: &str,
    ) -> String {
        let id = self.id.clone();
        let thread = connection
            .attempt(config, base_url, async |client: &Client| {
                client.comments(&id).await
            })
            .await;
        match thread {
            Ok(mut comments) => {
                // The endpoint promises no ordering, so the thread is ordered here: oldest first,
                // which puts a comment just added at the bottom where it was typed. A comment
                // with no timestamp sorts last rather than jumping to the top.
                comments.sort_by(|left, right| {
                    sort_key(left)
                        .cmp(&sort_key(right))
                        .then(left.id.cmp(&right.id))
                });
                self.comments = comments;
                comment_count(self.comments.len())
            }
            Err(error) => error,
        }
    }
}

/// The status line's own count, singular for exactly one comment.
fn comment_count(count: usize) -> String {
    if count == 1 {
        "1 comment".to_string()
    } else {
        format!("{count} comments")
    }
}

/// What a comment's header line says: when it was posted, as its date, and who posted it.
fn posted(comment: &Comment) -> String {
    let when = comment
        .posted_at
        .as_deref()
        .map(|stamp| stamp.get(..10).unwrap_or(stamp).to_string())
        .unwrap_or_else(|| "undated".to_string());
    match &comment.posted_uid {
        Some(uid) => format!("{when}  #{uid}"),
        None => when,
    }
}

/// The key a comment sorts on: its timestamp, with an absent one sorting after every dated one.
fn sort_key(comment: &Comment) -> (bool, &str) {
    match comment.posted_at.as_deref() {
        Some(stamp) => (false, stamp),
        None => (true, ""),
    }
}

fn dim(text: impl Into<String>) -> Line<'static> {
    Line::styled(text.into(), Style::new().add_modifier(Modifier::DIM))
}

#[cfg(test)]
mod tests;
