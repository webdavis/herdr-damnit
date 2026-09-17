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
    /// The API refused the request and said why. A malformed filter query arrives here, carrying
    /// Todoist's own explanation of what it rejected.
    #[error("{0}")]
    Refused(String),
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

    /// Replace a bare status with the message the API sent with it, when it sent one. The named
    /// failures keep their own wording, which reads better than the body they arrive with.
    pub fn with_api_message(self, body: &str) -> Self {
        match (&self, api_message(body)) {
            (Self::Status(_), Some(message)) => Self::Refused(message),
            _ => self,
        }
    }
}

/// The API's own error document. Every field is optional, so a body in some other shape is read
/// as carrying no message rather than as a parse failure.
#[derive(serde::Deserialize)]
struct ApiError {
    error: Option<String>,
    error_extra: Option<ErrorExtra>,
}

#[derive(serde::Deserialize)]
struct ErrorExtra {
    explanation: Option<String>,
}

/// What the API said, as one line: its error name and, when it named one, the explanation of the
/// argument it rejected.
fn api_message(body: &str) -> Option<String> {
    let parsed: ApiError = serde_json::from_str(body).ok()?;
    let explanation = parsed.error_extra.and_then(|extra| extra.explanation);
    match (parsed.error, explanation) {
        (Some(error), Some(explanation)) => Some(format!("{error}: {explanation}")),
        (Some(message), None) | (None, Some(message)) => Some(message),
        (None, None) => None,
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

    /// The document Todoist answers a malformed filter query with.
    const REFUSED_FILTER: &str = r#"{"error_tag":"INVALID_ARGUMENT_VALUE","error_code":20,
        "error":"Invalid argument value","http_code":400,
        "error_extra":{"argument":"filter","explanation":"Unable to parse the filter query"}}"#;

    #[test]
    fn a_refused_request_carries_the_api_s_own_message() {
        let error = Error::from_status(400, None)
            .expect("an error")
            .with_api_message(REFUSED_FILTER);

        assert_eq!(
            error.to_string(),
            "Invalid argument value: Unable to parse the filter query"
        );
    }

    #[test]
    fn a_refusal_with_no_readable_message_keeps_its_status() {
        assert_eq!(
            Error::from_status(400, None)
                .expect("an error")
                .with_api_message("<html>gateway</html>"),
            Error::Status(400)
        );
    }

    #[test]
    fn a_named_failure_keeps_its_own_wording() {
        assert_eq!(
            Error::from_status(401, None)
                .expect("an error")
                .with_api_message(REFUSED_FILTER),
            Error::Unauthorized
        );
    }

    #[test]
    fn other_statuses_report_their_code() {
        assert_eq!(Error::from_status(503, None), Some(Error::Status(503)));
    }
}
