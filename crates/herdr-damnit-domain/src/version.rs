pub const DAM_MINIMUM: DamVersion = DamVersion {
    major: 0,
    minor: 2,
    patch: 0,
};

pub const DAM_HIGHEST_TESTED: DamVersion = DamVersion {
    major: 0,
    minor: 2,
    patch: 0,
};

pub const DAM_RESTORE: DamVersion = DamVersion {
    major: 0,
    minor: 2,
    patch: 0,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DamVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl std::fmt::Display for DamVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Fine,
    Warn(String),
    Refuse(String),
}

pub fn parse_version(line: &str) -> Option<DamVersion> {
    let number = line.trim().strip_prefix("dam ")?.trim();
    let mut parts = number.split('.');
    let mut next = || parts.next()?.parse::<u32>().ok();
    let version = DamVersion {
        major: next()?,
        minor: next()?,
        patch: next()?,
    };
    parts.next().is_none().then_some(version)
}

fn breaking_release(version: DamVersion) -> (u32, u32) {
    (version.major, version.minor)
}

pub fn verdict(found: DamVersion) -> Verdict {
    if found < DAM_MINIMUM {
        return Verdict::Refuse(format!(
            "dam {found} is older than the {}.{} this pane needs; run cargo install damnit to \
             update it.",
            DAM_MINIMUM.major, DAM_MINIMUM.minor
        ));
    }
    if breaking_release(found) > breaking_release(DAM_HIGHEST_TESTED) {
        return Verdict::Warn(format!(
            "dam {found} is newer than this pane knows; some keys may be refused."
        ));
    }
    Verdict::Fine
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(major: u32, minor: u32, patch: u32) -> DamVersion {
        DamVersion {
            major,
            minor,
            patch,
        }
    }

    #[test]
    fn dams_own_version_line_is_read() {
        assert_eq!(parse_version("dam 0.1.0"), Some(at(0, 1, 0)));
        assert_eq!(parse_version("dam 0.1.0\n"), Some(at(0, 1, 0)));
        assert_eq!(parse_version("dam 12.3.45"), Some(at(12, 3, 45)));
    }

    #[test]
    fn anything_that_is_not_a_version_line_is_no_version() {
        assert_eq!(parse_version(""), None);
        assert_eq!(parse_version("dam"), None);
        assert_eq!(parse_version("dam 0.1"), None);
        assert_eq!(parse_version("dam version one"), None);
        assert_eq!(parse_version("dam 0.1.0.4"), None);
    }

    #[test]
    fn a_dam_below_the_floor_is_refused_by_both_numbers() {
        let Verdict::Refuse(message) = verdict(at(0, 0, 9)) else {
            panic!("expected a refusal");
        };
        assert_eq!(
            message,
            "dam 0.0.9 is older than the 0.2 this pane needs; run cargo install damnit to update it."
        );
        assert!(
            matches!(verdict(at(0, 1, 9)), Verdict::Refuse(_)),
            "the error document and the exit codes this pane reads arrived in 0.2"
        );
    }

    #[test]
    fn a_newer_minor_draws_and_warns_once() {
        let Verdict::Warn(message) = verdict(at(0, 4, 0)) else {
            panic!("expected a warning");
        };
        assert_eq!(
            message,
            "dam 0.4.0 is newer than this pane knows; some keys may be refused."
        );
    }

    #[test]
    fn a_newer_patch_of_a_known_minor_says_nothing() {
        assert!(matches!(verdict(at(0, 2, 7)), Verdict::Fine));
        assert!(matches!(verdict(DAM_HIGHEST_TESTED), Verdict::Fine));
    }

    #[test]
    fn every_dam_the_pane_accepts_clears_the_restore_gate_the_destructive_discard_key_keeps() {
        assert!(DAM_MINIMUM >= DAM_RESTORE);
        assert!(at(0, 2, 0) >= DAM_RESTORE);
        assert!(at(0, 2, 9) >= DAM_RESTORE);
        assert!(at(0, 3, 0) >= DAM_RESTORE);
        assert!(at(0, 1, 9) < DAM_RESTORE, "and that dam is refused anyway");
    }
}
