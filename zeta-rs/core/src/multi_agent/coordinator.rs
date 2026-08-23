use super::AgentTreeLimits;
use super::budget::validate_spawn_capacity;
use crate::CommandDisposition;
use crate::CoreError;
use crate::InterruptTurnRequest;
use crate::SequenceExpectation;
use crate::SessionCoordinator;
use crate::SpawnAgentThreadRequest;
use crate::StartTurnDisposition;
use crate::StartTurnRequest;
use crate::ThreadSnapshot;
use crate::thread_reducer::satisfied_agent_join;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeSet;
use std::sync::Arc;
use zeta_protocol::AgentContextContent;
use zeta_protocol::AgentContextMode;
use zeta_protocol::AgentContextSeed;
use zeta_protocol::AgentContextSource;
use zeta_protocol::AgentJoin;
use zeta_protocol::AgentJoinId;
use zeta_protocol::AgentJoinPolicy;
use zeta_protocol::AgentJoinStatus;
use zeta_protocol::AgentMaterializedContext;
use zeta_protocol::AgentMessage;
use zeta_protocol::AgentMessageContent;
use zeta_protocol::AgentMessageId;
use zeta_protocol::AgentMessageProvenance;
use zeta_protocol::AgentRoleSnapshot;
use zeta_protocol::CommandId;
use zeta_protocol::ContentDigest;
use zeta_protocol::ContextSeedDigest;
use zeta_protocol::DelegatedCapabilityScope;
use zeta_protocol::DelegatedPolicyCeiling;
use zeta_protocol::DelegatedTask;
use zeta_protocol::DelegationArtifactRef;
use zeta_protocol::DelegationId;
use zeta_protocol::DelegationResult;
use zeta_protocol::DelegationResultDigest;
use zeta_protocol::DelegationResultStatus;
use zeta_protocol::SessionId;
use zeta_protocol::SessionStatus;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadSequenceRange;
use zeta_protocol::TurnId;
use zeta_protocol::TurnStatus;
use zeta_protocol::UserInput;

const MAX_TASK_BYTES: usize = 256 * 1024;
const MAX_ROLE_BYTES: usize = 64 * 1024;
const MAX_RESULT_BYTES: usize = 256 * 1024;
const MAX_MATERIALIZED_CONTEXT_BYTES: usize = 2 * 1024 * 1024;

/// Complete caller intent required to create one child Agent Thread.
pub struct SpawnAgentRequest {
    pub delegation_id: DelegationId,
    pub session_id: SessionId,
    pub parent_thread_id: ThreadId,
    pub parent_turn_id: TurnId,
    pub task: DelegatedTask,
    pub role: AgentRoleSnapshot,
    pub inheritance: AgentContextMode,
    pub policy_ceiling: DelegatedPolicyCeiling,
    pub capability_scope: DelegatedCapabilityScope,
}

/// Durable child identities returned after the spawn saga and initial Turn acceptance complete.
pub struct SpawnedAgent {
    pub delegation_id: DelegationId,
    pub child_thread_id: ThreadId,
    pub child_turn_id: TurnId,
    pub context_seed: AgentContextSeed,
    pub session_sequence: u64,
    pub child_sequence: u64,
    pub disposition: CommandDisposition,
}

/// Terminal child result material to commit and deliver to its parent.
pub struct CompleteDelegationRequest {
    pub parent_thread_id: ThreadId,
    pub delegation_id: DelegationId,
    pub status: DelegationResultStatus,
    pub summary: String,
    pub artifacts: Vec<DelegationArtifactRef>,
}

/// Caller intent for one durable cross-Thread steering message.
pub struct SendAgentMessageRequest {
    pub message_id: AgentMessageId,
    pub delegation_id: Option<DelegationId>,
    pub sender_thread_id: ThreadId,
    pub receiver_thread_id: ThreadId,
    pub text: String,
    pub provenance: AgentMessageProvenance,
}

/// Delivery result after sender outbox and receiver inbox facts are both durable.
pub struct DeliveredAgentMessage {
    pub message: AgentMessage,
    pub sender_sequence: u64,
    pub receiver_sequence: u64,
}

/// Caller intent for one durable parent-side join over a frozen delegation set.
pub struct JoinAgentsRequest {
    pub join_id: AgentJoinId,
    pub parent_thread_id: ThreadId,
    pub policy: AgentJoinPolicy,
    /// Optional exact target set. When absent, All/Any/Quorum freeze every current child.
    pub delegations: Option<Vec<DelegationId>>,
}

/// Current durable join projection after result facts have been evaluated.
pub struct JoinedAgents {
    pub join: AgentJoin,
    pub results: Vec<DelegationResult>,
}

/// Coordinates durable Agent spawn and cross-Thread delivery without owning any Thread context.
pub struct MultiAgentCoordinator {
    sessions: Arc<SessionCoordinator>,
    limits: AgentTreeLimits,
}

