use crate::{
    CoreError, CreateAgentThreadRequest, CreateThreadRequest, LeaseGuard, SessionCommandResult,
    SessionSnapshot, StartContextCompactionRequest, StartShellTurnRequest, StartTurnRequest,
    StartTurnResult, SteerTurnRequest, SteerTurnResult, ThreadController, WriterLease,
    reduce_session_event,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_protocol::TurnId;
use zeta_protocol::{
    CommandId, ModelRef, SessionCommand, SessionEvent, SessionId, SessionStatus, SessionThread,
    SessionThreadStatus, SessionUpdate, SessionUpdateEnvelope, ThreadId, ThreadOrigin, TurnStatus,
    WorkspaceBinding,
};
use zeta_session_store::{
    AppendSessionBatchResult, CURRENT_SESSION_EVENT_SCHEMA_VERSION, SessionCommandReceipt,
    SessionEventBatch, SessionEventId, SessionStore, SessionStoreError, SessionTimestamp,
    StoredSessionEvent, validate_session_append_batch,
};

mod agent_spawn;

pub use agent_spawn::SpawnAgentThreadRequest;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequenceExpectation {
    Any,
    Exact(u64),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandDisposition {
    Committed,
    Replayed,
}

pub struct CreateSessionRequest {
    pub command_id: CommandId,
    pub title: String,
    pub model: Option<ModelRef>,
    pub workspace: Option<WorkspaceBinding>,
}

pub struct SetSessionModelRequest {
    pub command_id: CommandId,
    pub session_id: SessionId,
    pub expected_sequence: SequenceExpectation,
    pub model: ModelRef,
}

pub struct CreateSessionResult {
    pub session_id: SessionId,
    pub sequence: u64,
    pub disposition: CommandDisposition,
}

pub struct CreateSessionThreadRequest {
    pub command_id: CommandId,
    pub session_id: SessionId,
    pub expected_sequence: SequenceExpectation,
    pub title: String,
}

pub struct ForkSessionThreadRequest {
    pub command_id: CommandId,
    pub session_id: SessionId,
    pub expected_sequence: SequenceExpectation,
    pub parent_thread_id: ThreadId,
    pub title: String,
}

pub struct RewindSessionThreadRequest {
    pub command_id: CommandId,
    pub session_id: SessionId,
    pub expected_sequence: SequenceExpectation,
    pub parent_thread_id: ThreadId,
    pub before_turn_id: TurnId,
    pub title: String,
}

pub struct ArchiveSessionThreadRequest {
    pub command_id: CommandId,
    pub session_id: SessionId,
    pub expected_sequence: SequenceExpectation,
    pub thread_id: ThreadId,
}

pub struct SessionLifecycleRequest {
    pub command_id: CommandId,
    pub session_id: SessionId,
    pub expected_sequence: SequenceExpectation,
}

pub struct SessionThreadResult {
    pub thread_id: ThreadId,
    pub sequence: u64,
    pub disposition: CommandDisposition,
}

pub struct SessionMutationResult {
    pub sequence: u64,
    pub disposition: CommandDisposition,
}

enum SessionBatchCommand {
    None,
    FirstEvent(SessionCommandReceipt),
}

/// Coordinates only product Session structure while child Thread execution remains independent.
pub struct SessionCoordinator {
    store: Arc<dyn SessionStore>,
    writer_lease: Option<Arc<dyn WriterLease<SessionId>>>,
    threads: Arc<ThreadController>,
    sessions: Mutex<BTreeMap<SessionId, SessionSnapshot>>,
    next_id: AtomicU64,
}

impl SessionCoordinator {
    pub fn with_store(store: Arc<dyn SessionStore>, threads: Arc<ThreadController>) -> Self {
        Self {
            store,
            writer_lease: None,
            threads,
            sessions: Mutex::new(BTreeMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    pub fn with_store_and_lease(
        store: Arc<dyn SessionStore>,
        threads: Arc<ThreadController>,
        writer_lease: Arc<dyn WriterLease<SessionId>>,
    ) -> Self {
        Self {
            store,
            writer_lease: Some(writer_lease),
            threads,
            sessions: Mutex::new(BTreeMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    pub fn threads(&self) -> &Arc<ThreadController> {
        &self.threads
    }

    /// Starts a model Turn only while the owning Session and membership are active.
    ///
    /// The Session lock is held across validation and the child mutation so an App Server
    /// The canonical Session stop operation cannot commit after this check and still accept a new Turn.
    pub fn start_turn(
        &self,
        session_id: &SessionId,
        thread_id: &ThreadId,
        request: StartTurnRequest,
    ) -> Result<StartTurnResult, CoreError> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| CoreError::Journal("Session state lock poisoned".into()))?;
        self.validate_active_thread(&sessions, session_id, thread_id)?;
        self.threads.start_turn(thread_id, request)
    }

    /// Starts standalone context compaction while the owning Session and membership are active.
    pub fn start_context_compaction(
        &self,
        session_id: &SessionId,
        thread_id: &ThreadId,
        request: StartContextCompactionRequest,
    ) -> Result<StartTurnResult, CoreError> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| CoreError::Journal("Session state lock poisoned".into()))?;
        self.validate_active_thread(&sessions, session_id, thread_id)?;
        self.threads.start_context_compaction(thread_id, request)
    }

    /// Starts a shell Turn only while the owning Session and membership are active.
    ///
    /// This is the Session-aware counterpart to the lower-level Thread controller operation;
    /// adapters should use it for product Turns so a concurrently stopped Session is respected.
    pub fn start_shell_turn(
        &self,
        session_id: &SessionId,
        thread_id: &ThreadId,
        request: StartShellTurnRequest,
    ) -> Result<StartTurnResult, CoreError> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| CoreError::Journal("Session state lock poisoned".into()))?;
        self.validate_active_thread(&sessions, session_id, thread_id)?;
        self.threads.start_shell_turn(thread_id, request)
    }

    /// Appends user input to an active product Turn while its Session membership remains active.
    pub fn steer_turn(
        &self,
        session_id: &SessionId,
        thread_id: &ThreadId,
        request: SteerTurnRequest,
    ) -> Result<SteerTurnResult, CoreError> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| CoreError::Journal("Session state lock poisoned".into()))?;
        self.validate_active_thread(&sessions, session_id, thread_id)?;
        self.threads.steer_turn(thread_id, request)
    }

    pub fn create_session(
        &self,
        request: CreateSessionRequest,
    ) -> Result<CreateSessionResult, CoreError> {
        validate_command(&request.command_id, &request.title)?;
        let command = SessionCommand::Create {
            title: request.title.clone(),
            model: request.model.clone(),
            workspace: request.workspace.clone(),
        };
        let catalog_id = SessionId::new("__zeta_session_catalog__")
            .expect("static Session catalog ID is non-empty");
        let _catalog_lease = self.acquire_writer_lease(&catalog_id)?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| CoreError::Journal("Session state lock poisoned".into()))?;

        for session_id in self.store.list_session_ids()? {
            let snapshot = match sessions.get(&session_id) {
                Some(snapshot) => snapshot.clone(),
                None => self.load_snapshot(&session_id)?,
            };
            if let Some(existing) = snapshot
                .commands
                .iter()
                .find(|existing| existing.receipt.command_id == request.command_id)
            {
                if existing.receipt.command != command
                    || existing.result != SessionCommandResult::SessionCreated
                {
                    return Err(CoreError::CommandConflict);
                }
                let result = CreateSessionResult {
                    session_id: snapshot.session_id.clone(),
                    sequence: existing.response_sequence,
                    disposition: CommandDisposition::Replayed,
                };
                sessions.insert(snapshot.session_id.clone(), snapshot);
                return Ok(result);
            }
        }

        let session_id = SessionId::new(self.next_identifier("session"))
            .expect("generated Session ID is non-empty");
        let _session_lease = self.acquire_writer_lease(&session_id)?;
        let (snapshot, batch) = self.project_batch(
            None,
            &session_id,
            vec![SessionEvent::SessionCreated {
                session_id: session_id.clone(),
                title: request.title,
                model: request.model,
                workspace: request.workspace,
            }],
            SessionBatchCommand::FirstEvent(SessionCommandReceipt {
                command_id: request.command_id,
                command,
            }),
        )?;
        self.store.append_batch(&batch)?;
        let result = CreateSessionResult {
            session_id: session_id.clone(),
            sequence: snapshot.sequence,
            disposition: CommandDisposition::Committed,
        };
        sessions.insert(session_id, snapshot);
        Ok(result)
    }

    pub fn create_thread(
        &self,
        request: CreateSessionThreadRequest,
    ) -> Result<SessionThreadResult, CoreError> {
        validate_command(&request.command_id, &request.title)?;
        self.plan_and_finish_thread(
            request.command_id,
            request.session_id,
            request.expected_sequence,
            SessionCommand::CreateThread {
                title: request.title.clone(),
            },
            ThreadOrigin::Root,
            request.title,
        )
    }

    pub fn set_model(
        &self,
        request: SetSessionModelRequest,
    ) -> Result<SessionMutationResult, CoreError> {
        validate_command_id(&request.command_id)?;
        let model = request.model;
        self.apply_single_command(
            request.session_id,
            request.expected_sequence,
            SessionCommandReceipt {
                command_id: request.command_id,
                command: SessionCommand::SetModel {
                    model: model.clone(),
                },
            },
            |session_id| SessionEvent::SessionModelChanged {
                session_id,
                model: model.clone(),
            },
            SessionCommandResult::SessionModelChanged {
                model: model.clone(),
            },
        )
    }

    pub fn fork_thread(
        &self,
        request: ForkSessionThreadRequest,
    ) -> Result<SessionThreadResult, CoreError> {
        validate_command(&request.command_id, &request.title)?;
        let parent = self.threads.read_thread(&request.parent_thread_id)?;
        if parent.session_id != request.session_id {
            return Err(CoreError::InvalidInput(
                "fork parent belongs to another Session".into(),
            ));
        }
        self.plan_and_finish_thread(
            request.command_id,
            request.session_id,
            request.expected_sequence,
            SessionCommand::ForkThread {
                parent_thread_id: request.parent_thread_id.clone(),
                title: request.title.clone(),
            },
            ThreadOrigin::Fork {
                parent_thread_id: request.parent_thread_id,
                parent_sequence: parent.sequence,
            },
            request.title,
        )
    }

    pub fn rewind_thread(
        &self,
        request: RewindSessionThreadRequest,
    ) -> Result<SessionThreadResult, CoreError> {
        validate_command(&request.command_id, &request.title)?;
        let parent = self.threads.read_thread(&request.parent_thread_id)?;
        if parent.session_id != request.session_id {
            return Err(CoreError::InvalidInput(
                "rewind parent belongs to another Session".into(),
            ));
        }
        if !parent
            .turns
            .iter()
            .any(|turn| turn.turn_id == request.before_turn_id)
        {
            return Err(CoreError::NotFound(request.before_turn_id.to_string()));
        }
        self.plan_and_finish_thread(
            request.command_id,
            request.session_id,
            request.expected_sequence,
            SessionCommand::RewindThread {
                parent_thread_id: request.parent_thread_id.clone(),
                before_turn_id: request.before_turn_id.clone(),
                title: request.title.clone(),
            },
            ThreadOrigin::Rewind {
                parent_thread_id: request.parent_thread_id,
                parent_sequence: parent.sequence,
                before_turn_id: request.before_turn_id,
            },
            request.title,
        )
    }

    pub fn archive_thread(
        &self,
        request: ArchiveSessionThreadRequest,
    ) -> Result<SessionMutationResult, CoreError> {
        validate_command_id(&request.command_id)?;
        let thread_id = request.thread_id;
        let command = SessionCommand::ArchiveThread {
            thread_id: thread_id.clone(),
        };
        self.apply_single_command(
            request.session_id,
            request.expected_sequence,
            SessionCommandReceipt {
                command_id: request.command_id,
                command,
            },
            |session_id| SessionEvent::ThreadArchived {
                session_id,
                thread_id: thread_id.clone(),
            },
            SessionCommandResult::ThreadArchived {
                thread_id: thread_id.clone(),
            },
        )
    }

    pub fn complete(
        &self,
        request: SessionLifecycleRequest,
    ) -> Result<SessionMutationResult, CoreError> {
        self.apply_lifecycle_command(request, SessionCommand::Complete)
    }

    pub fn archive(
        &self,
        request: SessionLifecycleRequest,
    ) -> Result<SessionMutationResult, CoreError> {
        self.apply_lifecycle_command(request, SessionCommand::Archive)
    }

    /// Stops a Session by durably archiving it and interrupting every active child Turn.
    ///
    /// The Session mutation is committed before cancellation is requested so the Session status
    /// prevents new Turns from starting while the child Thread transitions are being applied.
    /// The same command ID deterministically derives each child interrupt command, making a
    /// retry after a partial failure safe and allowing recovery to finish the cancellation pass.
    ///
    /// This is an internal Core execution action. The App Server owns the external
    /// lifecycle mapping (for example, a frontend Tab close); no canonical stop command is
    /// added to `zeta-protocol`.
    pub fn stop(
        &self,
        request: SessionLifecycleRequest,
    ) -> Result<SessionMutationResult, CoreError> {
        validate_command_id(&request.command_id)?;
        let _lease = self.acquire_writer_lease(&request.session_id)?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| CoreError::Journal("Session state lock poisoned".into()))?;
        let snapshot = sessions
            .get(&request.session_id)
            .cloned()
            .ok_or_else(|| CoreError::NotFound(request.session_id.to_string()))?;
        let receipt = SessionCommandReceipt {
            command_id: request.command_id.clone(),
            command: SessionCommand::Archive,
        };
        let thread_ids = snapshot
            .threads
            .iter()
            .map(|thread| thread.membership.thread_id.clone())
            .collect::<Vec<_>>();
        if let Some(existing) = snapshot
            .commands
            .iter()
            .find(|existing| existing.receipt.command_id == request.command_id)
        {
            if existing.receipt != receipt
                || existing.result != SessionCommandResult::SessionArchived
            {
                return Err(CoreError::CommandConflict);
            }
            self.interrupt_session_threads(&thread_ids, &request.command_id)?;
            return Ok(SessionMutationResult {
                sequence: existing.response_sequence,
                disposition: CommandDisposition::Replayed,
            });
        }
        validate_expectation(request.expected_sequence, snapshot.sequence)?;
        let (next, batch) = self.project_batch(
            Some(snapshot),
            &request.session_id,
            vec![SessionEvent::SessionArchived {
                session_id: request.session_id.clone(),
            }],
            SessionBatchCommand::FirstEvent(receipt),
        )?;
        self.store.append_batch(&batch)?;
        let sequence = next.sequence;
        sessions.insert(request.session_id.clone(), next);
        self.interrupt_session_threads(&thread_ids, &request.command_id)?;
        Ok(SessionMutationResult {
            sequence,
            disposition: CommandDisposition::Committed,
        })
    }

    pub fn read_session(&self, session_id: &SessionId) -> Result<SessionSnapshot, CoreError> {
        self.sessions
            .lock()
            .map_err(|_| CoreError::Journal("Session state lock poisoned".into()))?
            .get(session_id)
            .cloned()
            .ok_or_else(|| CoreError::NotFound(session_id.to_string()))
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionSnapshot>, CoreError> {
        Ok(self
            .sessions
            .lock()
            .map_err(|_| CoreError::Journal("Session state lock poisoned".into()))?
            .values()
            .cloned()
            .collect())
    }

    /// Replays one Session and completes any planned child-Thread saga before exposing it.
    pub fn recover_session(&self, session_id: &SessionId) -> Result<SessionSnapshot, CoreError> {
        let _lease = self.acquire_writer_lease(session_id)?;
        let mut snapshot = self.load_snapshot(session_id)?;
        let pending = snapshot
            .threads
            .iter()
            .filter(|thread| thread.membership.status == SessionThreadStatus::Creating)
            .map(|thread| thread.membership.thread_id.clone())
            .collect::<Vec<_>>();
        for thread_id in pending {
            self.finish_thread_creation(&mut snapshot, &thread_id)?;
        }
        self.sessions
            .lock()
            .map_err(|_| CoreError::Journal("Session state lock poisoned".into()))?
            .insert(session_id.clone(), snapshot.clone());
        Ok(snapshot)
    }

    pub fn session_events_after(
        &self,
        session_id: &SessionId,
        sequence: u64,
    ) -> Result<Vec<StoredSessionEvent>, CoreError> {
        Ok(self
            .store
            .load(session_id)?
            .into_iter()
            .filter(|event| event.sequence > sequence)
            .collect())
    }

    pub fn session_updates_after(
        &self,
        session_id: &SessionId,
        sequence: u64,
    ) -> Result<Vec<SessionUpdateEnvelope>, CoreError> {
        Ok(self
            .session_events_after(session_id, sequence)?
            .into_iter()
            .map(|event| SessionUpdateEnvelope {
                session_id: session_id.clone(),
                durable_sequence: event.sequence,
                update: SessionUpdate::Committed { event: event.event },
            })
            .collect())
    }

    fn plan_and_finish_thread(
        &self,
        command_id: CommandId,
        session_id: SessionId,
        expectation: SequenceExpectation,
        command: SessionCommand,
        origin: ThreadOrigin,
        title: String,
    ) -> Result<SessionThreadResult, CoreError> {
        let _lease = self.acquire_writer_lease(&session_id)?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| CoreError::Journal("Session state lock poisoned".into()))?;
        let snapshot = sessions
            .get_mut(&session_id)
            .ok_or_else(|| CoreError::NotFound(session_id.to_string()))?;
        if let Some(existing) = snapshot
            .commands
            .iter()
            .find(|existing| existing.receipt.command_id == command_id)
            .cloned()
        {
            if existing.receipt.command != command {
                return Err(CoreError::CommandConflict);
            }
            let SessionCommandResult::ThreadCreated { thread_id } = existing.result else {
                return Err(CoreError::Journal(
                    "create-Thread command has an invalid result".into(),
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
        validate_expectation(expectation, snapshot.sequence)?;

        let thread_id = ThreadId::new(self.next_identifier("thread"))
            .expect("generated Thread ID is non-empty");
        let event = SessionEvent::ThreadCreationPlanned {
            session_id: session_id.clone(),
            thread: SessionThread {
                thread_id: thread_id.clone(),
                origin,
                status: SessionThreadStatus::Creating,
            },
            title,
        };
        let (planned, batch) = self.project_batch(
            Some(snapshot.clone()),
            &session_id,
            vec![event],
            SessionBatchCommand::FirstEvent(SessionCommandReceipt {
                command_id,
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

    fn finish_thread_creation(
        &self,
        snapshot: &mut SessionSnapshot,
        thread_id: &ThreadId,
    ) -> Result<(), CoreError> {
        let planned = snapshot
            .threads
            .iter()
            .find(|thread| thread.membership.thread_id == *thread_id)
            .ok_or_else(|| CoreError::NotFound(thread_id.to_string()))?;
        if planned.membership.status == SessionThreadStatus::Active {
            return Ok(());
        }
        if planned.membership.status != SessionThreadStatus::Creating {
            return Err(CoreError::Journal(
                "only a creating Thread saga can be finished".into(),
            ));
        }
        let title = planned.title.clone();
        let origin = planned.membership.origin.clone();
        let agent_context_seed = planned.agent_context_seed.clone();
        match origin {
            ThreadOrigin::Rewind {
                parent_thread_id,
                before_turn_id,
                ..
            } => self
                .threads
                .create_rewound_thread(crate::CreateRewoundThreadRequest {
                    session_id: snapshot.session_id.clone(),
                    thread_id: thread_id.clone(),
                    title,
                    source_thread_id: parent_thread_id,
                    before_turn_id,
                })?,
            ThreadOrigin::AgentSpawn { .. } => {
                self.threads.create_agent_thread(CreateAgentThreadRequest {
                    session_id: snapshot.session_id.clone(),
                    thread_id: thread_id.clone(),
                    title,
                    context_seed: agent_context_seed.ok_or_else(|| {
                        CoreError::Journal("Agent child Thread is missing its context seed".into())
                    })?,
                })?
            }
            ThreadOrigin::Root | ThreadOrigin::Fork { .. } => {
                if agent_context_seed.is_some() {
                    return Err(CoreError::Journal(
                        "ordinary Thread cannot carry an Agent context seed".into(),
                    ));
                }
                self.threads.create_thread(CreateThreadRequest {
                    session_id: snapshot.session_id.clone(),
                    thread_id: thread_id.clone(),
                    title,
                })?
            }
        };
        let (attached, batch) = self.project_batch(
            Some(snapshot.clone()),
            &snapshot.session_id,
            vec![SessionEvent::ThreadAttached {
                session_id: snapshot.session_id.clone(),
                thread_id: thread_id.clone(),
            }],
            SessionBatchCommand::None,
        )?;
        self.store.append_batch(&batch)?;
        *snapshot = attached;
        Ok(())
    }

    fn apply_lifecycle_command(
        &self,
        request: SessionLifecycleRequest,
        command: SessionCommand,
    ) -> Result<SessionMutationResult, CoreError> {
        validate_command_id(&request.command_id)?;
        let expected_result = match command {
            SessionCommand::Complete => SessionCommandResult::SessionCompleted,
            SessionCommand::Archive => SessionCommandResult::SessionArchived,
            _ => unreachable!("only lifecycle commands are accepted here"),
        };
        self.apply_single_command(
            request.session_id,
            request.expected_sequence,
            SessionCommandReceipt {
                command_id: request.command_id,
                command: command.clone(),
            },
            |session_id| match command {
                SessionCommand::Complete => SessionEvent::SessionCompleted { session_id },
                SessionCommand::Archive => SessionEvent::SessionArchived { session_id },
                _ => unreachable!("only lifecycle commands are accepted here"),
            },
            expected_result,
        )
    }

    fn apply_single_command(
        &self,
        session_id: SessionId,
        expectation: SequenceExpectation,
        receipt: SessionCommandReceipt,
        event: impl FnOnce(SessionId) -> SessionEvent,
        expected_result: SessionCommandResult,
    ) -> Result<SessionMutationResult, CoreError> {
        let _lease = self.acquire_writer_lease(&session_id)?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| CoreError::Journal("Session state lock poisoned".into()))?;
        let snapshot = sessions
            .get_mut(&session_id)
            .ok_or_else(|| CoreError::NotFound(session_id.to_string()))?;
        if let Some(existing) = snapshot
            .commands
            .iter()
            .find(|existing| existing.receipt.command_id == receipt.command_id)
        {
            if existing.receipt != receipt || existing.result != expected_result {
                return Err(CoreError::CommandConflict);
            }
            return Ok(SessionMutationResult {
                sequence: existing.response_sequence,
                disposition: CommandDisposition::Replayed,
            });
        }
        validate_expectation(expectation, snapshot.sequence)?;
        let (next, batch) = self.project_batch(
            Some(snapshot.clone()),
            &session_id,
            vec![event(session_id.clone())],
            SessionBatchCommand::FirstEvent(receipt),
        )?;
        self.store.append_batch(&batch)?;
        *snapshot = next;
        Ok(SessionMutationResult {
            sequence: snapshot.sequence,
            disposition: CommandDisposition::Committed,
        })
    }

    fn load_snapshot(&self, session_id: &SessionId) -> Result<SessionSnapshot, CoreError> {
        let events = self.store.load(session_id)?;
        if events.is_empty() {
            return Err(CoreError::NotFound(session_id.to_string()));
        }
        events
            .iter()
            .try_fold(None, |snapshot, event| {
                reduce_session_event(snapshot, event).map(Some)
            })?
            .ok_or_else(|| CoreError::Journal("cannot recover an empty Session rollout".into()))
    }

    fn validate_active_thread(
        &self,
        sessions: &BTreeMap<SessionId, SessionSnapshot>,
        session_id: &SessionId,
        thread_id: &ThreadId,
    ) -> Result<(), CoreError> {
        let session = sessions
            .get(session_id)
            .ok_or_else(|| CoreError::NotFound(session_id.to_string()))?;
        if session.status != SessionStatus::Active {
            return Err(CoreError::InvalidInput("Session is not active".into()));
        }
        let membership = session
            .threads
            .iter()
            .find(|thread| thread.membership.thread_id == *thread_id)
            .ok_or_else(|| CoreError::NotFound(thread_id.to_string()))?;
        if membership.membership.status != SessionThreadStatus::Active {
            return Err(CoreError::InvalidInput(
                "Thread is not active in its Session".into(),
            ));
        }
        let thread = self.threads.read_thread(thread_id)?;
        if thread.session_id != *session_id {
            return Err(CoreError::InvalidInput(
                "Thread does not belong to Session".into(),
            ));
        }
        Ok(())
    }

    fn interrupt_session_threads(
        &self,
        thread_ids: &[ThreadId],
        session_command_id: &CommandId,
    ) -> Result<(), CoreError> {
        for thread_id in thread_ids {
            loop {
                let snapshot = self.threads.read_thread(thread_id)?;
                let turn_ids = snapshot
                    .turns
                    .iter()
                    .filter(|turn| is_interruptible_turn(turn.status))
                    .map(|turn| turn.turn_id.clone())
                    .collect::<Vec<_>>();
                if turn_ids.is_empty() {
                    break;
                }
                for turn_id in turn_ids {
                    let command_id = CommandId::new(format!(
                        "session-stop-{}-{}-{}",
                        session_command_id, thread_id, turn_id
                    ))
                    .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
                    if let Err(error) = self.threads.interrupt_turn(
                        thread_id,
                        crate::InterruptTurnRequest {
                            command_id,
                            expected_sequence: SequenceExpectation::Any,
                            turn_id: turn_id.clone(),
                        },
                    ) {
                        let current = self.threads.read_thread(thread_id)?;
                        if current
                            .turns
                            .iter()
                            .find(|turn| turn.turn_id == turn_id)
                            .is_some_and(|turn| is_interruptible_turn(turn.status))
                        {
                            return Err(error);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn project_batch(
        &self,
        snapshot: Option<SessionSnapshot>,
        session_id: &SessionId,
        events: Vec<SessionEvent>,
        command: SessionBatchCommand,
    ) -> Result<(SessionSnapshot, SessionEventBatch), CoreError> {
        if events.is_empty() {
            return Err(CoreError::SessionStore(SessionStoreError::InvalidBatch(
                "batch must contain at least one event".into(),
            )));
        }
        let expected_sequence = snapshot.as_ref().map_or(0, |snapshot| snapshot.sequence);
        let mut projection = snapshot;
        let mut envelopes = Vec::with_capacity(events.len());
        for (index, event) in events.into_iter().enumerate() {
            let envelope = StoredSessionEvent {
                schema_version: CURRENT_SESSION_EVENT_SCHEMA_VERSION,
                event_id: SessionEventId(self.next_identifier("session_event")),
                sequence: expected_sequence + index as u64 + 1,
                session_id: session_id.clone(),
                recorded_at: self.timestamp()?,
                command: match (&command, index) {
                    (SessionBatchCommand::FirstEvent(command), 0) => Some(command.clone()),
                    _ => None,
                },
                event,
            };
            projection = Some(reduce_session_event(projection, &envelope)?);
            envelopes.push(envelope);
        }
        Ok((
            projection.expect("a non-empty Session batch always creates a projection"),
            SessionEventBatch {
                batch_id: self.next_identifier("session_batch"),
                session_id: session_id.clone(),
                expected_sequence,
                events: envelopes,
            },
        ))
    }

    fn timestamp(&self) -> Result<SessionTimestamp, CoreError> {
        Ok(SessionTimestamp(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| CoreError::Journal(error.to_string()))?
                .as_millis(),
        ))
    }

    fn acquire_writer_lease(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<Box<dyn LeaseGuard>>, CoreError> {
        self.writer_lease
            .as_ref()
            .map(|lease| lease.acquire(session_id))
            .transpose()
    }

    fn next_identifier(&self, prefix: &str) -> String {
        let ordinal = self.next_id.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("{prefix}_{timestamp:032x}_{ordinal:016x}")
    }
}

fn validate_command(command_id: &CommandId, title: &str) -> Result<(), CoreError> {
    validate_command_id(command_id)?;
    if title.trim().is_empty() {
        Err(CoreError::InvalidInput("title must not be empty".into()))
    } else {
        Ok(())
    }
}

fn is_interruptible_turn(status: TurnStatus) -> bool {
    matches!(
        status,
        TurnStatus::Created
            | TurnStatus::Running
            | TurnStatus::WaitingForApproval
            | TurnStatus::WaitingForUserInput
            | TurnStatus::WaitingForCapability
            | TurnStatus::Cancelling
    )
}

fn validate_command_id(command_id: &CommandId) -> Result<(), CoreError> {
    if command_id.as_str().trim().is_empty() {
        Err(CoreError::InvalidInput(
            "command ID must be non-empty".into(),
        ))
    } else {
        Ok(())
    }
}

fn validate_expectation(expectation: SequenceExpectation, actual: u64) -> Result<(), CoreError> {
    match expectation {
        SequenceExpectation::Any => Ok(()),
        SequenceExpectation::Exact(expected) if expected == actual => Ok(()),
        SequenceExpectation::Exact(expected) => Err(CoreError::SessionStore(
            SessionStoreError::SequenceConflict { expected, actual },
        )),
    }
}

#[derive(Default)]
pub struct InMemorySessionStore(Mutex<InMemorySessionStoreState>);

#[derive(Default)]
struct InMemorySessionStoreState {
    sessions: BTreeMap<SessionId, Vec<StoredSessionEvent>>,
    batch_ids: BTreeSet<String>,
}

impl SessionStore for InMemorySessionStore {
    fn list_session_ids(&self) -> Result<Vec<SessionId>, SessionStoreError> {
        Ok(self
            .0
            .lock()
            .map_err(|_| SessionStoreError::Storage("in-memory store lock poisoned".into()))?
            .sessions
            .keys()
            .cloned()
            .collect())
    }

    fn load(&self, session_id: &SessionId) -> Result<Vec<StoredSessionEvent>, SessionStoreError> {
        Ok(self
            .0
            .lock()
            .map_err(|_| SessionStoreError::Storage("in-memory store lock poisoned".into()))?
            .sessions
            .get(session_id)
            .cloned()
            .unwrap_or_default())
    }

    fn append_batch(
        &self,
        batch: &SessionEventBatch,
    ) -> Result<AppendSessionBatchResult, SessionStoreError> {
        let mut state = self
            .0
            .lock()
            .map_err(|_| SessionStoreError::Storage("in-memory store lock poisoned".into()))?;
        if state.batch_ids.contains(&batch.batch_id) {
            return Err(SessionStoreError::InvalidBatch(
                "batch ID already exists".into(),
            ));
        }
        let events = state.sessions.entry(batch.session_id.clone()).or_default();
        let actual_sequence = events.last().map_or(0, |event| event.sequence);
        let result = validate_session_append_batch(batch, actual_sequence)?;
        if batch.events.iter().any(|event| {
            events
                .iter()
                .any(|existing| existing.event_id == event.event_id)
        }) {
            return Err(SessionStoreError::InvalidBatch(
                "event ID already exists".into(),
            ));
        }
        events.extend(batch.events.iter().cloned());
        state.batch_ids.insert(batch.batch_id.clone());
        Ok(result)
    }
}

#[cfg(test)]
#[path = "session_coordinator_tests.rs"]
mod tests;
