use super::*;

mod log;
mod status;
mod sync;

pub(super) fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn something_that_is_not_json_is_an_error_rather_than_an_empty_model() {
    assert!(stage("not json").is_err());
    assert!(completions("not json").is_err());
    assert!(push_summary("not json").is_err());
    assert!(pull_summary("not json").is_err());
}