impl MultiAgentCoordinator {
    pub fn new(sessions: Arc<SessionCoordinator>, limits: AgentTreeLimits) -> Self {
        Self { sessions, limits }
    }

    /// Commits a parent request, creates and seeds its child Thread, then accepts the initial Turn.
    pub fn spawn(&self, request: SpawnAgentRequest) -> Result<SpawnedAgent, CoreError> {
        validate_spawn_request(&request)?;
        let parent = self
            .sessions
            .threads()
            .read_thread(&request.parent_thread_id)?;
        if parent.session_id != request.session_id {
            return Err(CoreError::InvalidInput(
                "delegation parent belongs to another Session".into(),
            ));
        }
        if let Some(existing) = parent.delegations.get(&request.delegation_id) {
            validate_replayed_spawn(&request, &existing.seed)?;
            validate_context_seed_digest(&existing.seed)?;
            return self.finish_spawn(existing.seed.clone());
        }
        let session = self.sessions.read_session(&request.session_id)?;
        if session.status != SessionStatus::Active {
            return Err(CoreError::InvalidInput(
                "Agent can only be spawned in an active Session".into(),
            ));
        }
        validate_spawn_capacity(&session, &request.parent_thread_id, self.limits)?;
        let materialized_context = self.materialize_context(&request, &parent)?;
        let seed =
            build_context_seed_with_materialized(request, parent.sequence, materialized_context)?;
        self.sessions
            .threads()
            .record_delegation_requested(&seed.parent_thread_id, seed.clone())?;
        self.finish_spawn(seed)
    }

    /// Commits one child result, delivers it exactly once, and records the parent projection.
    pub fn complete_delegation(
        &self,
        request: CompleteDelegationRequest,
    ) -> Result<DelegationResult, CoreError> {
        if request.summary.trim().is_empty() || request.summary.len() > MAX_RESULT_BYTES {
            return Err(CoreError::InvalidInput(
                "delegation result summary must be non-empty and bounded".into(),
            ));
        }
        let parent = self
            .sessions
            .threads()
            .read_thread(&request.parent_thread_id)?;
        let delegation = parent
            .delegations
            .get(&request.delegation_id)
            .ok_or_else(|| CoreError::NotFound(request.delegation_id.to_string()))?;
        let child_thread_id = delegation.child_thread_id.clone().ok_or_else(|| {
            CoreError::InvalidInput("delegation has not started a child Thread".into())
        })?;
        let child = self.sessions.threads().read_thread(&child_thread_id)?;
        if child
            .turns
            .iter()
            .any(|turn| !is_terminal_turn(turn.status))
        {
            return Err(CoreError::InvalidInput(
                "delegation result requires every child Turn to be terminal".into(),
            ));
        }
        if let Some(existing) = child
            .produced_delegation_results
            .get(&request.delegation_id)
            .cloned()
        {
            if existing.status != request.status
                || existing.summary != request.summary
                || existing.artifacts != request.artifacts
            {
                return Err(CoreError::CommandConflict);
            }
            self.deliver_result(&request.parent_thread_id, existing.clone())?;
            return Ok(existing);
        }
        let mut result = DelegationResult {
            delegation_id: request.delegation_id,
            child_thread_id: child_thread_id.clone(),
            status: request.status,
            summary: request.summary,
            artifacts: request.artifacts,
            source_range: ThreadSequenceRange {
                start_sequence: 1,
                end_sequence: child.sequence,
            },
            digest: DelegationResultDigest::new(format!("sha256:{}", "0".repeat(64)))
                .expect("static result digest placeholder is valid"),
        };
        result.digest = delegation_result_digest(&result)?;
        self.sessions
            .threads()
            .record_delegation_result_produced(&child_thread_id, result.clone())?;
        self.deliver_result(&request.parent_thread_id, result.clone())?;
        Ok(result)
    }

