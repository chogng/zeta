use crate::local_tools::local_policy_revision;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use zeta_action_policy::ActionDigest;
use zeta_action_policy::ActionKind;
use zeta_action_policy::ActionPolicyRevision;
use zeta_action_policy::ActionProvenance;
use zeta_action_policy::ActionReviewRequest;
use zeta_action_policy::ActionSource;
use zeta_action_policy::CapabilitySet;
use zeta_action_policy::ResolvedAction;
use zeta_action_policy::SandboxCompatibility;
use zeta_async_utils::CancellationToken;
use zeta_core::CoreError;
use zeta_core::JoinAgentsRequest;
use zeta_core::MultiAgentCoordinator;
use zeta_core::SendAgentMessageRequest;
use zeta_core::SpawnAgentRequest;
use zeta_core::ThreadController;
use zeta_core::ToolAuthorization;
use zeta_core::ToolExecutionFacts;
use zeta_core::ToolOutputSink;
use zeta_core::ToolService;
use zeta_core::TurnExecutionBackend;
use zeta_protocol::AgentContextMode;
use zeta_protocol::AgentContextSource;
use zeta_protocol::AgentJoinId;
use zeta_protocol::AgentJoinPolicy;
use zeta_protocol::AgentJoinStatus;
use zeta_protocol::AgentMessageId;
use zeta_protocol::AgentMessageProvenance;
use zeta_protocol::ContextCheckpointId;
use zeta_protocol::DelegatedPolicyCeiling;
use zeta_protocol::DelegatedTask;
use zeta_protocol::DelegationId;
use zeta_protocol::ForkedAgentContext;
use zeta_protocol::ItemId;
use zeta_protocol::ThreadId;
use zeta_protocol::ToolCall;
use zeta_protocol::ToolDefinition;
use zeta_protocol::ToolExecutionOutput;
use zeta_protocol::ToolName;

mod agent_selection;

use agent_selection::ResolvedAgentSelection;
use agent_selection::resolve_agent_selection;

pub(crate) const SPAWN_AGENT_TOOL_NAME: &str = "spawn_agent";
pub(crate) const SEND_AGENT_MESSAGE_TOOL_NAME: &str = "send_agent_message";
pub(crate) const WAIT_AGENT_TOOL_NAME: &str = "wait_agent";
const MAX_WAIT: Duration = Duration::from_secs(30);

pub(super) struct MultiAgentToolService {
    coordinator: Arc<MultiAgentCoordinator>,
    threads: Arc<ThreadController>,
    turn_backend: Arc<dyn TurnExecutionBackend>,
    definitions: Vec<ToolDefinition>,
    action_policy_revision: ActionPolicyRevision,
    customizations: Option<Arc<super::dir_contributions::DirContributions>>,
}

impl MultiAgentToolService {
    pub(super) fn new(
        coordinator: Arc<MultiAgentCoordinator>,
        threads: Arc<ThreadController>,
        turn_backend: Arc<dyn TurnExecutionBackend>,
    ) -> Self {
        Self {
            coordinator,
            threads,
            turn_backend,
            definitions: vec![spawn_definition(), send_definition(), wait_definition()],
            action_policy_revision: local_policy_revision(),
            customizations: None,
        }
    }

    pub(super) fn with_action_policy_revision(mut self, revision: ActionPolicyRevision) -> Self {
        self.action_policy_revision = revision;
        self
    }

    pub(super) fn with_dir_contributions(
        mut self,
        customizations: Arc<super::dir_contributions::DirContributions>,
    ) -> Self {
        self.customizations = Some(customizations);
        self
    }

