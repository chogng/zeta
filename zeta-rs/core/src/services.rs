use crate::ContextBudget;
use crate::ContextTokenMeasurementCapability;
use crate::ContextTokenMeasurementOutcome;
use crate::CoreError;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;
use zeta_async_utils::CancellationToken;
use zeta_policy::ActionReviewRequest;
use zeta_policy::AutoReviewGrant;
use zeta_policy::GrantId;
use zeta_policy::PermissionBypassGrant;
use zeta_policy::ReviewEvidence;
use zeta_protocol::ActionApprovalRequest;
use zeta_protocol::ModelRef;
use zeta_protocol::ModelRequest;
use zeta_protocol::ModelResponse;
use zeta_protocol::ModelStreamEvent;
use zeta_protocol::RequestId;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadUpdateEnvelope;
use zeta_protocol::ToolCall;
use zeta_protocol::ToolCallBinding;
use zeta_protocol::ToolCallCaller;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolDefinition;
use zeta_protocol::ToolExecutionOutput;
use zeta_protocol::ToolOutputStream;
use zeta_protocol::ToolSourceProvenance;
use zeta_protocol::TurnId;
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
    /// Returns the immutable context budget for the selected model invocation.
    ///
    /// Implementations should return a Core-managed budget only when the model window and product
    /// output reservation are known. Unknown or unlisted models retain provider-managed overflow
    /// behavior rather than receiving a fabricated context limit.
    fn context_budget(&self, _: ModelSelection<'_>) -> Result<ContextBudget, CoreError> {
        Ok(ContextBudget::provider_managed())
    }

    /// Reports whether the selected immutable model can measure input locally or remotely.
    fn input_token_measurement_capability(
        &self,
        _: ModelSelection<'_>,
    ) -> Result<ContextTokenMeasurementCapability, CoreError> {
        Ok(ContextTokenMeasurementCapability::Unavailable)
    }

    /// Measures one fully assembled candidate request before invocation.
    ///
    /// Implementations must measure the same immutable model and canonical request snapshot that
    /// [`Self::invoke`] receives. Post-response usage does not satisfy this contract.
    fn measure_input(
        &self,
        _: ModelSelection<'_>,
        _: &ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<ContextTokenMeasurementOutcome, CoreError> {
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        Ok(ContextTokenMeasurementOutcome::Unavailable)
    }

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

/// One bounded, revision-bound piece of untrusted evidence supplied to model context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextEvidence {
    pub source: String,
    pub reference: String,
    pub revision: String,
    pub body: String,
}

/// Stable identities and user query for one optional context-source lookup.
pub struct ContextSourceRequest<'a> {
    pub session_id: &'a SessionId,
    pub thread_id: &'a ThreadId,
    pub turn_id: &'a TurnId,
    pub query: &'a str,
}

/// Supplies optional, low-trust evidence without owning context ordering or budget policy.
///
/// Implementations must return bounded data, preserve revision/provenance identities, observe
/// cancellation, and avoid mutating Thread state. Core treats all returned bodies as untrusted
/// user-level data and may omit them under budget pressure.
pub trait ContextSource: Send + Sync {
    fn collect(
        &self,
        request: &ContextSourceRequest<'_>,
        cancellation: &CancellationToken,
    ) -> Result<Vec<ContextEvidence>, CoreError>;
}

/// Empty source used when a host has not enabled automatic context enrichment.
pub struct NoContextSource;

impl ContextSource for NoContextSource {
    fn collect(
        &self,
        _: &ContextSourceRequest<'_>,
        cancellation: &CancellationToken,
    ) -> Result<Vec<ContextEvidence>, CoreError> {
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        Ok(Vec::new())
    }
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
    PermissionBypassed(PermissionBypassToolGrant),
    ApprovedOnce(OneTimeToolGrant),
}

type ModelToolCallBinder =
    dyn Fn(&ToolCall, ToolCallCaller) -> Result<Option<ToolCallBinding>, CoreError> + Send + Sync;

/// Frozen tool catalog selected for one model invocation safe point.
///
/// Reloadable registries attach a binder that resolves model-produced calls against this exact
/// catalog generation. Static services may omit the binder and let Core use their ordinary live
/// binding method because their definitions cannot change during the invocation.
pub struct ModelToolCatalogSnapshot {
    definitions: Vec<ToolDefinition>,
    binder: Option<Arc<ModelToolCallBinder>>,
}

impl ModelToolCatalogSnapshot {
    /// Freezes one static catalog that continues to use the service's ordinary binder.
    pub fn new(definitions: Vec<ToolDefinition>) -> Self {
        Self {
            definitions,
            binder: None,
        }
    }