    /// Sends a steering message through a durable sender outbox and deduplicated receiver inbox.
    pub fn send_message(
        &self,
        request: SendAgentMessageRequest,
    ) -> Result<DeliveredAgentMessage, CoreError> {
        if request.text.trim().is_empty() || request.text.len() > MAX_TASK_BYTES {
            return Err(CoreError::InvalidInput(
                "Agent message must be non-empty and bounded".into(),
            ));
        }
        let sender = self
            .sessions
            .threads()
            .read_thread(&request.sender_thread_id)?;
        let receiver = self
            .sessions
            .threads()
            .read_thread(&request.receiver_thread_id)?;
        if sender.session_id != receiver.session_id {
            return Err(CoreError::InvalidInput(
                "Agent messages cannot cross product Sessions".into(),
            ));
        }
        let message = match sender.sent_agent_messages.get(&request.message_id) {
            Some(existing) => {
                if existing.delegation_id != request.delegation_id
                    || existing.receiver_thread_id != request.receiver_thread_id
                    || existing.provenance != request.provenance
                    || existing.content != (AgentMessageContent::Instruction { text: request.text })
                {
                    return Err(CoreError::CommandConflict);
                }
                existing.clone()
            }
            None => AgentMessage {
                message_id: request.message_id,
                delegation_id: request.delegation_id,
                sender_thread_id: request.sender_thread_id,
                receiver_thread_id: request.receiver_thread_id,
                sender_sequence: sender.sequence,
                content: AgentMessageContent::Instruction { text: request.text },
                provenance: request.provenance,
            },
        };
        let sender_sequence = self
            .sessions
            .threads()
            .record_agent_message_sent(&message.sender_thread_id, message.clone())?;
        let receiver_sequence = self
            .sessions
            .threads()
            .record_agent_message_received(&message.receiver_thread_id, message.clone())?;
        Ok(DeliveredAgentMessage {
            message,
            sender_sequence,
            receiver_sequence,
        })
    }

    /// Creates or replays a durable join and evaluates it from parent result facts.
    pub fn join(&self, request: JoinAgentsRequest) -> Result<JoinedAgents, CoreError> {
        let parent = self
            .sessions
            .threads()
            .read_thread(&request.parent_thread_id)?;
        if let Some(existing) = parent.agent_joins.get(&request.join_id) {
            if existing.policy != request.policy
                || request
                    .delegations
                    .as_ref()
                    .is_some_and(|targets| targets != &existing.delegations)
            {
                return Err(CoreError::CommandConflict);
            }
        } else {
            let delegations =
                frozen_join_targets(&parent, &request.policy, request.delegations.as_deref())?;
            self.sessions.threads().record_agent_join_requested(
                &request.parent_thread_id,
                AgentJoin {
                    join_id: request.join_id.clone(),
                    parent_thread_id: request.parent_thread_id.clone(),
                    policy: request.policy,
                    delegations,
                    status: AgentJoinStatus::Waiting,
                    satisfied_by: Vec::new(),
                },
            )?;
        }
        self.refresh_join(&request.parent_thread_id, &request.join_id)
    }

    /// Durably cancels one delegation and all of its live descendants.
    pub fn cancel_delegation(
        &self,
        parent_thread_id: &ThreadId,
        delegation_id: &DelegationId,
    ) -> Result<DelegationResult, CoreError> {
        let parent = self.sessions.threads().read_thread(parent_thread_id)?;
        if let Some(result) = parent
            .received_delegation_results
            .get(delegation_id)
            .cloned()
        {
            return Ok(result);
        }
        self.sessions
            .threads()
            .record_delegation_cancellation_requested(parent_thread_id, delegation_id.clone())?;
        self.finish_cancellation(parent_thread_id, delegation_id)
    }

