use crate::CoreError;
use crate::SequenceExpectation;
use crate::ThreadCommandResult;
use crate::ThreadEventBatch;
use crate::ThreadSnapshot;
use crate::ThreadStore;
use crate::WriterLease;
use crate::reduce_thread_event;
use crate::thread_reducer::validate_agent_request;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_protocol::AgentRequest;
use zeta_protocol::AgentResponse;
use zeta_protocol::CommandId;
use zeta_protocol::InteractionCancelReason;
use zeta_protocol::InteractionDeadline;
use zeta_protocol::ItemId;
use zeta_protocol::RequestId;
use zeta_protocol::SessionId;
use zeta_protocol::StableTurnError;
use zeta_protocol::ThreadCommand;
use zeta_protocol::ThreadEvent;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadItem;
use zeta_protocol::ThreadUpdate;
use zeta_protocol::ThreadUpdateEnvelope;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolName;
use zeta_protocol::TurnId;
use zeta_protocol::TurnInteraction;
use zeta_protocol::UserInput;
use zeta_thread_store::validate_append_batch;
use zeta_thread_store::{
    AppendBatchResult, CURRENT_STORED_EVENT_SCHEMA_VERSION, EventId, StoredEvent,
    ThreadCommandReceipt, ThreadStoreError, Timestamp,
};

mod execution;
mod mailbox;
mod user_input;

pub struct StartTurnRequest {
    pub command_id: CommandId,
    pub expected_sequence: SequenceExpectation,
    pub input: Vec<UserInput>,
}

pub struct CreateThreadRequest {
    pub session_id: SessionId,
    pub thread_id: ThreadId,
    pub title: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartTurnDisposition {
    Created,
    Replayed,
}

pub struct StartTurnResult {
    pub turn_id: TurnId,
    pub sequence: u64,
    pub disposition: StartTurnDisposition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterruptTurnDisposition {
    Interrupted,
    Replayed,
}

pub struct InterruptTurnRequest {
    pub command_id: CommandId,
    pub expected_sequence: SequenceExpectation,
    pub turn_id: TurnId,
}

pub struct InterruptTurnResult {
    pub sequence: u64,
    pub disposition: InterruptTurnDisposition,
}

/// Execution request to place a running Turn in a durable interaction wait state.
pub struct RequestTurnInteraction {
    pub request_id: RequestId,
    pub item_id: Option<ItemId>,
    pub request: AgentRequest,
    pub deadline: Option<InteractionDeadline>,
}

pub struct RequestedTurnInteraction {
    pub interaction: TurnInteraction,
    pub sequence: u64,
}

/// Client command to resolve exactly one outstanding Turn interaction.
pub struct ResolveTurnInteractionRequest {
    pub command_id: CommandId,
    pub expected_sequence: SequenceExpectation,
    pub turn_id: TurnId,
    pub request_id: RequestId,
    pub response: AgentResponse,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolveTurnInteractionDisposition {
    Resolved,
    Replayed,
}

pub struct ResolveTurnInteractionResult {
    pub sequence: u64,
    pub disposition: ResolveTurnInteractionDisposition,
}

/// Execution action that closes an outstanding interaction without accepting a client response.
pub struct CancelTurnInteractionRequest {
    pub turn_id: TurnId,
    pub request_id: RequestId,
    pub reason: InteractionCancelReason,
}

pub struct CancelledTurnInteraction {
    pub sequence: u64,
}

pub struct CompletedTurn {
    pub item: ThreadItem,
    pub sequence: u64,
}

pub struct RecordToolCallRequest {
    pub name: ToolName,
    pub arguments_json: String,
}

pub struct RecordedToolCall {
    pub item: ThreadItem,
    pub tool_call_id: ToolCallId,
    pub sequence: u64,
}

pub enum ToolCallOutput {
    Success(String),
    Failure(String),
}

pub struct RecordToolResultRequest {
    pub tool_call_id: ToolCallId,
    pub output: ToolCallOutput,
}

pub struct RecordedToolResult {
    pub item: ThreadItem,
    pub sequence: u64,
}

pub(crate) struct RecordToolExecutionStart {
    pub tool_call_id: ToolCallId,
    pub action_digest: String,
    pub policy_revision: String,
    pub authority: zeta_protocol::ToolExecutionAuthority,
}

pub(crate) struct RecordToolExecutionEscalation {
    pub tool_call_id: ToolCallId,
    pub action_digest: String,
    pub policy_revision: String,
    pub denial: zeta_protocol::SandboxDenialOutput,
    pub authority: zeta_protocol::ToolExecutionAuthority,
}

enum BatchCommand {
    None,
    AtEvent {
        index: usize,
        receipt: ThreadCommandReceipt,
    },
}

/// Coordinates durable mutations for each loaded Thread.
pub struct ThreadController {
    store: Arc<dyn ThreadStore>,
    writer_lease: Option<Arc<dyn WriterLease<ThreadId>>>,
    threads: Mutex<BTreeMap<ThreadId, ThreadSnapshot>>,
    execution_mailboxes: mailbox::ThreadExecutionMailboxes,
    next_id: AtomicU64,
}

impl ThreadController {
    pub fn with_store(store: Arc<dyn ThreadStore>) -> Self {
        Self {
            store,
            writer_lease: None,
            threads: Mutex::new(BTreeMap::new()),
            execution_mailboxes: mailbox::ThreadExecutionMailboxes::default(),
            next_id: AtomicU64::new(1),
        }
    }

