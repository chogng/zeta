use crate::ApiError;
use serde_json::Value;
use std::fmt;
use std::time::Duration;

#[derive(Clone, Eq, PartialEq)]
pub struct HttpHeader {
    name: String,
    value: String,
}

impl HttpHeader {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

impl fmt::Debug for HttpHeader {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpHeader")
            .field("name", &self.name)
            .field("value", &"[redacted]")
            .finish()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedApiTarget {
    pub base_url: String,
    pub headers: Vec<HttpHeader>,
}

impl ResolvedApiTarget {
    pub fn new(base_url: impl Into<String>, headers: Vec<HttpHeader>) -> Self {
        Self {
            base_url: base_url.into(),
            headers,
        }
    }

    pub(crate) fn endpoint(&self, path: &str) -> Result<String, ApiError> {
        let base_url = self.base_url.trim_end_matches('/');
        if !(base_url.starts_with("https://") || base_url.starts_with("http://")) {
            return Err(ApiError::InvalidRequest(
                "provider base URL must use HTTP or HTTPS".into(),
            ));
        }
        Ok(format!("{base_url}/{}", path.trim_start_matches('/')))
    }
}

/// Sends one JSON request and returns its decoded JSON response.
///
/// Implementations must keep header values out of logs and errors because they can contain
/// credentials. Protocol conversion remains the responsibility of `zeta_api::Api`.
pub trait JsonHttpTransport: Send + Sync {
    fn post_json(
        &self,
        endpoint: &str,
        headers: &[HttpHeader],
        request: Value,
    ) -> Result<Value, ApiError>;
}

pub struct UreqJsonHttpTransport {
    agent: ureq::Agent,
}

impl UreqJsonHttpTransport {
    pub fn new() -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(60))
                .build(),
        }
    }
}

impl Default for UreqJsonHttpTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonHttpTransport for UreqJsonHttpTransport {
    fn post_json(
        &self,
        endpoint: &str,
        headers: &[HttpHeader],
        request: Value,
    ) -> Result<Value, ApiError> {
        let mut request_builder = self
            .agent
            .post(endpoint)
            .set("Content-Type", "application/json");
        for header in headers {
            request_builder = request_builder.set(header.name(), header.value());
        }
        let response = request_builder
            .send_json(request)
            .map_err(transport_error)?;
        response
            .into_json()
            .map_err(|_| ApiError::InvalidResponse("provider returned invalid JSON".into()))
    }
}

fn transport_error(error: ureq::Error) -> ApiError {
    match error {
        ureq::Error::Status(status, _) => {
            ApiError::Transport(format!("provider returned HTTP {status}"))
        }
        ureq::Error::Transport(_) => ApiError::Transport("request failed".into()),
    }
}