    /// Freezes a reloadable catalog with the exact binder that produced its definitions.
    pub fn with_binder(
        definitions: Vec<ToolDefinition>,
        binder: impl Fn(&ToolCall, ToolCallCaller) -> Result<Option<ToolCallBinding>, CoreError>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            definitions,
            binder: Some(Arc::new(binder)),
        }
    }

    /// Returns the definitions visible to this model invocation.
    pub fn definitions(&self) -> &[ToolDefinition] {
        &self.definitions
    }

    /// Uses the frozen binder when the service supplied one.
    ///
    /// `None` means Core must use the static service's ordinary `bind_call` implementation.
    pub fn bind_call(
        &self,
        call: &ToolCall,
        caller: ToolCallCaller,
    ) -> Option<Result<Option<ToolCallBinding>, CoreError>> {
        self.binder.as_ref().map(|binder| binder(call, caller))
    }
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

/// Non-reusable permission-bypass authority bound to one exact durable Tool Call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PermissionBypassToolGrant {
    tool_call_id: ToolCallId,
    policy_grant: PermissionBypassGrant,
}

impl PermissionBypassToolGrant {
    pub(crate) fn new(tool_call_id: ToolCallId, policy_grant: PermissionBypassGrant) -> Self {
        Self {
            tool_call_id,
            policy_grant,
        }
    }

    pub fn tool_call_id(&self) -> &ToolCallId {
        &self.tool_call_id
    }

