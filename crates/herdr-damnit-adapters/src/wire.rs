//! `dam`'s documents, read into the pane's own types. The structs here mirror `dam`'s wire shape
//! field for field; the fixtures beside them are byte copies of what the built `dam` produced.

use herdr_damnit_domain::{
    Attendee, EventFields, Kind, Object, Oid, Priority, TaskFields, parse_date,
};
use serde::Deserialize;

mod failures;
mod reports;

pub use failures::error_document;
pub use reports::{completions, pull_summary, push_summary, stage};

#[derive(Deserialize)]
pub(super) struct WireObject {
    oid: String,
    kind: String,
    pub(super) subject: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    depends: Vec<String>,
    #[serde(default)]
    recurrence: Option<String>,
    #[serde(default)]
    task: Option<WireTask>,
    #[serde(default)]
    event: Option<WireEvent>,
}

#[derive(Deserialize)]
struct WireTask {
    done: bool,
    priority: u8,
    #[serde(default)]
    due: Option<String>,
    #[serde(default)]
    deadline: Option<String>,
    #[serde(default)]
    event: Option<String>,
}

#[derive(Deserialize)]
struct WireEvent {
    start: String,
    end: String,
    #[serde(default)]
    timezone: Option<String>,
    #[serde(default)]
    location: Option<String>,
    #[serde(default)]
    attendees: Vec<WireAttendee>,
    status: String,
    transparency: String,
}

#[derive(Deserialize)]
struct WireAttendee {
    email: String,
    response: String,
}

#[derive(Deserialize)]
struct Listing {
    objects: Vec<WireObject>,
}

/// `dam ls --json`.
pub fn objects(json: &str) -> Result<Vec<Object>, String> {
    let listing: Listing = read(json)?;
    Ok(listing.objects.into_iter().map(into_object).collect())
}

/// `dam show <oid> --json`.
pub fn object(json: &str) -> Result<Object, String> {
    read::<WireObject>(json).map(into_object)
}

pub(super) fn into_object(wire: WireObject) -> Object {
    Object {
        oid: Oid::new(wire.oid),
        kind: match wire.kind.as_str() {
            "event" => Kind::Event,
            _ => Kind::Task,
        },
        subject: wire.subject,
        body: wire.body,
        path: wire.path,
        labels: wire.labels,
        depends: wire.depends.into_iter().map(Oid::new).collect(),
        recurrence: wire.recurrence,
        task: wire.task.map(|task| TaskFields {
            done: task.done,
            priority: Priority::new(task.priority).unwrap_or_default(),
            due: task.due.as_deref().and_then(parse_date),
            deadline: task.deadline.as_deref().and_then(parse_date),
            attached: task.event.map(Oid::new),
        }),
        event: wire.event.map(|event| EventFields {
            start: event.start,
            end: event.end,
            timezone: event.timezone,
            location: event.location,
            status: event.status,
            transparency: event.transparency,
            attendees: event
                .attendees
                .into_iter()
                .map(|attendee| Attendee {
                    email: attendee.email,
                    response: attendee.response,
                })
                .collect(),
        }),
    }
}

pub(super) fn read<T: serde::de::DeserializeOwned>(json: &str) -> Result<T, String> {
    serde_json::from_str(json).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests;
