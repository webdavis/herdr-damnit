use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use herdr_damnit_domain::{Kind, Object, Oid, long};

use crate::app::App;
use crate::theme::Palette;

const INDENT: &str = "  ";

pub fn draw(frame: &mut Frame<'_>, area: Rect, app: &App, palette: &Palette) {
    let Some(object) = &app.detail else {
        return;
    };
    frame.render_widget(
        Paragraph::new(lines(object, app, palette)).wrap(Wrap { trim: false }),
        area,
    );
}

fn lines(object: &Object, app: &App, palette: &Palette) -> Vec<Line<'static>> {
    let plain = Style::new().fg(palette.text);
    let named = |name: &str, value: String| {
        Line::from(vec![
            Span::styled(format!("{name}: "), Style::new().fg(palette.dim1)),
            Span::styled(value, plain),
        ])
    };
    let mut lines = vec![Line::styled(
        object.subject.clone(),
        plain.add_modifier(Modifier::BOLD),
    )];
    lines.extend(
        fields(object, app)
            .into_iter()
            .map(|(name, value)| named(name, value)),
    );
    let mut section = |name: &str, entries: Vec<String>| {
        if !entries.is_empty() {
            lines.push(named(name, String::new()));
            lines.extend(
                entries
                    .into_iter()
                    .map(|entry| Line::styled(format!("{INDENT}{entry}"), plain)),
            );
        }
    };
    section(
        "depends",
        object
            .depends
            .iter()
            .map(|oid| with_subject(oid, app))
            .collect(),
    );
    if let Some(event) = &object.event {
        section(
            "attendees",
            event
                .attendees
                .iter()
                .map(|attendee| format!("{}: {}", attendee.email, attendee.response))
                .collect(),
        );
    }
    if !object.body.trim().is_empty() {
        lines.push(Line::default());
        lines.extend(crate::markdown::render(&object.body, ""));
    }
    lines
}

fn fields(object: &Object, app: &App) -> Vec<(&'static str, String)> {
    let kind = match object.kind {
        Kind::Task => "task",
        Kind::Event => "event",
    };
    let mut fields = vec![
        (
            "path",
            Some(object.path.clone()).filter(|path| !path.is_empty()),
        ),
        ("kind", Some(kind.to_string())),
    ];
    if let Some(task) = &object.task {
        fields.extend([
            (
                "priority",
                (!task.priority.is_lowest()).then(|| format!("p{}", task.priority.get())),
            ),
            ("due", task.due.map(long)),
            ("deadline", task.deadline.map(long)),
            (
                "event",
                task.attached_event
                    .as_ref()
                    .map(|oid| with_subject(oid, app)),
            ),
        ]);
    }
    fields.extend([
        ("recurrence", object.recurrence.clone()),
        (
            "labels",
            Some(object.labels.join(", ")).filter(|labels| !labels.is_empty()),
        ),
    ]);
    if let Some(event) = &object.event {
        fields.extend([
            ("start", Some(event.start.clone())),
            ("end", Some(event.end.clone())),
            ("timezone", event.timezone.clone()),
            ("location", event.location.clone()),
            ("status", Some(event.status.clone())),
            ("transparency", Some(event.transparency.clone())),
        ]);
    }
    fields
        .into_iter()
        .filter_map(|(name, value)| Some((name, value?)))
        .collect()
}

fn with_subject(oid: &Oid, app: &App) -> String {
    match app.subject_of(oid) {
        Some(subject) => format!("{}  {subject}", oid.short()),
        None => oid.short().to_string(),
    }
}
