use crate::{ClientError, RetryPolicy};
use zeta_http_client::HttpHeader;

/// A resolved HTTP target supplied by the runtime before an API adapter adds
/// its relative endpoint path and protocol headers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedApiTarget {
    pub base_url: String,
    pub headers: Vec<HttpHeader>,
    pub retry_policy: RetryPolicy,
}

impl ResolvedApiTarget {
    pub fn new(base_url: impl Into<String>, headers: Vec<HttpHeader>) -> Self {
        Self {
            base_url: base_url.into(),
            headers,
            retry_policy: RetryPolicy::never(),
        }
    }

    /// Replaces the default no-retry policy with runtime-selected replay
    /// semantics for requests built from this target.
    pub fn with_retry_policy(mut self, retry_policy: RetryPolicy) -> Self {
        self.retry_policy = retry_policy;
        self
    }

    /// Resolves a protocol-owned relative path without replacing the runtime's
    /// scheme, authority, or credential headers.
    pub fn endpoint(&self, path: &str) -> Result<String, ClientError> {
        let base_url = self.base_url.trim_end_matches('/');
        if !(base_url.starts_with("https://") || base_url.starts_with("http://")) {
            return Err(ClientError::InvalidRequest(
                "provider base URL must use HTTP or HTTPS".into(),
            ));
        }
        Ok(format!("{base_url}/{}", path.trim_start_matches('/')))
    }
}