    /// Builds a manager that acquires the configured writer lease before each durable
    /// Thread mutation. Composition roots use this constructor with the storage adapter.
    pub fn with_store_and_lease(
        store: Arc<dyn ThreadStore>,
        writer_lease: Arc<dyn WriterLease<ThreadId>>,
    ) -> Self {
        Self {
            store,
            writer_lease: Some(writer_lease),
            threads: Mutex::new(BTreeMap::new()),
            execution_mailboxes: mailbox::ThreadExecutionMailboxes::default(),
            next_id: AtomicU64::new(1),
        }
    }

    /// Creates the child Thread already planned by its owning Session.
    ///
    /// Repeating the same request is safe when the durable Thread identity, owner, and title all
    /// match. A conflicting existing stream is rejected.
    pub fn create_thread(&self, request: CreateThreadRequest) -> Result<ThreadSnapshot, CoreError> {
        let _lease = self.acquire_writer_lease(&request.thread_id)?;
        let mut threads = self
            .threads
            .lock()
            .map_err(|_| CoreError::Journal("thread state lock poisoned".into()))?;
        if let Some(existing) = threads.get(&request.thread_id) {
            return matching_created_thread(existing, &request);
        }
        let durable = self.store.load(&request.thread_id)?;
        if !durable.is_empty() {
            let existing = self.load_snapshot(&request.thread_id)?;
            let existing = matching_created_thread(&existing, &request)?;
            threads.insert(request.thread_id, existing.clone());
            return Ok(existing);
        }
        let (snapshot, batch) = self.project_batch(
            None,
            &request.thread_id,
            vec![ThreadEvent::ThreadCreated {
                session_id: request.session_id,
                thread_id: request.thread_id.clone(),
                title: request.title,
            }],
            BatchCommand::None,
        )?;
        self.commit_batch(&batch)?;
        threads.insert(request.thread_id, snapshot.clone());
        Ok(snapshot)
    }