    /// Propagates cancellation from one parent Thread to every live descendant delegation.
    pub fn cancel_descendants(&self, parent_thread_id: &ThreadId) -> Result<usize, CoreError> {
        let parent = self.sessions.threads().read_thread(parent_thread_id)?;
        let pending = parent
            .delegations
            .keys()
            .filter(|delegation_id| {
                !parent
                    .received_delegation_results
                    .contains_key(*delegation_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        for delegation_id in &pending {
            self.cancel_delegation(parent_thread_id, delegation_id)?;
        }
        Ok(pending.len())
    }

    /// Reconciles incomplete spawn and delivery sagas from durable Session and Thread facts.
    pub fn recover_session(&self, session_id: &SessionId) -> Result<Vec<SpawnedAgent>, CoreError> {
        let session = self.sessions.recover_session(session_id)?;
        let mut recovered = Vec::new();
        for membership in &session.threads {
            let parent = self
                .sessions
                .threads()
                .read_thread(&membership.membership.thread_id)?;
            for delegation in parent.delegations.values() {
                recovered.push(self.finish_spawn(delegation.seed.clone())?);
            }
        }
        for membership in &session.threads {
            let sender = self
                .sessions
                .threads()
                .read_thread(&membership.membership.thread_id)?;
            for message in sender.sent_agent_messages.values() {
                self.sessions
                    .threads()
                    .record_agent_message_received(&message.receiver_thread_id, message.clone())?;
            }
            if let Some(seed) = &sender.agent_context_seed
                && let Some(result) = sender
                    .produced_delegation_results
                    .get(&seed.delegation_id)
                    .cloned()
            {
                self.deliver_result(&seed.parent_thread_id, result)?;
            }
            for delegation in sender.delegations.values() {
                if delegation.cancellation_requested
                    && !sender
                        .received_delegation_results
                        .contains_key(&delegation.seed.delegation_id)
                {
                    self.finish_cancellation(&sender.thread_id, &delegation.seed.delegation_id)?;
                }
            }
            self.refresh_waiting_joins(&sender.thread_id)?;
        }
        Ok(recovered)
    }

    fn finish_spawn(&self, seed: AgentContextSeed) -> Result<SpawnedAgent, CoreError> {
        validate_context_seed_digest(&seed)?;
        let session = self.sessions.read_session(
            &self
                .sessions
                .threads()
                .read_thread(&seed.parent_thread_id)?
                .session_id,
        )?;
        let spawned = self.sessions.spawn_agent_thread(SpawnAgentThreadRequest {
            command_id: spawn_command_id(&seed.delegation_id)?,
            session_id: session.session_id.clone(),
            expected_sequence: SequenceExpectation::Any,
            parent_thread_id: seed.parent_thread_id.clone(),
            title: seed.task.title.clone(),
            context_seed: seed.clone(),
        })?;
        self.sessions.threads().record_delegation_started(
            &seed.parent_thread_id,
            seed.delegation_id.clone(),
            spawned.thread_id.clone(),
        )?;
        let initial_turn = self.sessions.start_turn(
            &session.session_id,
            &spawned.thread_id,
            StartTurnRequest {
                command_id: initial_turn_command_id(&seed.delegation_id)?,
                expected_sequence: SequenceExpectation::Any,
                model: seed.role.model.clone(),
                policy_revision: seed.policy_ceiling.policy_revision.clone(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                resource_budget: None,
                activated_skills: seed.capability_scope.skills.clone(),
                input: vec![UserInput::Text {
                    text: seed.task.instructions.clone(),
                }],
            },
        )?;
        Ok(SpawnedAgent {
            delegation_id: seed.delegation_id.clone(),
            child_thread_id: spawned.thread_id,
            child_turn_id: initial_turn.turn_id,
            context_seed: seed,
            session_sequence: spawned.sequence,
            child_sequence: initial_turn.sequence,
            disposition: if spawned.disposition == CommandDisposition::Replayed
                || initial_turn.disposition == StartTurnDisposition::Replayed
            {
                CommandDisposition::Replayed
            } else {
                CommandDisposition::Committed
            },
        })
    }

    fn deliver_result(
        &self,
        parent_thread_id: &ThreadId,
        result: DelegationResult,
    ) -> Result<(), CoreError> {
        let child = self
            .sessions
            .threads()
            .read_thread(&result.child_thread_id)?;
        let message_id = result_message_id(&result.delegation_id)?;
        let message = child
            .sent_agent_messages
            .get(&message_id)
            .cloned()
            .unwrap_or_else(|| AgentMessage {
                message_id,
                delegation_id: Some(result.delegation_id.clone()),
                sender_thread_id: result.child_thread_id.clone(),
                receiver_thread_id: parent_thread_id.clone(),
                sender_sequence: child.sequence,
                content: AgentMessageContent::Result {
                    result: result.clone(),
                },
                provenance: AgentMessageProvenance::Agent,
            });
        self.sessions
            .threads()
            .record_agent_message_sent(&message.sender_thread_id, message.clone())?;
        self.sessions
            .threads()
            .record_agent_message_received(parent_thread_id, message)?;
        self.sessions
            .threads()
            .record_delegation_result_received(parent_thread_id, result)?;
        self.refresh_waiting_joins(parent_thread_id)?;
        Ok(())
    }

    fn refresh_join(
        &self,
        parent_thread_id: &ThreadId,
        join_id: &AgentJoinId,
    ) -> Result<JoinedAgents, CoreError> {
        let parent = self.sessions.threads().read_thread(parent_thread_id)?;
        let join = parent
            .agent_joins
            .get(join_id)
            .cloned()
            .ok_or_else(|| CoreError::NotFound(join_id.to_string()))?;
        if join.status == AgentJoinStatus::Waiting
            && let Some(satisfied_by) = satisfied_agent_join(&parent, &join)
        {
            self.sessions.threads().record_agent_join_satisfied(
                parent_thread_id,
                join_id.clone(),
                satisfied_by,
            )?;
        }
        let parent = self.sessions.threads().read_thread(parent_thread_id)?;
        let join = parent
            .agent_joins
            .get(join_id)
            .cloned()
            .ok_or_else(|| CoreError::NotFound(join_id.to_string()))?;
        let results = join
            .satisfied_by
            .iter()
            .filter_map(|delegation_id| {
                parent
                    .received_delegation_results
                    .get(delegation_id)
                    .cloned()
            })
            .collect();
        Ok(JoinedAgents { join, results })
    }

    fn refresh_waiting_joins(&self, parent_thread_id: &ThreadId) -> Result<(), CoreError> {
        let parent = self.sessions.threads().read_thread(parent_thread_id)?;
        let waiting = parent
            .agent_joins
            .values()
            .filter(|join| join.status == AgentJoinStatus::Waiting)
            .map(|join| join.join_id.clone())
            .collect::<Vec<_>>();
        for join_id in waiting {
            self.refresh_join(parent_thread_id, &join_id)?;
        }
        Ok(())
    }

    fn finish_cancellation(
        &self,
        parent_thread_id: &ThreadId,
        delegation_id: &DelegationId,
    ) -> Result<DelegationResult, CoreError> {
        let parent = self.sessions.threads().read_thread(parent_thread_id)?;
        let delegation = parent
            .delegations
            .get(delegation_id)
            .ok_or_else(|| CoreError::NotFound(delegation_id.to_string()))?;
        let child_thread_id = delegation.child_thread_id.clone().ok_or_else(|| {
            CoreError::InvalidInput("cannot cancel a delegation before child creation".into())
        })?;
        let child = self.sessions.threads().read_thread(&child_thread_id)?;
        if let Some(result) = child
            .produced_delegation_results
            .get(delegation_id)
            .cloned()
        {
            self.deliver_result(parent_thread_id, result.clone())?;
            return Ok(result);
        }
        self.sessions.threads().record_agent_cancellation_received(
            &child_thread_id,
            delegation_id.clone(),
            parent_thread_id.clone(),
        )?;
        self.cancel_descendants(&child_thread_id)?;
        loop {
            let child = self.sessions.threads().read_thread(&child_thread_id)?;
            let interruptible = child
                .turns
                .iter()
                .filter(|turn| is_interruptible_turn(turn.status))
                .map(|turn| turn.turn_id.clone())
                .collect::<Vec<_>>();
            if interruptible.is_empty() {
                break;
            }
            for turn_id in interruptible {
                self.sessions.threads().interrupt_turn(
                    &child_thread_id,
                    InterruptTurnRequest {
                        command_id: cancellation_command_id(delegation_id, &turn_id)?,
                        expected_sequence: SequenceExpectation::Any,
                        turn_id,
                    },
                )?;
            }
        }
        self.complete_delegation(CompleteDelegationRequest {
            parent_thread_id: parent_thread_id.clone(),
            delegation_id: delegation_id.clone(),
            status: DelegationResultStatus::Cancelled,
            summary: "Cancelled because an ancestor Agent was interrupted.".into(),
            artifacts: Vec::new(),
        })
    }

    fn materialize_context(
        &self,
        request: &SpawnAgentRequest,
        parent: &ThreadSnapshot,
    ) -> Result<Vec<AgentMaterializedContext>, CoreError> {
        let materialized = match &request.inheritance {
            AgentContextMode::Fresh => Vec::new(),
            AgentContextMode::Selected { sources } => {
                let mut materialized = Vec::with_capacity(sources.len());
                for (index, source) in sources.iter().enumerate() {
                    if sources[..index].contains(source) {
                        return Err(CoreError::InvalidInput(
                            "Selected Agent context contains a duplicate source".into(),
                        ));
                    }
                    materialized.push(self.materialize_source(&request.session_id, source)?);
                }
                materialized
            }
            AgentContextMode::ForkedPrefix { selection } => {
                materialize_parent_prefix(parent, selection)?
            }
        };
        let encoded = serde_json::to_vec(&materialized).map_err(|error| {
            CoreError::Journal(format!("cannot encode materialized Agent context: {error}"))
        })?;
        if encoded.len() > MAX_MATERIALIZED_CONTEXT_BYTES {
            return Err(CoreError::Context(format!(
                "materialized Agent context exceeds {MAX_MATERIALIZED_CONTEXT_BYTES} bytes"
            )));
        }
        Ok(materialized)
    }

    fn materialize_source(
        &self,
        session_id: &SessionId,
        source: &AgentContextSource,
    ) -> Result<AgentMaterializedContext, CoreError> {
        let (source_thread_id, source_sequence) = source_identity(source);
        let snapshot = self.sessions.threads().read_thread(source_thread_id)?;
        if &snapshot.session_id != session_id {
            return Err(CoreError::InvalidInput(
                "Selected Agent context cannot cross product Sessions".into(),
            ));
        }
        if snapshot.sequence != source_sequence {
            return Err(CoreError::Context(format!(
                "Selected Agent context source {} moved from sequence {} to {}",
                source_thread_id, source_sequence, snapshot.sequence
            )));
        }
        match source {
            AgentContextSource::Item { item_id, .. } => {
                let item = snapshot
                    .items
                    .iter()
                    .find(|item| item.item_id() == item_id)
                    .ok_or_else(|| CoreError::NotFound(item_id.to_string()))?;
                materialize_item(source.clone(), item)
            }
            AgentContextSource::Checkpoint { checkpoint_id, .. } => {
                let checkpoint = snapshot
                    .context_checkpoints
                    .iter()
                    .find(|checkpoint| &checkpoint.checkpoint_id == checkpoint_id)
                    .ok_or_else(|| CoreError::NotFound(checkpoint_id.to_string()))?;
                materialized_entry(
                    source.clone(),
                    AgentContextContent::Checkpoint {
                        summary: checkpoint.summary.clone(),
                    },
                )
            }
        }
    }
}

fn frozen_join_targets(
    parent: &ThreadSnapshot,
    policy: &AgentJoinPolicy,
    selected: Option<&[DelegationId]>,
) -> Result<Vec<DelegationId>, CoreError> {
    let targets = match (policy, selected) {
        (AgentJoinPolicy::Explicit { delegations }, None) => delegations.clone(),
        (AgentJoinPolicy::Explicit { delegations }, Some(selected)) if delegations == selected => {
            selected.to_vec()
        }
        (AgentJoinPolicy::Explicit { .. }, Some(_)) => {
            return Err(CoreError::InvalidInput(
                "explicit Agent join targets disagree with its policy".into(),
            ));
        }
        (
            AgentJoinPolicy::All | AgentJoinPolicy::Any | AgentJoinPolicy::Quorum { .. },
            Some(selected),
        ) => selected.to_vec(),
        (AgentJoinPolicy::All | AgentJoinPolicy::Any | AgentJoinPolicy::Quorum { .. }, None) => {
            parent.delegations.keys().cloned().collect()
        }
    };
    if targets.is_empty() {
        return Err(CoreError::InvalidInput(
            "Agent join requires at least one delegation".into(),
        ));
    }
    let mut unique = BTreeSet::new();
    if targets.iter().any(|delegation_id| {
        !unique.insert(delegation_id.clone()) || !parent.delegations.contains_key(delegation_id)
    }) {
        return Err(CoreError::InvalidInput(
            "Agent join targets must be unique parent delegations".into(),
        ));
    }
    if let AgentJoinPolicy::Quorum { count } = policy
        && (*count == 0 || usize::try_from(*count).map_or(true, |count| count > targets.len()))
    {
        return Err(CoreError::InvalidInput(
            "Agent join quorum must fit its frozen target set".into(),
        ));
    }
    Ok(targets)
}

fn cancellation_command_id(
    delegation_id: &DelegationId,
    turn_id: &TurnId,
) -> Result<CommandId, CoreError> {
    CommandId::new(format!("agent-cancel:{delegation_id}:{turn_id}"))
        .map_err(|error| CoreError::InvalidInput(error.to_string()))
}

fn is_interruptible_turn(status: TurnStatus) -> bool {
    matches!(
        status,
        TurnStatus::Created
            | TurnStatus::Running
            | TurnStatus::WaitingForApproval
            | TurnStatus::WaitingForUserInput
            | TurnStatus::WaitingForCapability
    )
}

fn materialize_parent_prefix(
    parent: &ThreadSnapshot,
    selection: &zeta_protocol::ForkedAgentContext,
) -> Result<Vec<AgentMaterializedContext>, CoreError> {
    let selected_turns = match selection {
        zeta_protocol::ForkedAgentContext::Full
        | zeta_protocol::ForkedAgentContext::CheckpointAndTail => None,
        zeta_protocol::ForkedAgentContext::LastTurns { count } => {
            let count = usize::try_from(*count).unwrap_or(usize::MAX);
            Some(
                parent
                    .turns
                    .iter()
                    .rev()
                    .take(count)
                    .map(|turn| turn.turn_id.clone())
                    .collect::<BTreeSet<_>>(),
            )
        }
    };
    let mut materialized = Vec::new();
    let checkpoint_end = if matches!(
        selection,
        zeta_protocol::ForkedAgentContext::CheckpointAndTail
    ) {
        if let Some(checkpoint) = parent.context_checkpoints.last() {
            materialized.push(materialized_entry(
                AgentContextSource::Checkpoint {
                    source_thread_id: parent.thread_id.clone(),
                    source_sequence: parent.sequence,
                    checkpoint_id: checkpoint.checkpoint_id.clone(),
                },
                AgentContextContent::Checkpoint {
                    summary: checkpoint.summary.clone(),
                },
            )?);
            checkpoint.covered.end_sequence
        } else {
            0
        }
    } else {
        0
    };
    for item in &parent.items {
        if selected_turns
            .as_ref()
            .is_some_and(|turns| !turns.contains(item.turn_id()))
            || parent
                .item_sequences
                .get(item.item_id())
                .is_some_and(|sequence| *sequence <= checkpoint_end)
        {
            continue;
        }
        materialized.push(materialize_item(
            AgentContextSource::Item {
                source_thread_id: parent.thread_id.clone(),
                source_sequence: parent.sequence,
                item_id: item.item_id().clone(),
            },
            item,
        )?);
    }
    Ok(materialized)
}

fn materialize_item(
    source: AgentContextSource,
    item: &zeta_protocol::ThreadItem,
) -> Result<AgentMaterializedContext, CoreError> {
    let content = match item {
        zeta_protocol::ThreadItem::UserMessage { text, .. } => {
            AgentContextContent::UserText { text: text.clone() }
        }
        zeta_protocol::ThreadItem::UserImage { url, .. } => {
            AgentContextContent::UserImage { url: url.clone() }
        }
        zeta_protocol::ThreadItem::UserImageAttachment { attachment, .. } => {
            AgentContextContent::UserImageAttachment {
                attachment: attachment.clone(),
            }
        }
        zeta_protocol::ThreadItem::AgentMessage { text, .. } => {
            AgentContextContent::AssistantText { text: text.clone() }
        }
        zeta_protocol::ThreadItem::Reasoning { text, .. } => {
            AgentContextContent::Reasoning { text: text.clone() }
        }
        zeta_protocol::ThreadItem::Plan { text, .. } => {
            AgentContextContent::Plan { text: text.clone() }
        }
        zeta_protocol::ThreadItem::ToolCall {
            name,
            arguments_json,
            ..
        } => AgentContextContent::ToolCall {
            name: name.clone(),
            arguments_json: arguments_json.clone(),
        },
        zeta_protocol::ThreadItem::ToolResult { text, is_error, .. } => {
            AgentContextContent::ToolResult {
                text: text.clone(),
                is_error: *is_error,
            }
        }
    };
    materialized_entry(source, content)
}

fn materialized_entry(
    source: AgentContextSource,
    content: AgentContextContent,
) -> Result<AgentMaterializedContext, CoreError> {
    let encoded = serde_json::to_vec(&content).map_err(|error| {
        CoreError::Journal(format!("cannot encode inherited Agent context: {error}"))
    })?;
    Ok(AgentMaterializedContext {
        source,
        content,
        content_digest: ContentDigest::sha256(&encoded),
    })
}

fn source_identity(source: &AgentContextSource) -> (&ThreadId, u64) {
    match source {
        AgentContextSource::Item {
            source_thread_id,
            source_sequence,
            ..
        }
        | AgentContextSource::Checkpoint {
            source_thread_id,
            source_sequence,
            ..
        } => (source_thread_id, *source_sequence),
    }
}

fn validate_spawn_request(request: &SpawnAgentRequest) -> Result<(), CoreError> {
    if request.task.title.trim().is_empty()
        || request.task.title.len() > 256
        || request.task.instructions.trim().is_empty()
        || request.task.instructions.len() > MAX_TASK_BYTES
        || request.role.name.trim().is_empty()
        || request.role.instructions.trim().is_empty()
        || request.role.instructions.len() > MAX_ROLE_BYTES
        || request.policy_ceiling.policy_revision.trim().is_empty()
    {
        return Err(CoreError::InvalidInput(
            "Agent spawn contains an empty or oversized required field".into(),
        ));
    }
    if let AgentContextMode::ForkedPrefix {
        selection: zeta_protocol::ForkedAgentContext::LastTurns { count },
    } = request.inheritance
        && count == 0
    {
        return Err(CoreError::InvalidInput(
            "forked Agent context must select at least one Turn".into(),
        ));
    }
    let mut tools = BTreeSet::new();
    if request
        .capability_scope
        .tools
        .iter()
        .any(|tool| !tools.insert(tool))
    {
        return Err(CoreError::InvalidInput(
            "Agent capability scope contains duplicate tools".into(),
        ));
    }
    let mut skills = BTreeSet::new();
    if request
        .capability_scope
        .skills
        .iter()
        .any(|skill| !skills.insert((&skill.id, &skill.content_digest)))
    {
        return Err(CoreError::InvalidInput(
            "Agent capability scope contains duplicate Skills".into(),
        ));
    }
    Ok(())
}

fn validate_replayed_spawn(
    request: &SpawnAgentRequest,
    seed: &AgentContextSeed,
) -> Result<(), CoreError> {
    if seed.delegation_id != request.delegation_id
        || seed.parent_thread_id != request.parent_thread_id
        || seed.parent_turn_id != request.parent_turn_id
        || seed.task != request.task
        || seed.role != request.role
        || seed.inheritance != request.inheritance
        || seed.policy_ceiling != request.policy_ceiling
        || seed.capability_scope != request.capability_scope
    {
        return Err(CoreError::CommandConflict);
    }
    Ok(())
}

#[cfg(test)]
fn build_context_seed(
    request: SpawnAgentRequest,
    parent_sequence: u64,
) -> Result<AgentContextSeed, CoreError> {
    build_context_seed_with_materialized(request, parent_sequence, Vec::new())
}

fn build_context_seed_with_materialized(
    request: SpawnAgentRequest,
    parent_sequence: u64,
    materialized_context: Vec<AgentMaterializedContext>,
) -> Result<AgentContextSeed, CoreError> {
    let mut seed = AgentContextSeed {
        delegation_id: request.delegation_id,
        parent_thread_id: request.parent_thread_id,
        parent_turn_id: request.parent_turn_id,
        parent_sequence,
        task: request.task,
        role: request.role,
        inheritance: request.inheritance,
        materialized_context,
        policy_ceiling: request.policy_ceiling,
        capability_scope: request.capability_scope,
        digest: ContextSeedDigest::new(format!("sha256:{}", "0".repeat(64)))
            .expect("static context seed digest placeholder is valid"),
    };
    seed.digest = context_seed_digest(&seed)?;
    Ok(seed)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ContextSeedMaterial<'a> {
    delegation_id: &'a DelegationId,
    parent_thread_id: &'a ThreadId,
    parent_turn_id: &'a TurnId,
    parent_sequence: u64,
    task: &'a DelegatedTask,
    role: &'a AgentRoleSnapshot,
    inheritance: &'a AgentContextMode,
    #[serde(skip_serializing_if = "<[AgentMaterializedContext]>::is_empty")]
    materialized_context: &'a [AgentMaterializedContext],
    policy_ceiling: &'a DelegatedPolicyCeiling,
    capability_scope: &'a DelegatedCapabilityScope,
}

fn context_seed_digest(seed: &AgentContextSeed) -> Result<ContextSeedDigest, CoreError> {
    let material = ContextSeedMaterial {
        delegation_id: &seed.delegation_id,
        parent_thread_id: &seed.parent_thread_id,
        parent_turn_id: &seed.parent_turn_id,
        parent_sequence: seed.parent_sequence,
        task: &seed.task,
        role: &seed.role,
        inheritance: &seed.inheritance,
        materialized_context: &seed.materialized_context,
        policy_ceiling: &seed.policy_ceiling,
        capability_scope: &seed.capability_scope,
    };
    let bytes = serde_json::to_vec(&material).map_err(|error| {
        CoreError::Journal(format!("cannot encode Agent context seed: {error}"))
    })?;
    ContextSeedDigest::new(format!("sha256:{:x}", Sha256::digest(bytes)))
        .map_err(|error| CoreError::Journal(error.to_string()))
}

pub(crate) fn validate_context_seed_digest(seed: &AgentContextSeed) -> Result<(), CoreError> {
    if context_seed_digest(seed)? != seed.digest {
        return Err(CoreError::Context(
            "Agent context seed digest does not match its immutable material".into(),
        ));
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DelegationResultMaterial<'a> {
    delegation_id: &'a DelegationId,
    child_thread_id: &'a ThreadId,
    status: DelegationResultStatus,
    summary: &'a str,
    artifacts: &'a [DelegationArtifactRef],
    source_range: ThreadSequenceRange,
}

fn delegation_result_digest(
    result: &DelegationResult,
) -> Result<DelegationResultDigest, CoreError> {
    let material = DelegationResultMaterial {
        delegation_id: &result.delegation_id,
        child_thread_id: &result.child_thread_id,
        status: result.status,
        summary: &result.summary,
        artifacts: &result.artifacts,
        source_range: result.source_range,
    };
    let bytes = serde_json::to_vec(&material)
        .map_err(|error| CoreError::Journal(format!("cannot encode delegation result: {error}")))?;
    DelegationResultDigest::new(format!("sha256:{:x}", Sha256::digest(bytes)))
        .map_err(|error| CoreError::Journal(error.to_string()))
}

pub(crate) fn validate_delegation_result_digest(
    result: &DelegationResult,
) -> Result<(), CoreError> {
    if delegation_result_digest(result)? != result.digest {
        return Err(CoreError::Journal(
            "delegation result digest does not match its immutable material".into(),
        ));
    }
    Ok(())
}

fn spawn_command_id(delegation_id: &DelegationId) -> Result<CommandId, CoreError> {
    CommandId::new(format!("agent-spawn:{delegation_id}"))
        .map_err(|error| CoreError::InvalidInput(error.to_string()))
}

fn initial_turn_command_id(delegation_id: &DelegationId) -> Result<CommandId, CoreError> {
    CommandId::new(format!("agent-initial-turn:{delegation_id}"))
        .map_err(|error| CoreError::InvalidInput(error.to_string()))
}

fn result_message_id(delegation_id: &DelegationId) -> Result<AgentMessageId, CoreError> {
    AgentMessageId::new(format!("agent-result:{delegation_id}"))
        .map_err(|error| CoreError::InvalidInput(error.to_string()))
}

fn is_terminal_turn(status: TurnStatus) -> bool {
    matches!(
        status,
        TurnStatus::Completed | TurnStatus::Failed | TurnStatus::Interrupted
    )
}

#[cfg(test)]
#[path = "coordinator_tests.rs"]
mod tests;
