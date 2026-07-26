use crate::{ClientError, ClientRequest, ClientResponse, OperationClient};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Low-cardinality, non-sensitive operation metadata for client telemetry.
///
/// The value is static so request URLs, model IDs, tenant identifiers, prompt
/// content, and credentials cannot accidentally become metric dimensions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClientOperation {
    name: &'static str,
}

impl ClientOperation {
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }

    pub const fn name(self) -> &'static str {
        self.name
    }
}

/// The safe result classification reported for one client operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientTelemetryOutcome {
    Succeeded,
    Failed,
}

/// A safe telemetry record emitted after a provider operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClientTelemetryEvent {
    pub operation: ClientOperation,
    pub outcome: ClientTelemetryOutcome,
    pub elapsed: Duration,
}

/// Receives transport-level telemetry without request or response payloads.
///
/// Implementations may bridge events to tracing or metrics systems, but must
/// preserve the supplied value boundary: they must not reconstruct or append
/// model IDs, URLs, headers, credentials, prompts, or response bodies.
pub trait ClientTelemetry: Send + Sync {
    fn record(&self, event: ClientTelemetryEvent);
}

/// Adds safe telemetry around any unary provider operation client.
pub struct TelemetryOperationClient {
    inner: Arc<dyn OperationClient>,
    telemetry: Arc<dyn ClientTelemetry>,
    operation: ClientOperation,
}

impl TelemetryOperationClient {
    pub fn new(
        inner: Arc<dyn OperationClient>,
        telemetry: Arc<dyn ClientTelemetry>,
        operation: ClientOperation,
    ) -> Self {
        Self {
            inner,
            telemetry,
            operation,
        }
    }
}

impl OperationClient for TelemetryOperationClient {
    fn execute(&self, request: &ClientRequest) -> Result<ClientResponse, ClientError> {
        let started = Instant::now();
        let result = self.inner.execute(request);
        let outcome = if result.is_ok() {
            ClientTelemetryOutcome::Succeeded
        } else {
            ClientTelemetryOutcome::Failed
        };
        self.telemetry.record(ClientTelemetryEvent {
            operation: self.operation,
            outcome,
            elapsed: started.elapsed(),
        });
        result
    }
}