    pub fn start_turn(
        &self,
        thread_id: &ThreadId,
        request: StartTurnRequest,
    ) -> Result<StartTurnResult, CoreError> {
        let validated_input = user_input::validate(&request.input)?;
        validate_command_id(&request.command_id)?;
        let command = ThreadCommand::StartTurn {
            input: request.input.clone(),
        };
        let _lease = self.acquire_writer_lease(thread_id)?;
        let mut threads = self
            .threads
            .lock()
            .map_err(|_| CoreError::Journal("thread state lock poisoned".into()))?;
        let snapshot = threads
            .get_mut(thread_id)
            .ok_or_else(|| CoreError::NotFound(thread_id.to_string()))?;
        if let Some(existing) = snapshot
            .commands
            .iter()
            .find(|existing| existing.receipt.command_id == request.command_id)
        {
            if existing.receipt.command != command {
                return Err(CoreError::CommandConflict);
            }
            let ThreadCommandResult::TurnAccepted { turn_id } = &existing.result else {
                return Err(CoreError::Journal(
                    "start-Turn command has an invalid result".into(),
                ));
            };
            return Ok(StartTurnResult {
                turn_id: turn_id.clone(),
                sequence: existing.response_sequence,
                disposition: StartTurnDisposition::Replayed,
            });
        }
        validate_thread_expectation(request.expected_sequence, snapshot.sequence)?;
        let turn_id =
            TurnId::new(self.next_identifier("turn")).expect("generated Turn ID is non-empty");
        let input_items = user_input::thread_items(&validated_input, &turn_id, || {
            ItemId::new(self.next_identifier("item")).expect("generated Item ID is non-empty")
        });
        let mut events = Vec::with_capacity(input_items.len() + 2);
        events.push(ThreadEvent::TurnAccepted {
            thread_id: thread_id.clone(),
            turn_id: turn_id.clone(),
        });
        events.extend(
            input_items
                .into_iter()
                .map(|item| ThreadEvent::ItemCompleted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    item,
                }),
        );
        events.push(ThreadEvent::TurnStarted {
            thread_id: thread_id.clone(),
            turn_id: turn_id.clone(),
        });
        let (next_snapshot, batch) = self.project_batch(
            Some(snapshot.clone()),
            &snapshot.thread_id,
            events,
            BatchCommand::AtEvent {
                index: 0,
                receipt: ThreadCommandReceipt {
                    command_id: request.command_id,
                    command,
                },
            },
        )?;
        self.commit_batch(&batch)?;
        *snapshot = next_snapshot;
        Ok(StartTurnResult {
            turn_id,
            sequence: snapshot.sequence,
            disposition: StartTurnDisposition::Created,
        })
    }

    pub fn complete_turn(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        output: String,
    ) -> Result<CompletedTurn, CoreError> {
        self.complete_turn_with_agent_message(
            thread_id,
            turn_id,
            ItemId::new(self.next_identifier("item")).expect("generated Item ID is non-empty"),
            output,
        )
    }

    /// Persists a request that pauses the running Turn until a matching response or cancellation.
    ///
    /// This is an execution action rather than a user command: the Agent loop creates the request,
    /// while a client response is accepted through `resolve_turn_interaction`.
    pub fn request_turn_interaction(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        request: RequestTurnInteraction,
    ) -> Result<RequestedTurnInteraction, CoreError> {
        validate_request_id(&request.request_id)?;
        validate_agent_request(&request.request).map_err(CoreError::InvalidInput)?;
        let interaction = TurnInteraction {
            request_id: request.request_id,
            item_id: request.item_id,
            request: request.request,
            deadline: request.deadline,
        };
        let _lease = self.acquire_writer_lease(thread_id)?;
        let mut threads = self
            .threads
            .lock()
            .map_err(|_| CoreError::Journal("thread state lock poisoned".into()))?;
        let snapshot = threads
            .get_mut(thread_id)
            .ok_or_else(|| CoreError::NotFound(thread_id.to_string()))?;
        self.record_batch(
            snapshot,
            vec![ThreadEvent::InteractionRequested {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                interaction: interaction.clone(),
            }],
        )?;
        Ok(RequestedTurnInteraction {
            interaction,
            sequence: snapshot.sequence,
        })
    }

