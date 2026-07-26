use crate::{HttpClient, HttpClientError, HttpMethod, HttpRequest, HttpResponse};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A low-cardinality classification of an HTTP response status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpStatusClass {
    Informational,
    Success,
    Redirection,
    ClientError,
    ServerError,
    Other,
}

impl HttpStatusClass {
    fn from_status(status: u16) -> Self {
        match status {
            100..=199 => Self::Informational,
            200..=299 => Self::Success,
            300..=399 => Self::Redirection,
            400..=499 => Self::ClientError,
            500..=599 => Self::ServerError,
            _ => Self::Other,
        }
    }
}

/// A safe outcome classification for one raw transport attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpTransportOutcome {
    Response { status_class: HttpStatusClass },
    TransportFailure,
}

/// Non-sensitive telemetry emitted for a single raw HTTP transport attempt.
///
/// URL, headers, certificate material, request body, and response body are
/// intentionally absent. Byte counts, method, response status class, and
/// elapsed time are safe for aggregate transport metrics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HttpClientTelemetryEvent {
    pub method: HttpMethod,
    pub outcome: HttpTransportOutcome,
    pub request_body_bytes: usize,
    pub response_body_bytes: usize,
    pub elapsed: Duration,
}

/// Receives safe, low-cardinality facts about raw HTTP transport attempts.
///
/// Implementations may forward these events to logging, tracing, or metrics,
/// but must not combine them with URLs, header values, TLS material, or raw
/// protocol payloads.
pub trait HttpClientTelemetry: Send + Sync {
    fn record(&self, event: HttpClientTelemetryEvent);
}

/// Adds safe transport telemetry around a raw HTTP client without changing its behavior.
pub struct TelemetryHttpClient {
    inner: Arc<dyn HttpClient>,
    telemetry: Arc<dyn HttpClientTelemetry>,
}

impl TelemetryHttpClient {
    pub fn new(inner: Arc<dyn HttpClient>, telemetry: Arc<dyn HttpClientTelemetry>) -> Self {
        Self { inner, telemetry }
    }
}

impl HttpClient for TelemetryHttpClient {
    fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, HttpClientError> {
        let started = Instant::now();
        let result = self.inner.execute(request);
        let (outcome, response_body_bytes) = match &result {
            Ok(response) => (
                HttpTransportOutcome::Response {
                    status_class: HttpStatusClass::from_status(response.status()),
                },
                response.body().len(),
            ),
            Err(_) => (HttpTransportOutcome::TransportFailure, 0),
        };
        self.telemetry.record(HttpClientTelemetryEvent {
            method: request.method(),
            outcome,
            request_body_bytes: request.body().len(),
            response_body_bytes,
            elapsed: started.elapsed(),
        });
        result
    }
}
