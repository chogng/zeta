use crate::ContextBudget;
use crate::CoreError;
use crate::HarnessInstructions;
use crate::SequenceExpectation;
use crate::ThreadCommandResult;
use crate::ThreadEventBatch;
use crate::ThreadSnapshot;
use crate::ThreadStore;
use crate::WriterLease;
use crate::context::ContextInput;
use crate::context::ContextPreparation;
use crate::context::FrozenModelSelection;
use crate::context::ModelInvocationPreparation;
use crate::context::ModelInvocationSnapshot;
use crate::reduce_thread_event;
use crate::thread_reducer::validate_agent_request;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::RwLock;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_history::CURRENT_STORED_EVENT_SCHEMA_VERSION;
use zeta_history::EventId;
use zeta_history::StoredEvent;
use zeta_history::ThreadCommandReceipt;
use zeta_history::Timestamp;
use zeta_protocol::AgentRequest;
use zeta_protocol::AgentResponse;
use zeta_protocol::ApprovalMode;
use zeta_protocol::CommandId;
use zeta_protocol::ContentPart;
use zeta_protocol::ContextCheckpoint;
use zeta_protocol::ContextCheckpointId;
use zeta_protocol::ContextCheckpointVerification;
use zeta_protocol::ContextSourceRange;
use zeta_protocol::FrozenSkillActivation;
use zeta_protocol::InteractionCancelReason;
use zeta_protocol::InteractionDeadline;
use zeta_protocol::ItemId;
use zeta_protocol::ModelRef;
use zeta_protocol::RequestId;
use zeta_protocol::SessionId;
use zeta_protocol::SkillActivationReason;
use zeta_protocol::StableTurnError;
use zeta_protocol::ThreadCommand;
use zeta_protocol::ThreadEvent;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadItem;
use zeta_protocol::ThreadUpdate;
use zeta_protocol::ThreadUpdateEnvelope;
use zeta_protocol::ToolCallBinding;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolDefinition;
use zeta_protocol::ToolName;
use zeta_protocol::TurnId;
use zeta_protocol::TurnInteraction;
use zeta_protocol::UserInput;
use zeta_thread_store::AppendBatchResult;
use zeta_thread_store::ThreadStoreError;
use zeta_thread_store::validate_append_batch;
use zeta_attachments::ImageAttachments;

mod agent;
mod execution;
mod loaded_thread;
mod mailbox;
mod user_input;

pub use agent::CreateAgentThreadRequest;

pub struct StartTurnRequest {
    pub command_id: CommandId,
    pub expected_sequence: SequenceExpectation,
    pub model: Option<ModelRef>,
    pub policy_revision: String,
    /// Host-seeded automatic activations. Explicit selections are resolved by extensions.
    pub approval_mode: ApprovalMode,
    pub activated_skills: Vec<FrozenSkillActivation>,
    pub input: Vec<UserInput>,
}

/// Named inputs for preparing one immutable model invocation from durable Thread state.
pub(crate) struct PrepareModelInvocationRequest<'a> {
    pub turn_id: &'a TurnId,
    pub instructions: &'a HarnessInstructions,
    pub extension_fragments: Vec<zeta_extension_api::PromptFragment>,
    pub evidence: Vec<crate::ContextEvidence>,
    pub tools: Vec<ToolDefinition>,
    pub budget: ContextBudget,
}

/// Concrete host invocation used to execute one explicit Shell Turn.
pub struct ShellTurnInvocation {
    pub command: String,
    pub shell_program: String,
    pub working_directory: String,
}

/// Client command that starts a model-free Turn containing one durable shell Tool Call.
pub struct StartShellTurnRequest {
    pub command_id: CommandId,
    pub expected_sequence: SequenceExpectation,
    pub policy_revision: String,
    pub approval_mode: ApprovalMode,
    pub tool_call_id: ToolCallId,
    pub binding: ToolCallBinding,
    pub invocation: ShellTurnInvocation,
}

pub struct CreateThreadRequest {
    pub session_id: SessionId,
    pub thread_id: ThreadId,
    pub title: String,
}

pub struct CreateRewoundThreadRequest {
    pub session_id: SessionId,
    pub thread_id: ThreadId,
    pub title: String,
    pub source_thread_id: ThreadId,
    pub before_turn_id: TurnId,
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
    pub tool_call_id: Option<ToolCallId>,
    pub name: ToolName,
    pub arguments_json: String,
    pub binding: Option<ToolCallBinding>,
}

