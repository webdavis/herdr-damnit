const DAM_SHORT_PREFIX_LENGTH: usize = 7;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Oid(String);

impl Oid {
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn short(&self) -> &str {
        match self.0.char_indices().nth(DAM_SHORT_PREFIX_LENGTH) {
            Some((at, _)) => &self.0[..at],
            None => &self.0,
        }
    }
}

impl std::fmt::Display for Oid {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_oid_is_the_first_seven_characters() {
        assert_eq!(Oid::new("1a2b3c4d5e6f").short(), "1a2b3c4");
    }

    #[test]
    fn an_oid_shorter_than_seven_characters_is_its_whole_self() {
        assert_eq!(Oid::new("1a2b").short(), "1a2b");
    }
}