    /// Resolves a durable interaction through a retry-safe client command.
    pub fn resolve_turn_interaction(
        &self,
        thread_id: &ThreadId,
        request: ResolveTurnInteractionRequest,
    ) -> Result<ResolveTurnInteractionResult, CoreError> {
        validate_command_id(&request.command_id)?;
        validate_request_id(&request.request_id)?;
        let command = resolution_command(&request);
        let _lease = self.acquire_writer_lease(thread_id)?;
        let mut threads = self
            .threads
            .lock()
            .map_err(|_| CoreError::Journal("thread state lock poisoned".into()))?;
        let snapshot = threads
            .get_mut(thread_id)
            .ok_or_else(|| CoreError::NotFound(thread_id.to_string()))?;
        if let Some(existing) = snapshot
            .commands
            .iter()
            .find(|existing| existing.receipt.command_id == request.command_id)
        {
            if existing.receipt.command != command {
                return Err(CoreError::CommandConflict);
            }
            if !matches!(
                &existing.result,
                ThreadCommandResult::InteractionResolved { turn_id, request_id }
                    if turn_id == &request.turn_id && request_id == &request.request_id
            ) {
                return Err(CoreError::Journal(
                    "interaction resolution command has an invalid result".into(),
                ));
            }
            return Ok(ResolveTurnInteractionResult {
                sequence: existing.response_sequence,
                disposition: ResolveTurnInteractionDisposition::Replayed,
            });
        }
        validate_thread_expectation(request.expected_sequence, snapshot.sequence)?;
        let (next_snapshot, batch) = self.project_batch(
            Some(snapshot.clone()),
            thread_id,
            vec![ThreadEvent::InteractionResolved {
                thread_id: thread_id.clone(),
                turn_id: request.turn_id.clone(),
                request_id: request.request_id.clone(),
                response: request.response,
            }],
            BatchCommand::AtEvent {
                index: 0,
                receipt: ThreadCommandReceipt {
                    command_id: request.command_id,
                    command,
                },
            },
        )?;
        self.commit_batch(&batch)?;
        *snapshot = next_snapshot;
        Ok(ResolveTurnInteractionResult {
            sequence: snapshot.sequence,
            disposition: ResolveTurnInteractionDisposition::Resolved,
        })
    }

    /// Closes an outstanding interaction when its execution policy cannot accept a response.
    ///
    /// The caller decides the next Turn outcome after this durable fact; this method only makes
    /// the wait state explicit and returns the Turn to `Running` for that decision.
    pub fn cancel_turn_interaction(
        &self,
        thread_id: &ThreadId,
        request: CancelTurnInteractionRequest,
    ) -> Result<CancelledTurnInteraction, CoreError> {
        validate_request_id(&request.request_id)?;
        let _lease = self.acquire_writer_lease(thread_id)?;
        let mut threads = self
            .threads
            .lock()
            .map_err(|_| CoreError::Journal("thread state lock poisoned".into()))?;
        let snapshot = threads
            .get_mut(thread_id)
            .ok_or_else(|| CoreError::NotFound(thread_id.to_string()))?;
        self.record_batch(
            snapshot,
            vec![ThreadEvent::InteractionCancelled {
                thread_id: thread_id.clone(),
                turn_id: request.turn_id,
                request_id: request.request_id,
                reason: request.reason,
            }],
        )?;
        Ok(CancelledTurnInteraction {
            sequence: snapshot.sequence,
        })
    }

    pub fn record_tool_call(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        request: RecordToolCallRequest,
    ) -> Result<RecordedToolCall, CoreError> {
        let tool_call_id = ToolCallId::new(self.next_identifier("tool"))
            .expect("generated tool call ID is non-empty");
        let item = ThreadItem::ToolCall {
            item_id: ItemId::new(self.next_identifier("item"))
                .expect("generated Item ID is non-empty"),
            turn_id: turn_id.clone(),
            tool_call_id: tool_call_id.clone(),
            name: request.name,
            arguments_json: request.arguments_json,
        };
        let sequence = self.record_item(thread_id, turn_id, item.clone())?;
        Ok(RecordedToolCall {
            item,
            tool_call_id,
            sequence,
        })
    }

