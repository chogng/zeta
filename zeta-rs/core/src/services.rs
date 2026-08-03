use crate::CoreError;
use std::collections::BTreeSet;
use std::path::PathBuf;
use zeta_async_utils::CancellationToken;
use zeta_policy::{ActionReviewRequest, AutoReviewGrant, GrantId, ReviewEvidence};
use zeta_protocol::{
    ActionApprovalRequest, ModelRef, ModelRequest, ModelResponse, ModelStreamEvent, RequestId,
    ThreadUpdateEnvelope, ToolCall, ToolCallId, ToolDefinition, ToolExecutionOutput,
    ToolOutputStream,
};
use zeta_sandboxing::SandboxPolicy;

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

/// Selects the immutable model runtime used for one Turn.
///
/// Legacy Sessions without a durable selection use the resolved configuration default. New
/// Sessions pass their snapshotted model explicitly so later configuration or Session changes
/// cannot alter an already-started Turn.
#[derive(Clone, Copy)]
pub enum ModelSelection<'a> {
    ConfiguredDefault,
    Session(&'a ModelRef),
}

/// Executes one provider-independent model invocation.
///
/// Implementations receive a complete immutable request assembled by Core. They must not read
/// Thread state or mutable product configuration. Implementations should observe `cancellation`
/// before beginning expensive work and at every safe checkpoint supported by their transport.
pub trait ModelService: Send + Sync {
    fn invoke(
        &self,
        selection: ModelSelection<'_>,
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
        selection: ModelSelection<'_>,
        request: &ModelRequest,
        cancellation: &CancellationToken,
        sink: &mut dyn ModelStreamSink,
    ) -> Result<ModelResponse, CoreError> {
        let response = self.invoke(selection, request, cancellation)?;
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

/// Receives transient, typed output from one running Tool Call.
///
/// Implementations publish best-effort output only. The durable Tool Result remains the
/// authoritative replay and recovery boundary.
pub trait ToolOutputSink {
    fn emit(&mut self, stream: ToolOutputStream, text: String) -> Result<(), CoreError>;
}

/// Update sink used by hosts that do not expose live Thread subscriptions.
pub struct NoThreadUpdates;

impl ThreadUpdateSink for NoThreadUpdates {
    fn publish(&self, _: ThreadUpdateEnvelope) {}
}

/// Explicit authority under which a prepared tool call may execute.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ToolAuthorization {
    Sandboxed(SandboxPolicy),
    UnsandboxedGrant { grant_id: GrantId },
    AutoReviewed(AutoReviewedToolGrant),
    ApprovedOnce(OneTimeToolGrant),
}

/// Non-reusable automatic-review authority bound to one exact durable Tool Call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutoReviewedToolGrant {
    tool_call_id: ToolCallId,
    policy_grant: AutoReviewGrant,
}

impl AutoReviewedToolGrant {
    pub(crate) fn new(tool_call_id: ToolCallId, policy_grant: AutoReviewGrant) -> Self {
        Self {
            tool_call_id,
            policy_grant,
        }
    }

    pub fn tool_call_id(&self) -> &ToolCallId {
        &self.tool_call_id
    }

    pub fn policy_grant(&self) -> &AutoReviewGrant {
        &self.policy_grant
    }
}

/// A non-reusable user grant bound to one durable interaction and exact Tool Call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OneTimeToolGrant {
    request_id: RequestId,
    tool_call_id: ToolCallId,
    approval: ActionApprovalRequest,
}

impl OneTimeToolGrant {
    pub(crate) fn new(
        request_id: RequestId,
        tool_call_id: ToolCallId,
        approval: ActionApprovalRequest,
    ) -> Self {
        Self {
            request_id,
            tool_call_id,
            approval,
        }
    }

    pub fn request_id(&self) -> &RequestId {
        &self.request_id
    }

    pub fn tool_call_id(&self) -> &ToolCallId {
        &self.tool_call_id
    }

    pub fn approval(&self) -> &ActionApprovalRequest {
        &self.approval
    }
}

