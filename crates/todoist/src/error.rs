/// Everything a request can fail with, in the words the pane's status line shows.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("unauthorized: the token was rejected")]
    Unauthorized,
    #[error("forbidden: the token lacks access to this resource")]
    Forbidden,
    #[error("rate limited, retry in {0}s")]
    RateLimitedAfter(u64),
    #[error("rate limited, retry shortly")]
    RateLimited,
    #[error("the API answered {0}")]
    Status(u16),
    #[error("network error: {0}")]
    Network(String),
    #[error("unreadable response: {0}")]
    Malformed(String),
}

impl Error {
    /// The error a response status carries, or `None` when the request succeeded.
    ///
    /// `retry_after` is the `Retry-After` header, read as whole seconds; its HTTP-date form is
    /// reported as a bare rate limit.
    pub fn from_status(status: u16, retry_after: Option<&str>) -> Option<Self> {
        match status {
            200..=299 => None,
            401 => Some(Self::Unauthorized),
            403 => Some(Self::Forbidden),
            429 => Some(
                match retry_after.and_then(|value| value.trim().parse().ok()) {
                    Some(seconds) => Self::RateLimitedAfter(seconds),
                    None => Self::RateLimited,
                },
            ),
            other => Some(Self::Status(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_carries_no_error() {
        assert_eq!(Error::from_status(200, None), None);
    }

    #[test]
    fn auth_failures_are_named() {
        assert_eq!(Error::from_status(401, None), Some(Error::Unauthorized));
        assert_eq!(Error::from_status(403, None), Some(Error::Forbidden));
    }

    #[test]
    fn rate_limit_keeps_the_retry_delay() {
        assert_eq!(
            Error::from_status(429, Some("42")),
            Some(Error::RateLimitedAfter(42))
        );
        assert_eq!(
            Error::from_status(429, Some("42")).unwrap().to_string(),
            "rate limited, retry in 42s"
        );
    }

    #[test]
    fn rate_limit_without_a_readable_delay_still_reports_the_limit() {
        assert_eq!(Error::from_status(429, None), Some(Error::RateLimited));
        assert_eq!(
            Error::from_status(429, Some("Wed, 21 Oct 2026 07:28:00 GMT")),
            Some(Error::RateLimited)
        );
    }

    #[test]
    fn other_statuses_report_their_code() {
        assert_eq!(Error::from_status(503, None), Some(Error::Status(503)));
    }
}