    fn execute_with_context(
        &self,
        call: &ToolCall,
        cancellation: &CancellationToken,
        facts: &ToolExecutionFacts,
    ) -> Result<ToolExecutionOutput, CoreError> {
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        let identity = facts.execution_identity().ok_or_else(|| {
            CoreError::Execution("Agent coordination tool requires durable caller identity".into())
        })?;
        match call.name.as_str() {
            SPAWN_AGENT_TOOL_NAME => {
                let arguments: SpawnArguments = decode_arguments(&call.arguments)?;
                let selection = self.resolve_agent(&arguments, facts)?;
                let delegation_id = DelegationId::new(format!("tool:{}", call.id))
                    .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
                let spawned = self.coordinator.spawn(SpawnAgentRequest {
                    delegation_id: delegation_id.clone(),
                    session_id: identity.session_id().clone(),
                    parent_thread_id: identity.thread_id().clone(),
                    parent_turn_id: identity.turn_id().clone(),
                    task: DelegatedTask {
                        title: arguments.name.unwrap_or_else(|| "subagent".into()),
                        instructions: arguments.task,
                    },
                    role: selection.role.clone(),
                    inheritance: spawn_context(arguments.context)?,
                    policy_ceiling: DelegatedPolicyCeiling {
                        policy_revision: identity.policy_revision().into(),
                    },
                    capability_scope: selection.capability_scope,
                })?;
                self.turn_backend
                    .start(&spawned.child_thread_id, &spawned.child_turn_id)?;
                success(json!({
                    "delegation_id": delegation_id,
                    "child_thread_id": spawned.child_thread_id,
                    "child_turn_id": spawned.child_turn_id,
                    "agent": selection.role.definition,
                    "status": "running"
                }))
            }
            SEND_AGENT_MESSAGE_TOOL_NAME => {
                let arguments: SendArguments = decode_arguments(&call.arguments)?;
                let delegation_id = DelegationId::new(arguments.delegation_id)
                    .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
                let child_thread_id =
                    child_thread_for(&self.threads, identity.thread_id(), &delegation_id)?;
                let delivered = self.coordinator.send_message(SendAgentMessageRequest {
                    message_id: AgentMessageId::new(format!("tool:{}", call.id))
                        .map_err(|error| CoreError::InvalidInput(error.to_string()))?,
                    delegation_id: Some(delegation_id),
                    sender_thread_id: identity.thread_id().clone(),
                    receiver_thread_id: child_thread_id,
                    text: arguments.message,
                    provenance: AgentMessageProvenance::Agent,
                })?;
                success(json!({
                    "message_id": delivered.message.message_id,
                    "receiver_thread_id": delivered.message.receiver_thread_id,
                    "status": "delivered"
                }))
            }
            WAIT_AGENT_TOOL_NAME => {
                let arguments: WaitArguments = decode_arguments(&call.arguments)?;
                let (delegations, policy) = wait_join_policy(&arguments)?;
                let timeout = Duration::from_millis(
                    arguments
                        .timeout_ms
                        .unwrap_or(30_000)
                        .min(MAX_WAIT.as_millis() as u64),
                );
                self.wait_for_join(
                    identity.thread_id(),
                    AgentJoinId::new(format!("tool:{}", call.id))
                        .map_err(|error| CoreError::InvalidInput(error.to_string()))?,
                    delegations,
                    policy,
                    timeout,
                    cancellation,
                )
            }
            _ => Err(CoreError::Policy(format!(
                "tool is not available: {}",
                call.name
            ))),
        }
    }

    fn resolve_agent(
        &self,
        arguments: &SpawnArguments,
        facts: &ToolExecutionFacts,
    ) -> Result<ResolvedAgentSelection, CoreError> {
        let identity = facts.execution_identity().ok_or_else(|| {
            CoreError::Execution("Agent coordination tool requires durable caller identity".into())
        })?;
        let agent_snapshots = self
            .customizations
            .as_ref()
            .map(|customizations| customizations.agent_snapshots_for(identity.session_id()))
            .unwrap_or_default();
        let instruction_snapshots = self
            .customizations
            .as_ref()
            .map(|customizations| customizations.instruction_snapshots_for(identity.session_id()))
            .unwrap_or_default();
        resolve_agent_selection(
            arguments.agent.as_deref(),
            &arguments.task,
            identity.model(),
            facts.available_tools().cloned().collect(),
            facts.activated_skills(),
            &agent_snapshots,
            &instruction_snapshots,
        )
    }

