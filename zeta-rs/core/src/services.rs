use crate::CoreError;
use zeta_async_utils::CancellationToken;
use zeta_protocol::{
    ModelRequest, ModelResponse, ModelStreamEvent, ThreadUpdateEnvelope, ToolCall, ToolDefinition,
};

/// Holds a process-local or inter-process write lock for a Thread.
///
/// Implementations release their underlying lease when the guard is dropped and must never let
/// two live guards represent concurrent writers for the same Thread.
pub trait LeaseGuard: Send {}

/// Arbitrates exclusive write access to one durable aggregate identity.
///
/// Implementations must scope leases by both the concrete ID type and value, reject competing
/// writers, and return a guard that holds the lease for the complete mutation.
pub trait WriterLease<Id>: Send + Sync {
    fn acquire(&self, id: &Id) -> Result<Box<dyn LeaseGuard>, CoreError>;
}

/// Receives provider-neutral incremental output for one model invocation.
///
/// Implementations must preserve event order and should return an error when the receiving
/// execution can no longer safely consume a delta, such as after cancellation.
pub trait ModelStreamSink {
    fn emit(&mut self, event: ModelStreamEvent) -> Result<(), CoreError>;
}

/// Executes one provider-independent model invocation.
///
/// Implementations receive a complete immutable request assembled by Core. They must not read
/// Thread state or mutable product configuration. Implementations should observe `cancellation`
/// before beginning expensive work and at every safe checkpoint supported by their transport.
pub trait ModelService: Send + Sync {
    fn invoke(
        &self,
        request: &ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<ModelResponse, CoreError>;

    /// Streams incremental output and returns the terminal canonical response.
    ///
    /// The default bridge keeps synchronous adapters compatible by invoking [`Self::invoke`] and
    /// emitting each final text or reasoning item as one delta. Provider adapters should override
    /// this method when their wire protocol exposes earlier incremental output.
    fn stream(
        &self,
        request: &ModelRequest,
        cancellation: &CancellationToken,
        sink: &mut dyn ModelStreamSink,
    ) -> Result<ModelResponse, CoreError> {
        let response = self.invoke(request, cancellation)?;
        for item in &response.output {
            let event = match item {
                zeta_protocol::ResponseItem::Text(text) => {
                    Some(ModelStreamEvent::TextDelta(text.clone()))
                }
                zeta_protocol::ResponseItem::Reasoning(text) => {
                    Some(ModelStreamEvent::ReasoningDelta(text.clone()))
                }
                zeta_protocol::ResponseItem::Refusal(_)
                | zeta_protocol::ResponseItem::ToolCall(_) => None,
            };
            if let Some(event) = event {
                sink.emit(event)?;
            }
        }
        Ok(response)
    }
}

/// Publishes a Core-produced Thread update to an outer subscription transport.
///
/// Implementations must treat transient updates as best-effort and must not block durable Core
/// commits on a slow client connection. Durable updates can always be replayed from the store.
pub trait ThreadUpdateSink: Send + Sync {
    fn publish(&self, update: ThreadUpdateEnvelope);
}

/// Update sink used by hosts that do not expose live Thread subscriptions.
pub struct NoThreadUpdates;

impl ThreadUpdateSink for NoThreadUpdates {
    fn publish(&self, _: ThreadUpdateEnvelope) {}
}

/// The user-safe outcome of one tool execution.
///
/// A tool-level failure is returned as data so the Agent can inspect it and decide whether to
/// continue. Infrastructure failures that prevent the service from producing a trustworthy
/// outcome should be returned as [`CoreError`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ToolExecutionOutput {
    Success(String),
    Failure(String),
}

/// Executes tools selected and durably recorded by Core.
///
/// Implementations expose immutable definitions and execute only the exact materialized call
/// passed by Core. They must enforce their sandbox and resource policy, preserve the call ID, and
/// never mutate Thread state directly.
pub trait ToolService: Send + Sync {
    fn definitions(&self) -> Vec<ToolDefinition>;

    fn execute(
        &self,
        call: &ToolCall,
        cancellation: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError>;
}

/// Tool service used by hosts that expose no tools.
pub struct NoTools;

impl ToolService for NoTools {
    fn definitions(&self) -> Vec<ToolDefinition> {
        Vec::new()
    }

    fn execute(
        &self,
        call: &ToolCall,
        _: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        Ok(ToolExecutionOutput::Failure(format!(
            "tool is not available: {}",
            call.name
        )))
    }
}
