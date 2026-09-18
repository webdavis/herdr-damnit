//! An async client for the Todoist API v1 (<https://developer.todoist.com/api/v1>).
//!
//! The token is held in a [`Token`], which has no `Display` and a redacted `Debug`, so no code
//! path can print it by accident.

mod client;
mod error;
mod model;
mod token;

pub use client::{Change, Client, CompletedPage, Destination, User};
pub use error::Error;
pub use model::{Comment, CompletedTask, Due, Label, Project, Section, Task};
pub use token::{Token, TokenError, TokenSource, resolve, resolve_with};

/// The default API base. Every request path is joined onto it, and tests point the client at a
/// loopback double instead.
pub const DEFAULT_BASE_URL: &str = "https://api.todoist.com/api/v1";
