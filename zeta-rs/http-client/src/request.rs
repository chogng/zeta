use crate::{HttpClientError, HttpHeader};
use std::time::Duration;
use zeroize::Zeroize;

/// An HTTP method supported by the synchronous transport.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Delete,
}

impl HttpMethod {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Delete => "DELETE",
        }
    }
}

/// A fully constructed HTTP request executed exactly once by [`crate::HttpClient`].
///
/// Callers serialize protocol payloads before creating this value. Retrying,
/// streaming framing, and provider-specific response handling belong to the
/// operation client above this transport boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpRequest {
    method: HttpMethod,
    url: String,
    headers: Vec<HttpHeader>,
    body: Vec<u8>,
}

impl HttpRequest {
    pub fn new(
        method: HttpMethod,
        url: impl Into<String>,
        headers: Vec<HttpHeader>,
        body: Vec<u8>,
    ) -> Result<Self, HttpClientError> {
        let url = url.into();
        if !is_http_url(&url) {
            return Err(HttpClientError::InvalidRequest(
                "request URL must use HTTP or HTTPS".into(),
            ));
        }
        Ok(Self {
            method,
            url,
            headers,
            body,
        })
    }

    pub fn post(
        url: impl Into<String>,
        headers: Vec<HttpHeader>,
        body: Vec<u8>,
    ) -> Result<Self, HttpClientError> {
        Self::new(HttpMethod::Post, url, headers, body)
    }

    pub fn method(&self) -> HttpMethod {
        self.method
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn headers(&self) -> &[HttpHeader] {
        &self.headers
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }
}

impl Drop for HttpRequest {
    fn drop(&mut self) {
        self.body.zeroize();
    }
}

/// A bounded unary HTTP response returned without provider-specific parsing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpResponse {
    status: u16,
    headers: Vec<HttpHeader>,
    body: Vec<u8>,
}

impl HttpResponse {
    pub fn new(status: u16, headers: Vec<HttpHeader>, body: Vec<u8>) -> Self {
        Self {
            status,
            headers,
            body,
        }
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn headers(&self) -> &[HttpHeader] {
        &self.headers
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Returns a delta-seconds `Retry-After` value when the server supplies one.
    pub fn retry_after(&self) -> Option<Duration> {
        self.headers
            .iter()
            .find(|header| header.name().eq_ignore_ascii_case("retry-after"))
            .and_then(|header| header.value().parse::<u64>().ok())
            .map(Duration::from_secs)
    }
}

impl Drop for HttpResponse {
    fn drop(&mut self) {
        self.body.zeroize();
    }
}

fn is_http_url(url: &str) -> bool {
    let url = url.trim();
    let Some(authority_and_path) = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
    else {
        return false;
    };
    let authority = authority_and_path.split('/').next().unwrap_or_default();
    !authority.is_empty() && !authority.chars().any(char::is_whitespace)
}
