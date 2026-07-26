use crate::{ClientError, RetryPolicy};
use std::sync::Arc;
use std::thread;
use zeta_http_client::{HttpClient, HttpMethod, HttpRequest, HttpResponse};

/// A provider operation paired with its explicit replay policy.
///
/// The raw request is owned by `zeta-http-client`; this layer only decides
/// whether the complete operation may be replayed after a transport failure or
/// retryable HTTP response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientRequest {
    request: HttpRequest,
    retry_policy: RetryPolicy,
}

impl ClientRequest {
    pub fn new(
        method: HttpMethod,
        url: impl Into<String>,
        headers: Vec<zeta_http_client::HttpHeader>,
        body: Vec<u8>,
        retry_policy: RetryPolicy,
    ) -> Result<Self, ClientError> {
        Ok(Self {
            request: HttpRequest::new(method, url, headers, body)?,
            retry_policy,
        })
    }

    pub fn post(
        url: impl Into<String>,
        headers: Vec<zeta_http_client::HttpHeader>,
        body: Vec<u8>,
        retry_policy: RetryPolicy,
    ) -> Result<Self, ClientError> {
        Ok(Self {
            request: HttpRequest::post(url, headers, body)?,
            retry_policy,
        })
    }

    pub fn request(&self) -> &HttpRequest {
        &self.request
    }

    pub fn method(&self) -> HttpMethod {
        self.request.method()
    }

    pub fn url(&self) -> &str {
        self.request.url()
    }

    pub fn headers(&self) -> &[zeta_http_client::HttpHeader] {
        self.request.headers()
    }

    pub fn body(&self) -> &[u8] {
        self.request.body()
    }

    pub fn retry_policy(&self) -> RetryPolicy {
        self.retry_policy
    }
}

/// A provider-neutral unary HTTP response returned for operation decoding.
pub type ClientResponse = HttpResponse;

/// Executes a provider operation with its selected replay semantics.
///
/// Implementations must delegate each attempt to `zeta-http-client` and keep
/// retry ownership here, where the caller has explicitly declared whether the
/// request can be replayed.
pub trait OperationClient: Send + Sync {
    fn execute(&self, request: &ClientRequest) -> Result<ClientResponse, ClientError>;
}

/// Applies operation retry policy around a shared raw HTTP transport client.
pub struct ZetaClient {
    transport: Arc<dyn HttpClient>,
}

impl ZetaClient {
    pub fn new(transport: Arc<dyn HttpClient>) -> Self {
        Self { transport }
    }
}

impl OperationClient for ZetaClient {
    fn execute(&self, request: &ClientRequest) -> Result<ClientResponse, ClientError> {
        let mut attempt = 1;
        loop {
            let result = self
                .transport
                .execute(request.request())
                .map_err(ClientError::from);
            let retry_delay = match &result {
                Ok(response)
                    if request
                        .retry_policy()
                        .should_retry_response(attempt, response.status()) =>
                {
                    response
                        .retry_after()
                        .unwrap_or_else(|| request.retry_policy().backoff_delay(attempt))
                }
                Err(_) if request.retry_policy().should_retry_transport_error(attempt) => {
                    request.retry_policy().backoff_delay(attempt)
                }
                _ => return result,
            };
            attempt += 1;
            if !retry_delay.is_zero() {
                thread::sleep(retry_delay);
            }
        }
    }
}