pub struct RecordedToolCall {
    pub item: ThreadItem,
    pub tool_call_id: ToolCallId,
    pub sequence: u64,
}

pub enum ToolCallOutput {
    Success(String),
    Failure(String),
    SuccessContent(Vec<ContentPart>),
    FailureContent(Vec<ContentPart>),
}

pub struct RecordToolResultRequest {
    pub tool_call_id: ToolCallId,
    pub output: ToolCallOutput,
}

pub struct RecordedToolResult {
    pub item: ThreadItem,
    pub sequence: u64,
}

pub(crate) struct CommitContextCheckpointRequest {
    pub(crate) source_thread_sequence: u64,
    pub(crate) covered: ContextSourceRange,
    pub(crate) summary: String,
    pub(crate) schema_revision: String,
    pub(crate) prompt_revision: String,
    pub(crate) context_policy_revision: String,
    pub(crate) generator_model: Option<ModelRef>,
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
    loaded_threads: Arc<loaded_thread::LoadedThreads>,
    execution_mailboxes: mailbox::ThreadExecutionMailboxes,
    extensions: RwLock<Arc<zeta_extension_api::ExtensionRegistry>>,
    image_attachments: Arc<ImageAttachments>,
    next_id: AtomicU64,
}

impl ThreadController {
    pub fn with_store(store: Arc<dyn ThreadStore>) -> Self {
        Self::with_store_and_image_attachments(store, Arc::new(ImageAttachments::in_memory()))
    }

    /// Builds a manager with the canonical image attachment service used before durable writes.
    pub fn with_store_and_image_attachments(
        store: Arc<dyn ThreadStore>,
        image_attachments: Arc<ImageAttachments>,
    ) -> Self {
        let loaded_threads = Arc::new(loaded_thread::LoadedThreads::new(store.clone()));
        Self {
            store,
            writer_lease: None,
            execution_mailboxes: mailbox::ThreadExecutionMailboxes::new(loaded_threads.clone()),
            extensions: RwLock::new(Arc::new(zeta_extension_api::ExtensionRegistry::default())),
            image_attachments,
            loaded_threads,
            next_id: AtomicU64::new(1),
        }
    }

    /// Builds a manager that acquires the configured writer lease before each durable
    /// Thread mutation. Composition roots use this constructor with the storage adapter.
    pub fn with_store_and_lease(
        store: Arc<dyn ThreadStore>,
        writer_lease: Arc<dyn WriterLease<ThreadId>>,
    ) -> Self {
        Self::with_store_lease_and_image_attachments(
            store,
            writer_lease,
            Arc::new(ImageAttachments::in_memory()),
        )
    }

    /// Builds a leased manager with the attachment service shared by RPC admission and models.
    pub fn with_store_lease_and_image_attachments(
        store: Arc<dyn ThreadStore>,
        writer_lease: Arc<dyn WriterLease<ThreadId>>,
        image_attachments: Arc<ImageAttachments>,
    ) -> Self {
        let loaded_threads = Arc::new(loaded_thread::LoadedThreads::new(store.clone()));
        Self {
            store,
            writer_lease: Some(writer_lease),
            execution_mailboxes: mailbox::ThreadExecutionMailboxes::new(loaded_threads.clone()),
            extensions: RwLock::new(Arc::new(zeta_extension_api::ExtensionRegistry::default())),
            image_attachments,
            loaded_threads,
            next_id: AtomicU64::new(1),
        }
    }

    /// Returns the canonical service used by this Thread authority and its model executors.
    pub fn image_attachments(&self) -> Arc<ImageAttachments> {
        Arc::clone(&self.image_attachments)
    }

    /// Installs the shared agent extension registry before product Turns are accepted.
    pub fn install_extensions(
        &self,
        extensions: Arc<zeta_extension_api::ExtensionRegistry>,
    ) -> Result<(), CoreError> {
        *self
            .extensions
            .write()
            .map_err(|_| CoreError::Journal("extension registry lock poisoned".into()))? =
            extensions;
        Ok(())
    }

