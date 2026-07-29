use crate::{ClientError, RetryPolicy};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use zeta_async_utils::CancellationToken;
use zeta_http_client::{HttpClient, HttpMethod, HttpRequest, HttpResponse};

const CANCELLATION_POLL_INTERVAL: Duration = Duration::from_millis(5);

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
/// request can be replayed. Implementations that override
/// [`OperationClient::execute_with_cancellation`] must stop local waiting promptly when the token
/// is cancelled and must not begin another retry attempt.
pub trait OperationClient: Send + Sync {
    fn execute(&self, request: &ClientRequest) -> Result<ClientResponse, ClientError>;

    /// Executes one operation while observing a caller-owned cancellation scope.
    ///
    /// The default preserves compatibility for synchronous implementations by checking before
    /// and after execution. Implementations that own retry or transport waiting should override
    /// this method so cancellation also stops backoff and active local waiting.
    fn execute_with_cancellation(
        &self,
        request: &ClientRequest,
        cancellation: &CancellationToken,
    ) -> Result<ClientResponse, ClientError> {
        check_cancellation(cancellation)?;
        let response = self.execute(request)?;
        check_cancellation(cancellation)?;
        Ok(response)
    }
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

    fn execute_with_cancellation(
        &self,
        request: &ClientRequest,
        cancellation: &CancellationToken,
    ) -> Result<ClientResponse, ClientError> {
        let mut attempt = 1;
        loop {
            check_cancellation(cancellation)?;
            let result = self.execute_attempt(request, cancellation);
            let retry_delay = match &result {
                Err(ClientError::Cancelled(_)) => return result,
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
            wait_for_retry(retry_delay, cancellation)?;
        }
    }
}

impl ZetaClient {
    fn execute_attempt(
        &self,
        request: &ClientRequest,
        cancellation: &CancellationToken,
    ) -> Result<ClientResponse, ClientError> {
        let transport = self.transport.clone();
        let request = request.request().clone();
        let (result_tx, result_rx) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("zeta-http-attempt".into())
            .spawn(move || {
                let _ = result_tx.send(transport.execute(&request).map_err(ClientError::from));
            })
            .map_err(|_| ClientError::Transport("failed to start HTTP attempt".into()))?;

        loop {
            check_cancellation(cancellation)?;
            match result_rx.recv_timeout(CANCELLATION_POLL_INTERVAL) {
                Ok(result) => {
                    check_cancellation(cancellation)?;
                    return result;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(ClientError::Transport(
                        "HTTP attempt ended without a result".into(),
                    ));
                }
            }
        }
    }
}

fn wait_for_retry(delay: Duration, cancellation: &CancellationToken) -> Result<(), ClientError> {
    let deadline = Instant::now() + delay;
    while Instant::now() < deadline {
        check_cancellation(cancellation)?;
        thread::sleep(
            deadline
                .saturating_duration_since(Instant::now())
                .min(CANCELLATION_POLL_INTERVAL),
        );
    }
    check_cancellation(cancellation)
}

fn check_cancellation(cancellation: &CancellationToken) -> Result<(), ClientError> {
    cancellation
        .check()
        .map_err(|signal| ClientError::Cancelled(signal.reason().to_string()))
}