    pub fn record_tool_result(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        request: RecordToolResultRequest,
    ) -> Result<RecordedToolResult, CoreError> {
        let (text, is_error) = match request.output {
            ToolCallOutput::Success(text) => (text, false),
            ToolCallOutput::Failure(text) => (text, true),
        };
        let item = ThreadItem::ToolResult {
            item_id: ItemId::new(self.next_identifier("item"))
                .expect("generated Item ID is non-empty"),
            turn_id: turn_id.clone(),
            tool_call_id: request.tool_call_id,
            text,
            is_error,
        };
        let sequence = self.record_item(thread_id, turn_id, item.clone())?;
        Ok(RecordedToolResult { item, sequence })
    }

    pub fn fail_turn(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        error: StableTurnError,
    ) -> Result<(), CoreError> {
        self.transition_turn(
            thread_id,
            vec![ThreadEvent::TurnFailed {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                error,
            }],
        )
    }

    pub fn interrupt_turn(
        &self,
        thread_id: &ThreadId,
        request: InterruptTurnRequest,
    ) -> Result<InterruptTurnResult, CoreError> {
        validate_command_id(&request.command_id)?;
        let turn_id_for_cancellation = request.turn_id.clone();
        let command = ThreadCommand::InterruptTurn {
            turn_id: request.turn_id.clone(),
        };
        let _lease = self.acquire_writer_lease(thread_id)?;
        let mut threads = self
            .threads
            .lock()
            .map_err(|_| CoreError::Journal("thread state lock poisoned".into()))?;
        let snapshot = threads
            .get_mut(thread_id)
            .ok_or_else(|| CoreError::NotFound(thread_id.to_string()))?;
        if let Some(existing) = snapshot
            .commands
            .iter()
            .find(|existing| existing.receipt.command_id == request.command_id)
        {
            if existing.receipt.command != command {
                return Err(CoreError::CommandConflict);
            }
            if !matches!(
                &existing.result,
                ThreadCommandResult::TurnInterrupted { turn_id }
                    if turn_id == &request.turn_id
            ) {
                return Err(CoreError::Journal(
                    "interrupt-Turn command has an invalid result".into(),
                ));
            }
            return Ok(InterruptTurnResult {
                sequence: existing.response_sequence,
                disposition: InterruptTurnDisposition::Replayed,
            });
        }
        validate_thread_expectation(request.expected_sequence, snapshot.sequence)?;
        let pending_interaction = snapshot
            .turns
            .iter()
            .find(|turn| turn.turn_id == request.turn_id)
            .ok_or_else(|| CoreError::NotFound(request.turn_id.to_string()))?
            .pending_interaction
            .clone();
        let command_event_index = usize::from(pending_interaction.is_some());
        let mut events = Vec::with_capacity(3);
        if let Some(interaction) = pending_interaction {
            events.push(ThreadEvent::InteractionCancelled {
                thread_id: thread_id.clone(),
                turn_id: request.turn_id.clone(),
                request_id: interaction.request_id,
                reason: InteractionCancelReason::TurnInterrupted,
            });
        }
        events.extend([
            ThreadEvent::TurnCancelling {
                thread_id: thread_id.clone(),
                turn_id: request.turn_id.clone(),
            },
            ThreadEvent::TurnInterrupted {
                thread_id: thread_id.clone(),
                turn_id: request.turn_id,
            },
        ]);
        let (next_snapshot, batch) = self.project_batch(
            Some(snapshot.clone()),
            thread_id,
            events,
            BatchCommand::AtEvent {
                index: command_event_index,
                receipt: ThreadCommandReceipt {
                    command_id: request.command_id,
                    command,
                },
            },
        )?;
        self.commit_batch(&batch)?;
        *snapshot = next_snapshot;
        self.cancel_turn_execution(thread_id, &turn_id_for_cancellation);
        Ok(InterruptTurnResult {
            sequence: snapshot.sequence,
            disposition: InterruptTurnDisposition::Interrupted,
        })
    }