    /// Creates the child Thread already planned by its owning Session.
    ///
    /// Repeating the same request is safe when the durable Thread identity, owner, and title all
    /// match. A conflicting existing stream is rejected.
    pub fn create_thread(&self, request: CreateThreadRequest) -> Result<ThreadSnapshot, CoreError> {
        let slot = self.loaded_threads.slot(&request.thread_id)?;
        let _permit = slot.enter_mutation()?;
        let _lease = self.acquire_writer_lease(&request.thread_id)?;
        let mut loaded = slot
            .loaded
            .lock()
            .map_err(|_| CoreError::Journal("loaded Thread state lock poisoned".into()))?;
        if let Some(existing) = loaded.as_ref() {
            return matching_created_thread(&existing.snapshot, &request);
        }
        let durable = self.store.load(&request.thread_id)?;
        if !durable.is_empty() {
            let existing = self.load_snapshot(&request.thread_id)?;
            let existing = matching_created_thread(&existing, &request)?;
            *loaded = Some(self.loaded_threads.install(existing.clone()));
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
        *loaded = Some(self.loaded_threads.install(snapshot.clone()));
        Ok(snapshot)
    }

    /// Creates a child Thread containing the source's terminal Turns before one checkpoint.
    ///
    /// The source remains immutable. Repeating the request after the import event committed is
    /// safe and returns the already-created child projection.
    pub fn create_rewound_thread(
        &self,
        request: CreateRewoundThreadRequest,
    ) -> Result<ThreadSnapshot, CoreError> {
        let source = self.read_thread(&request.source_thread_id)?;
        if source.session_id != request.session_id {
            return Err(CoreError::InvalidInput(
                "rewind source belongs to another Session".into(),
            ));
        }
        let checkpoint = source
            .turns
            .iter()
            .position(|turn| turn.turn_id == request.before_turn_id)
            .ok_or_else(|| CoreError::NotFound(request.before_turn_id.to_string()))?;
        let imported_turns = source.public_thread().turns[..checkpoint].to_vec();
        let created = self.create_thread(CreateThreadRequest {
            session_id: request.session_id,
            thread_id: request.thread_id.clone(),
            title: request.title,
        })?;
        if created.sequence > 1 {
            return Ok(created);
        }

        let child_thread_id = request.thread_id;
        let event_thread_id = child_thread_id.clone();
        self.mutate_thread(&child_thread_id, |snapshot| {
            if snapshot.sequence > 1 {
                return Ok(snapshot.clone());
            }
            self.record_batch(
                snapshot,
                vec![ThreadEvent::HistoryImported {
                    thread_id: event_thread_id,
                    source_thread_id: request.source_thread_id,
                    before_turn_id: request.before_turn_id,
                    turns: imported_turns,
                }],
            )?;
            Ok(snapshot.clone())
        })
    }

    pub fn start_turn(
        &self,
        thread_id: &ThreadId,
        request: StartTurnRequest,
    ) -> Result<StartTurnResult, CoreError> {
        validate_command_id(&request.command_id)?;
        validate_policy_revision(&request.policy_revision)?;
        let normalized_input =
            user_input::normalize_images(&request.input, &self.image_attachments)?;
        if let Some(existing) = self
            .read_thread(thread_id)?
            .commands
            .into_iter()
            .find(|existing| existing.receipt.command_id == request.command_id)
        {
            let ThreadCommand::StartTurn {
                model,
                activated_skills,
                approval_mode,
                input,
            } = &existing.receipt.command
            else {
                return Err(CoreError::CommandConflict);
            };
            let host_activations = activated_skills
                .iter()
                .filter(|activation| activation.reason == SkillActivationReason::Automatic)
                .cloned()
                .collect::<Vec<_>>();
            if model != &request.model
                || host_activations != request.activated_skills
                || approval_mode != &request.approval_mode
                || input != &normalized_input
            {
                return Err(CoreError::CommandConflict);
            }
            let ThreadCommandResult::TurnAccepted { turn_id } = existing.result else {
                return Err(CoreError::Journal(
                    "start-Turn command has an invalid result".into(),
                ));
            };
            return Ok(StartTurnResult {
                turn_id,
                sequence: existing.response_sequence,
                disposition: StartTurnDisposition::Replayed,
            });
        }
        let mut activated_skills = request.activated_skills.clone();
        let contributed_activations = self
            .extensions
            .read()
            .map_err(|_| CoreError::Journal("extension registry lock poisoned".into()))?
            .contribute_skill_activations(zeta_extension_api::SkillActivationContext::new(
                &normalized_input,
            ))
            .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
        for activation in contributed_activations {
            if activated_skills
                .iter()
                .any(|existing| existing.id == activation.id)
            {
                return Err(CoreError::InvalidInput(format!(
                    "Skill '{}:{}' was activated more than once",
                    activation.id.source, activation.id.name
                )));
            }
            activated_skills.push(activation);
        }
        let validated_input = user_input::validate(&normalized_input, &activated_skills)?;
        let command = ThreadCommand::StartTurn {
            model: request.model.clone(),
            activated_skills: activated_skills.clone(),
            approval_mode: request.approval_mode,
            input: normalized_input.clone(),
        };
        self.mutate_thread(thread_id, |snapshot| {
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
                policy_revision: request.policy_revision.clone(),
                approval_mode: request.approval_mode,
                activated_skills: activated_skills.clone(),
                model: request.model.clone(),
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
        })
    }

    pub fn start_shell_turn(
        &self,
        thread_id: &ThreadId,
        request: StartShellTurnRequest,
    ) -> Result<StartTurnResult, CoreError> {
        validate_command_id(&request.command_id)?;
        validate_policy_revision(&request.policy_revision)?;
        let command_text = request.invocation.command.trim();
        if command_text.is_empty() {
            return Err(CoreError::InvalidInput(
                "Shell Turn command must not be empty".into(),
            ));
        }
        if request.invocation.shell_program.trim().is_empty() {
            return Err(CoreError::InvalidInput(
                "Shell Turn program must not be empty".into(),
            ));
        }
        if request.invocation.working_directory.trim().is_empty() {
            return Err(CoreError::InvalidInput(
                "Shell Turn working directory must not be empty".into(),
            ));
        }
        let command = ThreadCommand::StartShellTurn {
            command: request.invocation.command.clone(),
            approval_mode: request.approval_mode,
        };
        self.mutate_thread(thread_id, |snapshot| {
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
                        "start-Shell-Turn command has an invalid result".into(),
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
            let arguments_json = serde_json::to_string(&serde_json::json!({
                "program": request.invocation.shell_program,
                "arguments": ["-lc", request.invocation.command],
                "working_directory": request.invocation.working_directory,
            }))
            .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
            let tool_call = ThreadItem::ToolCall {
                item_id: ItemId::new(self.next_identifier("item"))
                    .expect("generated Item ID is non-empty"),
                turn_id: turn_id.clone(),
                tool_call_id: request.tool_call_id.clone(),
                name: ToolName::new("shell-command").expect("built-in shell-command name is valid"),
                arguments_json,
                binding: Some(request.binding.clone()),
            };
            let events = vec![
                ThreadEvent::TurnAccepted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    policy_revision: request.policy_revision.clone(),
                    approval_mode: request.approval_mode,
                    activated_skills: Vec::new(),
                    model: None,
                },
                ThreadEvent::ItemCompleted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    item: tool_call,
                },
                ThreadEvent::TurnStarted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                },
            ];
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
        self.mutate_thread(thread_id, |snapshot| {
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
        self.mutate_thread(thread_id, |snapshot| {
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
        self.mutate_thread(thread_id, |snapshot| {
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
        })
    }

    pub fn record_tool_call(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        request: RecordToolCallRequest,
    ) -> Result<RecordedToolCall, CoreError> {
        let tool_call_id = request.tool_call_id.unwrap_or_else(|| {
            ToolCallId::new(self.next_identifier("tool"))
                .expect("generated tool call ID is non-empty")
        });
        let item = ThreadItem::ToolCall {
            item_id: ItemId::new(self.next_identifier("item"))
                .expect("generated Item ID is non-empty"),
            turn_id: turn_id.clone(),
            tool_call_id: tool_call_id.clone(),
            name: request.name,
            arguments_json: request.arguments_json,
            binding: request.binding,
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
        let (text, content, is_error) = match request.output {
            ToolCallOutput::Success(text) => (text, None, false),
            ToolCallOutput::Failure(text) => (text, None, true),
            ToolCallOutput::SuccessContent(mut content) => {
                crate::image_preparation::prepare_tool_content(
                    &mut content,
                    &self.image_attachments,
                );
                (tool_content_preview(&content), Some(content), false)
            }
            ToolCallOutput::FailureContent(mut content) => {
                crate::image_preparation::prepare_tool_content(
                    &mut content,
                    &self.image_attachments,
                );
                (tool_content_preview(&content), Some(content), true)
            }
        };
        let item = ThreadItem::ToolResult {
            item_id: ItemId::new(self.next_identifier("item"))
                .expect("generated Item ID is non-empty"),
            turn_id: turn_id.clone(),
            tool_call_id: request.tool_call_id,
            text,
            content,
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
        let result = self.mutate_thread(thread_id, |snapshot| {
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
            Ok(InterruptTurnResult {
                sequence: snapshot.sequence,
                disposition: InterruptTurnDisposition::Interrupted,
            })
        })?;
        self.cancel_turn_execution(thread_id, &turn_id_for_cancellation);
        Ok(result)
    }

    pub fn read_thread(&self, thread_id: &ThreadId) -> Result<ThreadSnapshot, CoreError> {
        self.with_loaded_thread(thread_id, |loaded| Ok(loaded.snapshot.clone()))
    }

    pub(crate) fn commit_context_checkpoint(
        &self,
        thread_id: &ThreadId,
        request: CommitContextCheckpointRequest,
    ) -> Result<ContextCheckpoint, CoreError> {
        if request.summary.trim().is_empty()
            || request.schema_revision.trim().is_empty()
            || request.prompt_revision.trim().is_empty()
            || request.context_policy_revision.trim().is_empty()
        {
            return Err(CoreError::InvalidInput(
                "context checkpoint summary and revision identities must not be empty".into(),
            ));
        }
        self.mutate_thread(thread_id, |snapshot| {
            if snapshot.sequence != request.source_thread_sequence {
                return Err(CoreError::ThreadStore(ThreadStoreError::SequenceConflict {
                    expected: request.source_thread_sequence,
                    actual: snapshot.sequence,
                }));
            }
            let checkpoint = ContextCheckpoint {
                checkpoint_id: ContextCheckpointId::new(self.next_identifier("context-checkpoint"))
                    .expect("generated context checkpoint ID is non-empty"),
                source_thread_id: snapshot.thread_id.clone(),
                covered: request.covered,
                referenced_items: snapshot
                    .items
                    .iter()
                    .filter(|item| {
                        snapshot
                            .item_sequences
                            .get(item.item_id())
                            .is_some_and(|sequence| *sequence <= request.covered.end_sequence)
                    })
                    .map(|item| item.item_id().clone())
                    .collect(),
                source_digest: snapshot.context_source_digest(request.covered)?,
                summary: request.summary,
                schema_revision: request.schema_revision,
                prompt_revision: request.prompt_revision,
                context_policy_revision: request.context_policy_revision,
                generator_model: request.generator_model,
                created_at_unix_ms: u64::try_from(self.timestamp()?.0).map_err(|_| {
                    CoreError::Journal("context checkpoint timestamp exceeds u64".into())
                })?,
                verification: ContextCheckpointVerification::Verified,
            };
            self.record_batch(
                snapshot,
                vec![ThreadEvent::ContextCheckpointCommitted {
                    thread_id: thread_id.clone(),
                    checkpoint: checkpoint.clone(),
                }],
            )?;
            Ok(checkpoint)
        })
    }

    pub(crate) fn prepare_model_invocation(
        &self,
        thread_id: &ThreadId,
        request: PrepareModelInvocationRequest<'_>,
    ) -> Result<ModelInvocationPreparation, CoreError> {
        self.with_loaded_thread(thread_id, |loaded| {
            let turn = loaded
                .snapshot
                .turns
                .iter()
                .find(|turn| &turn.turn_id == request.turn_id)
                .ok_or_else(|| CoreError::NotFound(request.turn_id.to_string()))?;
            let model = match &turn.model {
                Some(model) => FrozenModelSelection::Selected(model.clone()),
                None => FrozenModelSelection::ConfiguredDefault,
            };
            let mut instruction_fragments = request.instructions.context_fragments();
            instruction_fragments.extend(crate::multi_agent::agent_context_fragments(
                &loaded.snapshot,
            ));
            instruction_fragments.extend(
                request
                    .extension_fragments
                    .into_iter()
                    .map(crate::context::InstructionFragment::try_from)
                    .collect::<Result<Vec<_>, _>>()?,
            );
            let tools = crate::multi_agent::scope_agent_tools(&loaded.snapshot, request.tools);
            let input = ContextInput::new(
                &loaded.snapshot,
                request.turn_id.clone(),
                instruction_fragments,
                tools,
                request.budget,
            )
            .with_evidence(request.evidence);
            match loaded
                .context
                .prepare(&input)
                .map_err(|error| CoreError::Context(error.to_string()))?
            {
                ContextPreparation::Ready(context) => Ok(ModelInvocationPreparation::Ready(
                    ModelInvocationSnapshot::new(
                        loaded.snapshot.session_id.clone(),
                        loaded.snapshot.thread_id.clone(),
                        request.turn_id.clone(),
                        model,
                        context,
                    ),
                )),
                ContextPreparation::NeedsCompaction(plan) => {
                    Ok(ModelInvocationPreparation::NeedsCompaction { model, plan })
                }
            }
        })
    }

    /// Returns the in-memory projections currently loaded by this manager.
    pub fn list_threads(&self) -> Result<Vec<ThreadSnapshot>, CoreError> {
        self.loaded_threads.loaded_snapshots()
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
        self.execution_mailboxes.retire_idle(thread_id)?;
        let slot = self.loaded_threads.slot(thread_id)?;
        let _permit = slot.enter_mutation()?;
        let _lease = self.acquire_writer_lease(thread_id)?;
        let mut loaded = slot
            .loaded
            .lock()
            .map_err(|_| CoreError::Journal("loaded Thread state lock poisoned".into()))?;
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
        *loaded = Some(self.loaded_threads.install(snapshot.clone()));
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
        self.mutate_thread(thread_id, |snapshot| self.record_batch(snapshot, events))
    }

    fn record_item(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        item: ThreadItem,
    ) -> Result<u64, CoreError> {
        self.mutate_thread(thread_id, |snapshot| {
            self.record_batch(
                snapshot,
                vec![ThreadEvent::ItemCompleted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    item,
                }],
            )?;
            Ok(snapshot.sequence)
        })
    }

    fn mutate_thread<R>(
        &self,
        thread_id: &ThreadId,
        mutation: impl FnOnce(&mut ThreadSnapshot) -> Result<R, CoreError>,
    ) -> Result<R, CoreError> {
        let slot = self.loaded_threads.slot(thread_id)?;
        let _permit = slot.enter_mutation()?;
        let _lease = self.acquire_writer_lease(thread_id)?;
        let mut loaded = slot
            .loaded
            .lock()
            .map_err(|_| CoreError::Journal("loaded Thread state lock poisoned".into()))?;
        if loaded.is_none() {
            let snapshot = self.load_snapshot(thread_id)?;
            *loaded = Some(self.loaded_threads.install(snapshot));
        }
        mutation(
            &mut loaded
                .as_mut()
                .expect("loaded Thread state was installed above")
                .snapshot,
        )
    }

    fn with_loaded_thread<R>(
        &self,
        thread_id: &ThreadId,
        operation: impl FnOnce(&mut loaded_thread::LoadedThreadState) -> Result<R, CoreError>,
    ) -> Result<R, CoreError> {
        let slot = self.loaded_threads.slot(thread_id)?;
        let mut loaded = slot
            .loaded
            .lock()
            .map_err(|_| CoreError::Journal("loaded Thread state lock poisoned".into()))?;
        if loaded.is_none() {
            let snapshot = self.load_snapshot(thread_id)?;
            *loaded = Some(self.loaded_threads.install(snapshot));
        }
        operation(
            loaded
                .as_mut()
                .expect("loaded Thread state was installed above"),
        )
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

fn tool_content_preview(content: &[ContentPart]) -> String {
    content
        .iter()
        .map(|part| match part {
            ContentPart::Text(text) => text.as_str(),
            ContentPart::ImageAttachment { .. } => "[image]",
            ContentPart::ImageUrl { .. } => "[image]",
        })
        .collect::<Vec<_>>()
        .join("\n")
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

fn validate_policy_revision(policy_revision: &str) -> Result<(), CoreError> {
    if policy_revision.trim().is_empty() {
        Err(CoreError::InvalidInput(
            "Turn policy revision must be non-empty".into(),
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
