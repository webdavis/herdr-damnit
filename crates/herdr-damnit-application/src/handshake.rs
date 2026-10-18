use herdr_damnit_domain::{DamVersion, Verdict, parse_version, verdict};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    Starting,
    Refused(String),
    Ready,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Handshake {
    Ready {
        version: DamVersion,
        warning: Option<String>,
    },
    Refuse(String),
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dam_at_the_floor_is_ready_and_quiet() {
        let Handshake::Ready { version, warning } = check_version("dam 0.2.0\n") else {
            panic!("expected ready");
        };
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
}