    pub fn read_thread(&self, thread_id: &ThreadId) -> Result<ThreadSnapshot, CoreError> {
        self.threads
            .lock()
            .map_err(|_| CoreError::Journal("thread state lock poisoned".into()))?
            .get(thread_id)
            .cloned()
            .ok_or_else(|| CoreError::NotFound(thread_id.to_string()))
    }

    /// Returns the in-memory projections currently loaded by this manager.
    pub fn list_threads(&self) -> Result<Vec<ThreadSnapshot>, CoreError> {
        Ok(self
            .threads
            .lock()
            .map_err(|_| CoreError::Journal("thread state lock poisoned".into()))?
            .values()
            .cloned()
            .collect())
    }

    /// Replays committed updates after a durable Thread sequence for reconnecting consumers.
    pub fn thread_updates_after(
        &self,
        thread_id: &ThreadId,
        sequence: u64,
    ) -> Result<Vec<ThreadUpdateEnvelope>, CoreError> {
        let session_id = self.read_thread(thread_id)?.session_id;
        Ok(self
            .store
            .load(thread_id)?
            .into_iter()
            .filter(|event| event.sequence > sequence)
            .map(|event| ThreadUpdateEnvelope {
                session_id: session_id.clone(),
                thread_id: thread_id.clone(),
                durable_sequence: event.sequence,
                stream_cursor: None,
                update: ThreadUpdate::Committed { event: event.event },
            })
            .collect())
    }

    /// Replays one Thread's durable rollout and interrupts any Turn that was in progress when the
    /// previous process stopped. The recovery markers are appended before the restored projection
    /// becomes observable.
    pub fn recover_thread(&self, thread_id: &ThreadId) -> Result<ThreadSnapshot, CoreError> {
        let _lease = self.acquire_writer_lease(thread_id)?;
        let mut snapshot = self.load_snapshot(thread_id)?;
        let mut recovery_events = Vec::new();
        for turn in snapshot.turns.clone() {
            if !matches!(
                turn.status,
                crate::TurnStatus::Completed
                    | crate::TurnStatus::Failed
                    | crate::TurnStatus::Interrupted
                    | crate::TurnStatus::WaitingForApproval
                    | crate::TurnStatus::WaitingForUserInput
                    | crate::TurnStatus::WaitingForCapability
            ) && !snapshot.has_resumable_tool_continuation(&turn.turn_id)
            {
                if turn.status != crate::TurnStatus::Cancelling {
                    recovery_events.push(ThreadEvent::TurnCancelling {
                        thread_id: thread_id.clone(),
                        turn_id: turn.turn_id.clone(),
                    });
                }
                recovery_events.push(ThreadEvent::TurnInterrupted {
                    thread_id: thread_id.clone(),
                    turn_id: turn.turn_id,
                });
            }
        }
        if !recovery_events.is_empty() {
            self.record_batch(&mut snapshot, recovery_events)?;
        }
        self.threads
            .lock()
            .map_err(|_| CoreError::Journal("thread state lock poisoned".into()))?
            .insert(thread_id.clone(), snapshot.clone());
        Ok(snapshot)
    }

    fn load_snapshot(&self, thread_id: &ThreadId) -> Result<ThreadSnapshot, CoreError> {
        let events = self.store.load(thread_id)?;
        if events.is_empty() {
            return Err(CoreError::NotFound(thread_id.to_string()));
        }
        events
            .iter()
            .try_fold(None, |snapshot, event| {
                reduce_thread_event(snapshot, event).map(Some)
            })?
            .ok_or_else(|| CoreError::Journal("cannot recover an empty rollout".into()))
    }

    fn transition_turn(
        &self,
        thread_id: &ThreadId,
        events: Vec<ThreadEvent>,
    ) -> Result<(), CoreError> {
        let _lease = self.acquire_writer_lease(thread_id)?;
        let mut threads = self
            .threads
            .lock()
            .map_err(|_| CoreError::Journal("thread state lock poisoned".into()))?;
        let snapshot = threads
            .get_mut(thread_id)
            .ok_or_else(|| CoreError::NotFound(thread_id.to_string()))?;
        self.record_batch(snapshot, events)
    }