/// Executes tools selected and durably recorded by Core.
///
/// Implementations expose immutable definitions and execute only the exact materialized call
/// passed by Core. They must enforce their sandbox and resource policy, preserve the call ID, and
/// never mutate Thread state directly.
pub trait ToolService: Send + Sync {
    fn definitions(&self) -> Vec<ToolDefinition>;

    /// Materializes every security-relevant field before policy review.
    ///
    /// Implementations must resolve aliases, paths, executable identity, provenance, required
    /// capabilities, and sandbox compatibility without causing the requested side effect.
    fn prepare(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError>;

    /// Collects bounded, secret-free evidence needed to interpret an otherwise opaque action.
    ///
    /// Implementations may inspect local state but must not perform the proposed action, use
    /// credentials, access the network, or mutate anything. Repository and file contents must be
    /// labeled as untrusted evidence by their constructors.
    fn review_evidence(&self, _: &ToolCall) -> Result<Vec<ReviewEvidence>, CoreError> {
        Ok(Vec::new())
    }

    /// Executes under the exact selected authority and reports a protocol-owned outcome.
    ///
    /// `SandboxDenied` is valid only for `Sandboxed` authority and must distinguish an ordinary
    /// command failure from backend enforcement. `SafeToRetry` additionally guarantees that the
    /// requested child action did not begin; otherwise implementations must report
    /// `MayHaveSideEffects` or `OutcomeUnknown`. An `Err` after invocation begins is treated as an
    /// unknown outcome and is never automatically replayed.
    fn execute(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError>;

    /// Executes a Tool Call with durable facts derived from the current Thread transcript.
    ///
    /// File-mutating implementations use these facts to enforce read-before-write without
    /// coupling the tool layer to Thread storage. Existing tools may retain the default bridge.
    fn execute_with_facts(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
        _: &ToolExecutionFacts,
    ) -> Result<ToolExecutionOutput, CoreError> {
        self.execute(call, authorization, cancellation)
    }

    /// Executes a Tool Call while optionally publishing typed transient output.
    ///
    /// Services without an incremental transport retain the default terminal-only behavior.
    fn execute_streaming(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
        _: &mut dyn ToolOutputSink,
    ) -> Result<ToolExecutionOutput, CoreError> {
        self.execute(call, authorization, cancellation)
    }

    /// Streaming counterpart to [`ToolService::execute_with_facts`].
    fn execute_streaming_with_facts(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
        facts: &ToolExecutionFacts,
        sink: &mut dyn ToolOutputSink,
    ) -> Result<ToolExecutionOutput, CoreError> {
        let _ = facts;
        self.execute_streaming(call, authorization, cancellation, sink)
    }
}

/// Read-before-write evidence reconstructed from successful durable `read_file` results.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ToolExecutionFacts {
    read_paths: BTreeSet<PathBuf>,
}

impl ToolExecutionFacts {
    pub(crate) fn from_items(items: &[zeta_protocol::ThreadItem]) -> Self {
        let mut calls = std::collections::BTreeMap::new();
        let mut facts = Self::default();
        for item in items {
            match item {
                zeta_protocol::ThreadItem::ToolCall {
                    tool_call_id,
                    name,
                    arguments_json,
                    ..
                } => {
                    if name.as_str() == "read_file"
                        && let Ok(arguments) =
                            serde_json::from_str::<serde_json::Value>(arguments_json)
                        && let Some(path) =
                            arguments.get("path").and_then(serde_json::Value::as_str)
                    {
                        calls.insert(tool_call_id.clone(), PathBuf::from(path));
                    }
                }
                zeta_protocol::ThreadItem::ToolResult {
                    tool_call_id,
                    is_error: false,
                    ..
                } => {
                    if let Some(path) = calls.get(tool_call_id) {
                        facts.read_paths.insert(path.clone());
                    }
                }
                _ => {}
            }
        }
        facts
    }

    pub fn read_paths(&self) -> impl Iterator<Item = &PathBuf> {
        self.read_paths.iter()
    }
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
        _: &ToolAuthorization,
        _: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        Ok(ToolExecutionOutput::Failure(format!(
            "tool is not available: {}",
            call.name
        )))
    }

    fn prepare(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        Err(CoreError::Policy(format!(
            "tool is not available for policy review: {}",
            call.name
        )))
    }
}
