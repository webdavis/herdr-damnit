//! The two object readers: a listing, and one object from `dam show`.

use super::*;

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn a_listing_becomes_objects_with_their_oids_paths_and_priorities() {
    let objects = objects(&fixture("ls.json")).expect("it parsed");

    assert_eq!(objects.len(), 3);
    let bump = objects
        .iter()
        .find(|object| object.subject == "ship the pin bump")
        .expect("the captured task");
    assert_eq!(bump.path, "proj/dotfiles/");
    assert_eq!(bump.priority().get(), 1);
    assert_eq!(bump.due(), herdr_damnit_domain::parse_date("2026-09-18"));
    assert!(!bump.is_done());
}

#[test]
fn a_listing_carries_the_labels_and_the_recurrence_a_row_marks() {
    let objects = objects(&fixture("ls.json")).expect("it parsed");
    let labelled = objects
        .iter()
        .find(|object| object.subject == "refresh the roster row")
        .expect("the labelled task");
    let recurring = objects
        .iter()
        .find(|object| object.subject == "water the plants")
        .expect("the recurring task");

    assert_eq!(labelled.labels, vec!["slow".to_string()]);
    assert_eq!(recurring.recurrence.as_deref(), Some("every week"));
}

#[test]
fn an_empty_listing_is_no_objects_rather_than_an_error() {
    assert!(
        objects(&fixture("ls-empty.json"))
            .expect("it parsed")
            .is_empty()
    );
}

#[test]
fn a_done_listing_carries_completed_tasks() {
    let objects = objects(&fixture("ls-done.json")).expect("it parsed");
    assert!(!objects.is_empty());
    assert!(
        objects.iter().all(herdr_damnit_domain::Object::is_done),
        "{objects:?}"
    );
}

#[test]
fn one_object_is_read_from_show() {
    let object = object(&fixture("show-task.json")).expect("it parsed");
    assert_eq!(object.kind, herdr_damnit_domain::Kind::Task);
    assert!(!object.oid.as_str().is_empty());
}

#[test]
fn an_event_carries_its_start_end_status_and_transparency() {
    let object = object(&fixture("show-event.json")).expect("it parsed");
    assert_eq!(object.kind, herdr_damnit_domain::Kind::Event);
    let event = object.event.as_ref().expect("the event fields");
    assert!(!event.start.is_empty());
    assert!(!event.end.is_empty());
    assert!(!event.status.is_empty());
    assert!(!event.transparency.is_empty());
}

/// A priority outside one to four is not a priority at all, so the row takes the default rather
/// than refusing the whole listing over one object.
#[test]
fn a_priority_dam_never_sends_falls_back_to_the_default() {
    let document = r#"{"objects":[{"oid":"1","kind":"task","subject":"a",
      "task":{"done":false,"priority":9}}]}"#;
    let objects = objects(document).expect("it parsed");
    assert_eq!(
        objects[0].priority(),
        herdr_damnit_domain::Priority::default()
    );
}

#[test]
fn something_that_is_not_json_is_an_error_rather_than_an_empty_model() {
    assert!(objects("not json").is_err());
    assert!(
        object("{}").is_err(),
        "an object with no oid is not an object"
    );
}