    fn record_item(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        item: ThreadItem,
    ) -> Result<u64, CoreError> {
        let _lease = self.acquire_writer_lease(thread_id)?;
        let mut threads = self
            .threads
            .lock()
            .map_err(|_| CoreError::Journal("thread state lock poisoned".into()))?;
        let snapshot = threads
            .get_mut(thread_id)
            .ok_or_else(|| CoreError::NotFound(thread_id.to_string()))?;
        self.record_batch(
            snapshot,
            vec![ThreadEvent::ItemCompleted {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                item,
            }],
        )?;
        Ok(snapshot.sequence)
    }

    fn record_batch(
        &self,
        snapshot: &mut ThreadSnapshot,
        events: Vec<ThreadEvent>,
    ) -> Result<(), CoreError> {
        let (next_snapshot, batch) = self.project_batch(
            Some(snapshot.clone()),
            &snapshot.thread_id,
            events,
            BatchCommand::None,
        )?;
        self.commit_batch(&batch)?;
        *snapshot = next_snapshot;
        Ok(())
    }

    fn project_batch(
        &self,
        snapshot: Option<ThreadSnapshot>,
        thread_id: &ThreadId,
        events: Vec<ThreadEvent>,
        command: BatchCommand,
    ) -> Result<(ThreadSnapshot, ThreadEventBatch), CoreError> {
        if events.is_empty() {
            return Err(CoreError::ThreadStore(ThreadStoreError::InvalidBatch(
                "batch must contain at least one event".into(),
            )));
        }
        let expected_sequence = snapshot.as_ref().map_or(0, |snapshot| snapshot.sequence);
        let mut projection = snapshot;
        let mut envelopes = Vec::with_capacity(events.len());
        for (index, event) in events.into_iter().enumerate() {
            let envelope = StoredEvent {
                schema_version: CURRENT_STORED_EVENT_SCHEMA_VERSION,
                event_id: EventId(self.next_identifier("event")),
                sequence: expected_sequence + index as u64 + 1,
                thread_id: thread_id.clone(),
                recorded_at: self.timestamp()?,
                command: match (&command, index) {
                    (BatchCommand::AtEvent { index, receipt }, event_index)
                        if index == &event_index =>
                    {
                        Some(receipt.clone())
                    }
                    _ => None,
                },
                event,
            };
            projection = Some(reduce_thread_event(projection, &envelope)?);
            envelopes.push(envelope);
        }
        Ok((
            projection.expect("a non-empty event batch always creates a projection"),
            ThreadEventBatch {
                batch_id: self.next_identifier("batch"),
                thread_id: thread_id.clone(),
                expected_sequence,
                events: envelopes,
            },
        ))
    }

    fn commit_batch(&self, batch: &ThreadEventBatch) -> Result<AppendBatchResult, CoreError> {
        self.store.append_batch(batch).map_err(CoreError::from)
    }

    fn timestamp(&self) -> Result<Timestamp, CoreError> {
        Ok(Timestamp(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| CoreError::Journal(error.to_string()))?
                .as_millis(),
        ))
    }

