//! What the pane checks before it draws anything: the version of `dam` it found, and that one
//! `dam status --json` carries the five keys every screen reads.

use herdr_damnit_domain::{DamVersion, Verdict, parse_version, verdict};

/// The five top-level keys `dam status --json` returns. A document missing one is a `dam` built
/// from a fork rather than the one this pane was written against.
const STATUS_KEYS: [&str; 5] = ["staged", "unstaged", "conflicts", "notices", "unpushed"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Handshake {
    Ready {
        version: DamVersion,
        warning: Option<String>,
    },
    Refuse(String),
}

/// What `dam --version` alone decides. The pane asks this first and stops there on a refusal,
/// because a `dam` whose command surface this pane does not know is not one to send a read to.
pub fn check_version(version_output: &str) -> Handshake {
    let Some(version) = parse_version(version_output) else {
        return Handshake::Refuse(
            "dam did not print a version this pane could read; run cargo install damnit to \
             update it."
                .to_string(),
        );
    };
    match verdict(version) {
        Verdict::Refuse(message) => Handshake::Refuse(message),
        Verdict::Warn(message) => Handshake::Ready {
            version,
            warning: Some(message),
        },
        Verdict::Fine => Handshake::Ready {
            version,
            warning: None,
        },
    }
}

/// Whether one `dam status --json` carries the five keys every screen reads, and the refusal
/// naming the first one it does not.
pub fn check_status(status_output: &str) -> Result<(), String> {
    let Ok(document) = serde_json::from_str::<serde_json::Value>(status_output) else {
        return Err(herdr_damnit_domain::message(
            &herdr_damnit_domain::Failure::Unreadable,
        ));
    };
    for key in STATUS_KEYS {
        if document.get(key).is_none() {
            return Err(format!(
                "dam status answered without {key:?}; this pane needs a dam built from upstream."
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATUS: &str = r#"{"staged":[],"unstaged":[],"conflicts":[],"notices":[],"unpushed":[]}"#;

    #[test]
    fn a_dam_at_the_floor_with_the_five_keys_is_ready_and_quiet() {
        let Handshake::Ready { version, warning } = check_version("dam 0.2.0\n") else {
            panic!("expected ready");
        };
        assert_eq!(check_status(STATUS), Ok(()));
        assert_eq!(version.to_string(), "0.2.0");
        assert_eq!(warning, None);
    }

    #[test]
    fn a_newer_minor_is_ready_and_carries_one_warning() {
        let Handshake::Ready { warning, .. } = check_version("dam 0.4.0") else {
            panic!("expected ready");
        };
        assert_eq!(
            warning.as_deref(),
            Some("dam 0.4.0 is newer than this pane knows; some keys may be refused.")
        );
    }

    #[test]
    fn a_dam_below_the_floor_refuses_to_draw() {
        let Handshake::Refuse(message) = check_version("dam 0.0.9") else {
            panic!("expected a refusal");
        };
        assert!(
            message.contains("is older than the 0.2 this pane needs"),
            "{message}"
        );
    }

    #[test]
    fn a_version_line_that_does_not_parse_refuses_rather_than_guessing() {
        let Handshake::Refuse(message) = check_version("not a version") else {
            panic!("expected a refusal");
        };
        assert_eq!(
            message,
            "dam did not print a version this pane could read; run cargo install damnit to update it."
        );
    }

    #[test]
    fn a_status_document_missing_a_key_names_it() {
        let missing = r#"{"staged":[],"unstaged":[],"conflicts":[],"notices":[]}"#;
        let message = check_status(missing).expect_err("expected a refusal");
        assert_eq!(
            message,
            "dam status answered without \"unpushed\"; this pane needs a dam built from upstream."
        );
    }

    #[test]
    fn a_status_document_that_is_not_json_is_refused() {
        let message = check_status("not json").expect_err("expected a refusal");
        assert_eq!(
            message,
            "dam answered with something this pane could not read."
        );
    }
}