    fn wait_for_join(
        &self,
        parent_thread_id: &ThreadId,
        join_id: AgentJoinId,
        delegations: Option<Vec<DelegationId>>,
        policy: AgentJoinPolicy,
        timeout: Duration,
        cancellation: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        let deadline = Instant::now() + timeout;
        loop {
            cancellation
                .check()
                .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
            self.complete_terminal_children(parent_thread_id, delegations.as_deref())?;
            let joined = self.coordinator.join(JoinAgentsRequest {
                join_id: join_id.clone(),
                parent_thread_id: parent_thread_id.clone(),
                policy: policy.clone(),
                delegations: delegations.clone(),
            })?;
            if joined.join.status == AgentJoinStatus::Satisfied {
                return success(json!({
                    "join_id": joined.join.join_id,
                    "status": joined.join.status,
                    "satisfied_by": joined.join.satisfied_by,
                    "results": joined.results
                }));
            }
            if Instant::now() >= deadline {
                return success(json!({
                    "join_id": joined.join.join_id,
                    "delegations": joined.join.delegations,
                    "status": joined.join.status
                }));
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn complete_terminal_children(
        &self,
        parent_thread_id: &ThreadId,
        selected: Option<&[DelegationId]>,
    ) -> Result<(), CoreError> {
        let parent = self.threads.read_thread(parent_thread_id)?;
        let delegation_ids = selected
            .map(<[DelegationId]>::to_vec)
            .unwrap_or_else(|| parent.delegations.keys().cloned().collect());
        for delegation_id in delegation_ids {
            if parent
                .received_delegation_results
                .contains_key(&delegation_id)
            {
                continue;
            }
            let child_thread_id =
                child_thread_for(&self.threads, parent_thread_id, &delegation_id)?;
            self.coordinator
                .reconcile_terminal_delegation(&child_thread_id)?;
        }
        Ok(())
    }
}

impl ToolService for MultiAgentToolService {
    fn definitions(&self) -> Vec<ToolDefinition> {
        self.definitions.clone()
    }

    fn prepare(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        if !self
            .definitions
            .iter()
            .any(|definition| definition.name == call.name)
        {
            return Err(CoreError::Policy(format!(
                "tool is not available: {}",
                call.name
            )));
        }
        let canonical = serde_json::to_vec(&json!({
            "tool": call.name,
            "arguments": call.arguments,
        }))
        .map_err(|error| CoreError::Policy(error.to_string()))?;
        Ok(ActionReviewRequest::new(
            ResolvedAction::new(
                ActionDigest::from_canonical_bytes(canonical),
                ActionKind::SystemOperation,
                format!("coordinate child Agent through {}", call.name),
                CapabilitySet::new([]),
            ),
            ActionProvenance::new(ActionSource::BuiltInTool, call.name.as_str()),
            SandboxCompatibility::NotApplicable {
                reason: "Agent coordination only mutates durable Zeta Session/Thread state".into(),
            },
            self.action_policy_revision.clone(),
        ))
    }

    fn execute(
        &self,
        _: &ToolCall,
        _: &ToolAuthorization,
        _: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        Err(CoreError::Execution(
            "Agent coordination tool requires durable execution facts".into(),
        ))
    }

    fn execute_with_facts(
        &self,
        call: &ToolCall,
        _: &ToolAuthorization,
        cancellation: &CancellationToken,
        facts: &ToolExecutionFacts,
    ) -> Result<ToolExecutionOutput, CoreError> {
        self.execute_with_context(call, cancellation, facts)
    }

    fn execute_streaming_with_facts(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
        facts: &ToolExecutionFacts,
        _: &mut dyn ToolOutputSink,
    ) -> Result<ToolExecutionOutput, CoreError> {
        self.execute_with_facts(call, authorization, cancellation, facts)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpawnArguments {
    task: String,
    name: Option<String>,
    agent: Option<String>,
    context: Option<SpawnContextArguments>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpawnContextArguments {
    mode: SpawnContextMode,
    count: Option<u32>,
    sources: Option<Vec<SpawnContextSourceArguments>>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
enum SpawnContextMode {
    Fresh,
    Full,
    LastTurns,
    CheckpointAndTail,
    Selected,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpawnContextSourceArguments {
    kind: SpawnContextSourceKind,
    source_thread_id: String,
    source_sequence: u64,
    item_id: Option<String>,
    checkpoint_id: Option<String>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
enum SpawnContextSourceKind {
    Item,
    Checkpoint,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SendArguments {
    delegation_id: String,
    message: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WaitArguments {
    delegation_id: Option<String>,
    delegation_ids: Option<Vec<String>>,
    policy: Option<WaitPolicy>,
    quorum: Option<u32>,
    timeout_ms: Option<u64>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
enum WaitPolicy {
    All,
    Any,
    Quorum,
}

fn spawn_context(context: Option<SpawnContextArguments>) -> Result<AgentContextMode, CoreError> {
    let Some(context) = context else {
        return Ok(AgentContextMode::Fresh);
    };
    match context.mode {
        SpawnContextMode::Fresh => {
            require_absent_context_fields(context.count, context.sources.as_ref())?;
            Ok(AgentContextMode::Fresh)
        }
        SpawnContextMode::Full => {
            require_absent_context_fields(context.count, context.sources.as_ref())?;
            Ok(AgentContextMode::ForkedPrefix {
                selection: ForkedAgentContext::Full,
            })
        }
        SpawnContextMode::LastTurns => {
            if context.sources.is_some() {
                return Err(CoreError::InvalidInput(
                    "lastTurns Agent context cannot include selected sources".into(),
                ));
            }
            let count = context.count.filter(|count| *count > 0).ok_or_else(|| {
                CoreError::InvalidInput("lastTurns Agent context requires count > 0".into())
            })?;
            Ok(AgentContextMode::ForkedPrefix {
                selection: ForkedAgentContext::LastTurns { count },
            })
        }
        SpawnContextMode::CheckpointAndTail => {
            require_absent_context_fields(context.count, context.sources.as_ref())?;
            Ok(AgentContextMode::ForkedPrefix {
                selection: ForkedAgentContext::CheckpointAndTail,
            })
        }
        SpawnContextMode::Selected => {
            if context.count.is_some() {
                return Err(CoreError::InvalidInput(
                    "Selected Agent context cannot include a Turn count".into(),
                ));
            }
            let sources = context.sources.ok_or_else(|| {
                CoreError::InvalidInput("Selected Agent context requires sources".into())
            })?;
            if sources.is_empty() {
                return Err(CoreError::InvalidInput(
                    "Selected Agent context requires at least one source".into(),
                ));
            }
            Ok(AgentContextMode::Selected {
                sources: sources
                    .into_iter()
                    .map(spawn_context_source)
                    .collect::<Result<Vec<_>, _>>()?,
            })
        }
    }
}

fn require_absent_context_fields(
    count: Option<u32>,
    sources: Option<&Vec<SpawnContextSourceArguments>>,
) -> Result<(), CoreError> {
    if count.is_some() || sources.is_some() {
        Err(CoreError::InvalidInput(
            "Agent context mode contains fields owned by another mode".into(),
        ))
    } else {
        Ok(())
    }
}

fn spawn_context_source(
    source: SpawnContextSourceArguments,
) -> Result<AgentContextSource, CoreError> {
    let source_thread_id = ThreadId::new(source.source_thread_id)
        .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
    match source.kind {
        SpawnContextSourceKind::Item => {
            if source.checkpoint_id.is_some() {
                return Err(CoreError::InvalidInput(
                    "Item Agent context source cannot include checkpoint_id".into(),
                ));
            }
            Ok(AgentContextSource::Item {
                source_thread_id,
                source_sequence: source.source_sequence,
                item_id: ItemId::new(source.item_id.ok_or_else(|| {
                    CoreError::InvalidInput("Item Agent context source requires item_id".into())
                })?)
                .map_err(|error| CoreError::InvalidInput(error.to_string()))?,
            })
        }
        SpawnContextSourceKind::Checkpoint => {
            if source.item_id.is_some() {
                return Err(CoreError::InvalidInput(
                    "checkpoint Agent context source cannot include item_id".into(),
                ));
            }
            Ok(AgentContextSource::Checkpoint {
                source_thread_id,
                source_sequence: source.source_sequence,
                checkpoint_id: ContextCheckpointId::new(source.checkpoint_id.ok_or_else(|| {
                    CoreError::InvalidInput(
                        "checkpoint Agent context source requires checkpoint_id".into(),
                    )
                })?)
                .map_err(|error| CoreError::InvalidInput(error.to_string()))?,
            })
        }
    }
}

fn wait_join_policy(
    arguments: &WaitArguments,
) -> Result<(Option<Vec<DelegationId>>, AgentJoinPolicy), CoreError> {
    if arguments.delegation_id.is_some() && arguments.delegation_ids.is_some() {
        return Err(CoreError::InvalidInput(
            "wait_agent accepts delegation_id or delegation_ids, not both".into(),
        ));
    }
    let delegations = match (&arguments.delegation_id, &arguments.delegation_ids) {
        (Some(delegation_id), None) => {
            Some(vec![DelegationId::new(delegation_id.clone()).map_err(
                |error| CoreError::InvalidInput(error.to_string()),
            )?])
        }
        (None, Some(delegation_ids)) => {
            if delegation_ids.is_empty() {
                return Err(CoreError::InvalidInput(
                    "delegation_ids must not be empty".into(),
                ));
            }
            Some(
                delegation_ids
                    .iter()
                    .cloned()
                    .map(DelegationId::new)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| CoreError::InvalidInput(error.to_string()))?,
            )
        }
        (None, None) => None,
        (Some(_), Some(_)) => unreachable!("conflicting fields rejected above"),
    };
    let policy = match arguments.policy {
        None | Some(WaitPolicy::All) => AgentJoinPolicy::All,
        Some(WaitPolicy::Any) => AgentJoinPolicy::Any,
        Some(WaitPolicy::Quorum) => AgentJoinPolicy::Quorum {
            count: arguments
                .quorum
                .filter(|count| *count > 0)
                .ok_or_else(|| CoreError::InvalidInput("quorum wait requires quorum > 0".into()))?,
        },
    };
    if !matches!(arguments.policy, Some(WaitPolicy::Quorum)) && arguments.quorum.is_some() {
        return Err(CoreError::InvalidInput(
            "quorum is only valid for quorum wait policy".into(),
        ));
    }
    Ok((delegations, policy))
}

fn child_thread_for(
    threads: &ThreadController,
    parent_thread_id: &ThreadId,
    delegation_id: &DelegationId,
) -> Result<ThreadId, CoreError> {
    threads
        .read_thread(parent_thread_id)?
        .delegations
        .get(delegation_id)
        .and_then(|delegation| delegation.child_thread_id.clone())
        .ok_or_else(|| CoreError::NotFound(delegation_id.to_string()))
}

fn decode_arguments<T: for<'de> Deserialize<'de>>(value: &Value) -> Result<T, CoreError> {
    serde_json::from_value(value.clone())
        .map_err(|error| CoreError::InvalidInput(format!("invalid tool arguments: {error}")))
}

fn success(value: Value) -> Result<ToolExecutionOutput, CoreError> {
    serde_json::to_string(&value)
        .map(ToolExecutionOutput::Success)
        .map_err(|error| CoreError::Execution(error.to_string()))
}

fn definition(name: &str, description: &str, parameters: Value) -> ToolDefinition {
    ToolDefinition {
        name: ToolName::new(name).expect("static Agent tool name is valid"),
        description: description.into(),
        parameters,
        strict: true,
    }
}

fn spawn_definition() -> ToolDefinition {
    definition(
        SPAWN_AGENT_TOOL_NAME,
        "Creates an independent child Agent Thread for one bounded task and returns immediately. The child has isolated history and receives only its frozen role, delegated task, active Skills, and allowed tool names. Use wait_agent with the returned delegation_id to collect its result.",
        json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "The complete, bounded task the child Agent should execute independently."
                },
                "name": {
                    "type": ["string", "null"],
                    "description": "An optional short label for the delegation and child Thread."
                },
                "agent": {
                    "type": ["string", "null"],
                    "description": "An optional exact Agent definition name. null lets the host select one unique metadata match or use the general fallback."
                },
                "context": {
                    "type": ["object", "null"],
                    "description": "Optional immutable context inheritance. null means fresh.",
                    "properties": {
                        "mode": {
                            "type": "string",
                            "enum": ["fresh", "full", "lastTurns", "checkpointAndTail", "selected"]
                        },
                        "count": {
                            "type": ["integer", "null"],
                            "minimum": 1
                        },
                        "sources": {
                            "type": ["array", "null"],
                            "items": {
                                "type": "object",
                                "properties": {
                                    "kind": { "type": "string", "enum": ["item", "checkpoint"] },
                                    "sourceThreadId": { "type": "string" },
                                    "sourceSequence": { "type": "integer", "minimum": 1 },
                                    "itemId": { "type": ["string", "null"] },
                                    "checkpointId": { "type": ["string", "null"] }
                                },
                                "required": ["kind", "sourceThreadId", "sourceSequence", "itemId", "checkpointId"],
                                "additionalProperties": false
                            }
                        }
                    },
                    "required": ["mode", "count", "sources"],
                    "additionalProperties": false
                }
            },
            "required": ["task", "name", "agent", "context"],
            "additionalProperties": false
        }),
    )
}

fn send_definition() -> ToolDefinition {
    definition(
        SEND_AGENT_MESSAGE_TOOL_NAME,
        "Durably sends additional instructions to a child Agent using exact-once cross-Thread delivery.",
        json!({
            "type": "object",
            "properties": {
                "delegation_id": {
                    "type": "string",
                    "description": "The delegation_id returned by spawn_agent."
                },
                "message": {
                    "type": "string",
                    "description": "The additional instruction to deliver to the child Agent."
                }
            },
            "required": ["delegation_id", "message"],
            "additionalProperties": false
        }),
    )
}

fn wait_definition() -> ToolDefinition {
    definition(
        WAIT_AGENT_TOOL_NAME,
        "Creates a durable All/Any/Quorum join over one, several, or all current child Agents; waits up to 30 seconds and returns satisfied results or the durable waiting join.",
        json!({
            "type": "object",
            "properties": {
                "delegation_id": {
                    "type": ["string", "null"],
                    "description": "One delegation_id. Use null when delegation_ids or all current children are selected."
                },
                "delegation_ids": {
                    "type": ["array", "null"],
                    "items": { "type": "string" },
                    "minItems": 1,
                    "description": "An exact frozen target set, or null to use delegation_id/all current children."
                },
                "policy": {
                    "type": ["string", "null"],
                    "enum": ["all", "any", "quorum", null],
                    "description": "Join policy. null defaults to all."
                },
                "quorum": {
                    "type": ["integer", "null"],
                    "minimum": 1,
                    "description": "Required result count for quorum; null for other policies."
                },
                "timeout_ms": {
                    "type": ["integer", "null"],
                    "minimum": 0,
                    "maximum": 30000,
                    "description": "Maximum wait in milliseconds. Defaults to 30000 and is capped at 30000."
                }
            },
            "required": ["delegation_id", "delegation_ids", "policy", "quorum", "timeout_ms"],
            "additionalProperties": false
        }),
    )
}

#[cfg(test)]
#[path = "multi_agent_tools_tests.rs"]
mod tests;