    fn acquire_writer_lease(
        &self,
        thread_id: &ThreadId,
    ) -> Result<Option<Box<dyn crate::LeaseGuard>>, CoreError> {
        self.writer_lease
            .as_ref()
            .map(|lease| lease.acquire(thread_id))
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

fn validate_command_id(command_id: &CommandId) -> Result<(), CoreError> {
    if command_id.as_str().trim().is_empty() {
        Err(CoreError::InvalidInput(
            "command ID must be non-empty".into(),
        ))
    } else {
        Ok(())
    }
}

fn validate_request_id(request_id: &RequestId) -> Result<(), CoreError> {
    if request_id.as_str().trim().is_empty() {
        Err(CoreError::InvalidInput(
            "request ID must be non-empty".into(),
        ))
    } else {
        Ok(())
    }
}

fn resolution_command(request: &ResolveTurnInteractionRequest) -> ThreadCommand {
    match &request.response {
        AgentResponse::Approval { response } => ThreadCommand::ResolveApproval {
            turn_id: request.turn_id.clone(),
            request_id: request.request_id.clone(),
            response: response.clone(),
        },
        AgentResponse::UserInput { response } => ThreadCommand::ResolveUserInput {
            turn_id: request.turn_id.clone(),
            request_id: request.request_id.clone(),
            response: response.clone(),
        },
        AgentResponse::DynamicTool { response } => ThreadCommand::ResolveDynamicTool {
            turn_id: request.turn_id.clone(),
            request_id: request.request_id.clone(),
            response: response.clone(),
        },
    }
}

fn validate_thread_expectation(
    expectation: SequenceExpectation,
    actual: u64,
) -> Result<(), CoreError> {
    match expectation {
        SequenceExpectation::Any => Ok(()),
        SequenceExpectation::Exact(expected) if expected == actual => Ok(()),
        SequenceExpectation::Exact(expected) => {
            Err(CoreError::ThreadStore(ThreadStoreError::SequenceConflict {
                expected,
                actual,
            }))
        }
    }
}

fn matching_created_thread(
    snapshot: &ThreadSnapshot,
    request: &CreateThreadRequest,
) -> Result<ThreadSnapshot, CoreError> {
    if snapshot.session_id == request.session_id
        && snapshot.thread_id == request.thread_id
        && snapshot.title == request.title
    {
        Ok(snapshot.clone())
    } else {
        Err(CoreError::CommandConflict)
    }
}

#[derive(Default)]
pub struct InMemoryThreadStore(Mutex<InMemoryThreadStoreState>);

#[derive(Default)]
struct InMemoryThreadStoreState {
    threads: BTreeMap<ThreadId, Vec<StoredEvent>>,
    batch_ids: BTreeSet<String>,
}

impl InMemoryThreadStore {
    pub fn events(&self) -> Vec<StoredEvent> {
        self.0
            .lock()
            .expect("in-memory Thread store lock should not be poisoned")
            .threads
            .values()
            .flatten()
            .cloned()
            .collect()
    }
}

impl ThreadStore for InMemoryThreadStore {
    fn list_thread_ids(&self) -> Result<Vec<ThreadId>, ThreadStoreError> {
        Ok(self
            .0
            .lock()
            .map_err(|_| ThreadStoreError::Storage("in-memory store lock poisoned".into()))?
            .threads
            .keys()
            .cloned()
            .collect())
    }

    fn load(&self, thread_id: &ThreadId) -> Result<Vec<StoredEvent>, ThreadStoreError> {
        Ok(self
            .0
            .lock()
            .map_err(|_| ThreadStoreError::Storage("in-memory store lock poisoned".into()))?
            .threads
            .get(thread_id)
            .cloned()
            .unwrap_or_default())
    }

    fn append_batch(
        &self,
        batch: &ThreadEventBatch,
    ) -> Result<AppendBatchResult, ThreadStoreError> {
        let mut state = self
            .0
            .lock()
            .map_err(|_| ThreadStoreError::Storage("in-memory store lock poisoned".into()))?;
        if state.batch_ids.contains(&batch.batch_id) {
            return Err(ThreadStoreError::InvalidBatch(
                "batch ID already exists".into(),
            ));
        }
        let events = state.threads.entry(batch.thread_id.clone()).or_default();
        let actual_sequence = events.last().map_or(0, |event| event.sequence);
        let result = validate_append_batch(batch, actual_sequence)?;
        if batch.events.iter().any(|event| {
            events
                .iter()
                .any(|existing| existing.event_id == event.event_id)
        }) {
            return Err(ThreadStoreError::InvalidBatch(
                "event ID already exists".into(),
            ));
        }
        events.extend(batch.events.iter().cloned());
        state.batch_ids.insert(batch.batch_id.clone());
        Ok(result)
    }
}
