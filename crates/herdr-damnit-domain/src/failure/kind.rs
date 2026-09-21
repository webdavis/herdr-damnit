//! The `kind` `dam` names on an error document. One word per kind, and the word itself for one
//! `dam` adds later, so a parser has somewhere to put a word this pane has not heard of.

/// Every kind word `dam` 0.2.0 publishes, and the word itself for one it adds later.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Refused,
    Store,
    Helper,
    Credential,
    Parse,
    Usage,
    Cancelled,
    Editor,
    Unknown(String),
}

impl ErrorKind {
    pub fn named(word: &str) -> Self {
        match word {
            "refused" => Self::Refused,
            "store" => Self::Store,
            "helper" => Self::Helper,
            "credential" => Self::Credential,
            "parse" => Self::Parse,
            "usage" => Self::Usage,
            "cancelled" => Self::Cancelled,
            "editor" => Self::Editor,
            other => Self::Unknown(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_word_dam_publishes_has_a_variant_of_its_own() {
        for word in [
            "refused",
            "store",
            "helper",
            "credential",
            "parse",
            "usage",
            "cancelled",
            "editor",
        ] {
            assert!(
                !matches!(ErrorKind::named(word), ErrorKind::Unknown(_)),
                "{word} has no variant"
            );
        }
    }

    #[test]
    fn a_kind_this_pane_has_not_heard_of_keeps_its_word_rather_than_failing_the_parse() {
        assert_eq!(
            ErrorKind::named("invented_tomorrow"),
            ErrorKind::Unknown("invented_tomorrow".to_string())
        );
    }
}