    pub fn policy_grant(&self) -> &PermissionBypassGrant {
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

    /// Freezes the exact host definition and stable source chain selected for a model Tool Call.
    ///
    /// The default bridge hashes the canonical protocol definition at generation zero. A
    /// generation-bound registry must override this method with its exact definition digest and
    /// stable source chain before Core commits the call.
    fn bind_call(
        &self,
        call: &ToolCall,
        caller: ToolCallCaller,
    ) -> Result<Option<ToolCallBinding>, CoreError> {
        let definition = self
            .definitions()
            .into_iter()
            .find(|definition| definition.name == call.name)
            .ok_or_else(|| {
                CoreError::Execution(format!("tool definition is unavailable: {}", call.name))
            })?;
        let canonical = serde_json::to_vec(&definition)
            .map_err(|error| CoreError::Execution(error.to_string()))?;
        let source_chain = self.source_provenance(&call.name);
        Ok(Some(ToolCallBinding {
            registry_incarnation: None,
            registry_generation: 0,
            definition_digest: format!("sha256:{:x}", Sha256::digest(canonical)),
            source_chain: if source_chain.is_empty() {
                vec![ToolSourceProvenance::System {
                    id: "tool-service".into(),
                }]
            } else {
                source_chain
            },
            caller,
        }))
    }

    /// Verifies that a durable call still maps to the exact frozen definition and source chain.
    ///
    /// Implementations must not silently accept a same-named tool from a newer generation.
    fn validate_call_binding(
        &self,
        call: &ToolCall,
        binding: Option<&ToolCallBinding>,
    ) -> Result<(), CoreError> {
        let Some(binding) = binding else {
            return Ok(());
        };
        let expected = self
            .bind_call(call, binding.caller.clone())?
            .ok_or_else(|| CoreError::Execution("tool binding is unavailable".into()))?;
        if &expected != binding {
            return Err(CoreError::Execution(format!(
                "tool {} no longer matches its durable definition and source binding",
                call.name
            )));
        }
        Ok(())
    }

    /// Returns a stable, secret-free source chain for a callable tool definition.
    fn source_provenance(&self, _: &zeta_protocol::ToolName) -> Vec<ToolSourceProvenance> {
        Vec::new()
    }

    /// Selects the exact definitions visible to the next model invocation.
    ///
    /// Implementations with no deferred catalog retain the complete definition set. Registry-backed
    /// implementations must include their direct tools and may additionally expose only the
    /// generation-validated names supplied in `activated`.
    fn model_definitions(
        &self,
        activated: &BTreeSet<zeta_protocol::ToolName>,
    ) -> Result<Vec<ToolDefinition>, CoreError> {
        let _ = activated;
        Ok(self.definitions())
    }

    /// Freezes definitions and optional generation-bound binding authority for one model call.
    ///
    /// A reloadable implementation must override this method so a registry publication between
    /// model request and response cannot rebind a returned call to a newer same-named tool.
    fn model_catalog_snapshot(
        &self,
        activated: &BTreeSet<zeta_protocol::ToolName>,
    ) -> Result<ModelToolCatalogSnapshot, CoreError> {
        Ok(ModelToolCatalogSnapshot::new(
            self.model_definitions(activated)?,
        ))
    }

    /// Interprets one successful tool result as additive model-tool activation.
    ///
    /// Ordinary tools return no names. A tool-search implementation must validate its own result,
    /// registry generation, binding identity, and definition digest before returning names. This
    /// method never executes a tool or changes the live registry.
    fn activated_tool_names(
        &self,
        _: &ToolCall,
        _: &str,
    ) -> Result<Vec<zeta_protocol::ToolName>, CoreError> {
        Ok(Vec::new())
    }

    /// Returns a durable client interaction required to execute this exact Tool Call.
    ///
    /// Ordinary host and MCP tools return `None`. Dynamic tools return a request carrying the same
    /// Tool Call identity. Core persists and routes the request only after policy authorization;
    /// implementations must not contact a client or perform the action from this method.
    fn execution_interaction(
        &self,
        _: &ToolCall,
    ) -> Result<Option<zeta_protocol::AgentRequest>, CoreError> {
        Ok(None)
    }

    /// Converts a resolved execution interaction into the canonical Tool execution outcome.
    ///
    /// Implementations must validate request kind, Tool Call identity, and output shape. Returning
    /// `None` declares that the response is not owned by this tool service.
    fn resolve_execution_interaction(
        &self,
        _: &ToolCall,
        _: &zeta_protocol::AgentRequest,
        _: &zeta_protocol::AgentResponse,
    ) -> Result<Option<ToolExecutionOutput>, CoreError> {
        Ok(None)
    }

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
    execution: Option<ToolExecutionIdentity>,
    read_paths: BTreeSet<PathBuf>,
    available_tools: BTreeSet<zeta_protocol::ToolName>,
    activated_skills: Vec<zeta_protocol::FrozenSkillActivation>,
}

impl ToolExecutionFacts {
    pub(crate) fn for_turn(
        snapshot: &crate::ThreadSnapshot,
        turn_id: &zeta_protocol::TurnId,
        available_tools: impl IntoIterator<Item = zeta_protocol::ToolName>,
    ) -> Result<Self, CoreError> {
        let turn = snapshot
            .turns
            .iter()
            .find(|turn| &turn.turn_id == turn_id)
            .ok_or_else(|| CoreError::NotFound(turn_id.to_string()))?;
        let mut calls = std::collections::BTreeMap::new();
        let mut facts = Self {
            execution: Some(ToolExecutionIdentity {
                session_id: snapshot.session_id.clone(),
                thread_id: snapshot.thread_id.clone(),
                turn_id: turn_id.clone(),
                model: turn.model.clone(),
                policy_revision: turn.policy_revision.clone(),
            }),
            read_paths: BTreeSet::new(),
            available_tools: available_tools.into_iter().collect(),
            activated_skills: turn.activated_skills.clone(),
        };
        for item in &snapshot.items {
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
        Ok(facts)
    }

    pub fn read_paths(&self) -> impl Iterator<Item = &PathBuf> {
        self.read_paths.iter()
    }

    /// Returns the exact durable Thread/Turn identity executing the current Tool Call.
    pub fn execution_identity(&self) -> Option<&ToolExecutionIdentity> {
        self.execution.as_ref()
    }

    /// Returns the host tool names from which the child capability ceiling may be derived.
    pub fn available_tools(&self) -> impl Iterator<Item = &zeta_protocol::ToolName> {
        self.available_tools.iter()
    }

    /// Returns the exact Skill versions already frozen for the current Turn.
    pub fn activated_skills(&self) -> &[zeta_protocol::FrozenSkillActivation] {
        &self.activated_skills
    }
}

/// Durable caller identity supplied to a Tool Service without granting Thread mutation access.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolExecutionIdentity {
    session_id: zeta_protocol::SessionId,
    thread_id: zeta_protocol::ThreadId,
    turn_id: zeta_protocol::TurnId,
    model: Option<zeta_protocol::ModelRef>,
    policy_revision: String,
}

impl ToolExecutionIdentity {
    pub fn session_id(&self) -> &zeta_protocol::SessionId {
        &self.session_id
    }

    pub fn thread_id(&self) -> &zeta_protocol::ThreadId {
        &self.thread_id
    }

    pub fn turn_id(&self) -> &zeta_protocol::TurnId {
        &self.turn_id
    }

    pub fn model(&self) -> Option<&zeta_protocol::ModelRef> {
        self.model.as_ref()
    }

    pub fn policy_revision(&self) -> &str {
        &self.policy_revision
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
