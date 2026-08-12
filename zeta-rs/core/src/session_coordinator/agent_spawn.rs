use super::CommandDisposition;
use super::SequenceExpectation;
use super::SessionBatchCommand;
use super::SessionCommandResult;
use super::SessionCoordinator;
use super::SessionThreadResult;
use super::validate_command;
use super::validate_expectation;
use crate::CoreError;
use zeta_protocol::AgentContextSeed;
use zeta_protocol::CommandId;
use zeta_protocol::SessionCommand;
use zeta_protocol::SessionEvent;
use zeta_protocol::SessionId;
use zeta_protocol::SessionThread;
use zeta_protocol::SessionThreadStatus;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadOrigin;
use zeta_session_store::SessionCommandReceipt;

/// Session topology request for one durable parent-to-child Agent spawn.
pub struct SpawnAgentThreadRequest {
    pub command_id: CommandId,
    pub session_id: SessionId,
    pub expected_sequence: SequenceExpectation,
    pub parent_thread_id: ThreadId,
    pub title: String,
    pub context_seed: AgentContextSeed,
}

impl SessionCoordinator {
    /// Plans, creates, seeds, and attaches one child Agent Thread through a recoverable saga.
    pub fn spawn_agent_thread(
        &self,
        request: SpawnAgentThreadRequest,
    ) -> Result<SessionThreadResult, CoreError> {
        validate_command(&request.command_id, &request.title)?;
        let parent = self.threads.read_thread(&request.parent_thread_id)?;
        if parent.session_id != request.session_id {
            return Err(CoreError::InvalidInput(
                "Agent spawn parent belongs to another Session".into(),
            ));
        }
        if request.context_seed.parent_thread_id != request.parent_thread_id {
            return Err(CoreError::InvalidInput(
                "Agent context seed parent does not match the spawn request".into(),
            ));
        }
        let delegation = parent
            .delegations
            .get(&request.context_seed.delegation_id)
            .ok_or_else(|| CoreError::NotFound(request.context_seed.delegation_id.to_string()))?;
        if delegation.seed != request.context_seed {
            return Err(CoreError::CommandConflict);
        }

        let command = SessionCommand::SpawnAgentThread {
            parent_thread_id: request.parent_thread_id.clone(),
            parent_turn_id: request.context_seed.parent_turn_id.clone(),
            delegation_id: request.context_seed.delegation_id.clone(),
            context_seed_digest: request.context_seed.digest.clone(),
            title: request.title.clone(),
        };
        let _lease = self.acquire_writer_lease(&request.session_id)?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| CoreError::Journal("Session state lock poisoned".into()))?;
        let snapshot = sessions
            .get_mut(&request.session_id)
            .ok_or_else(|| CoreError::NotFound(request.session_id.to_string()))?;
        if let Some(existing) = snapshot
            .commands
            .iter()
            .find(|existing| existing.receipt.command_id == request.command_id)
            .cloned()
        {
            if existing.receipt.command != command {
                return Err(CoreError::CommandConflict);
            }
            let SessionCommandResult::ThreadCreated { thread_id } = existing.result else {
                return Err(CoreError::Journal(
                    "spawn-Agent command has an invalid result".into(),
                ));
            };
            if snapshot.threads.iter().any(|thread| {
                thread.membership.thread_id == thread_id
                    && thread.membership.status == SessionThreadStatus::Creating
            }) {
                self.finish_thread_creation(snapshot, &thread_id)?;
            }
            return Ok(SessionThreadResult {
                thread_id,
                sequence: snapshot.sequence,
                disposition: CommandDisposition::Replayed,
            });
        }
        validate_expectation(request.expected_sequence, snapshot.sequence)?;

        let thread_id = ThreadId::new(self.next_identifier("agent_thread"))
            .expect("generated Agent Thread ID is non-empty");
        let origin = ThreadOrigin::AgentSpawn {
            parent_thread_id: request.parent_thread_id,
            parent_sequence: request.context_seed.parent_sequence,
            delegation_id: request.context_seed.delegation_id.clone(),
        };
        let event = SessionEvent::AgentThreadCreationPlanned {
            session_id: request.session_id.clone(),
            thread: SessionThread {
                thread_id: thread_id.clone(),
                origin,
                status: SessionThreadStatus::Creating,
            },
            title: request.title,
            context_seed: Box::new(request.context_seed),
        };
        let (planned, batch) = self.project_batch(
            Some(snapshot.clone()),
            &request.session_id,
            vec![event],
            SessionBatchCommand::FirstEvent(SessionCommandReceipt {
                command_id: request.command_id,
                command,
            }),
        )?;
        self.store.append_batch(&batch)?;
        *snapshot = planned;
        self.finish_thread_creation(snapshot, &thread_id)?;
        Ok(SessionThreadResult {
            thread_id,
            sequence: snapshot.sequence,
            disposition: CommandDisposition::Committed,
        })
    }
}
