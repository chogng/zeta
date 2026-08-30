use crate::ContextBudget;
use crate::CoreError;
use crate::HarnessContext;
use crate::ThreadCommandResult;
use crate::ThreadEventBatch;
use crate::ThreadSnapshot;
use crate::ThreadStore;
use crate::ThreadWorktreeBinder;
use crate::ThreadWorktreeBindingRequest;
use crate::WriterLease;
use crate::reduce_thread_event;
use crate::thread_reducer::validate_agent_request;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use zeta_attachments::ImageAttachments;
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
use zeta_protocol::ThreadGoal;
use zeta_protocol::ThreadGoalStatus;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadItem;
use zeta_protocol::ThreadOrigin;
use zeta_protocol::ThreadUpdate;
use zeta_protocol::ThreadUpdateEnvelope;
use zeta_protocol::ToolCallBinding;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolDefinition;
use zeta_protocol::ToolName;
use zeta_protocol::TurnId;
use zeta_protocol::TurnInstructions;
use zeta_protocol::TurnInteraction;
use zeta_protocol::TurnKind;
use zeta_protocol::TurnStatus;
use zeta_protocol::UserInput;
use zeta_thread_store::AppendBatchResult;
use zeta_thread_store::ThreadStoreError;
use zeta_thread_store::validate_append_batch;

mod agent;
mod context;
mod execution;
pub(crate) mod live_interaction;
mod loaded_thread;
mod mailbox;
mod steering;
mod user_input;

pub use agent::CreateAgentThreadRequest;
pub use mailbox::ThreadExecutionContext;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequenceExpectation {
    Any,
    Exact(u64),
}

pub struct StartTurnRequest {
    pub command_id: CommandId,
    pub expected_sequence: SequenceExpectation,
    pub model: Option<ModelRef>,
    pub kind: TurnKind,
    pub instructions: TurnInstructions,
    pub policy_revision: String,
    /// Host-seeded automatic activations. Explicit selections are resolved by extensions.
    pub approval_mode: ApprovalMode,
    pub tool_mode: zeta_protocol::ToolMode,
    pub tool_profile: Option<zeta_protocol::ToolProfileSnapshot>,
    pub activated_skills: Vec<FrozenSkillActivation>,
    pub input: Vec<UserInput>,
}

/// Internal continuation request created when an active Thread Goal reaches a Turn boundary.
///
/// Goal continuations intentionally carry no user input. The Goal prompt is injected at the next
/// model-invocation boundary, so the continuation is durable without manufacturing a user
/// message that was never sent.
pub struct StartGoalTurnRequest {
    pub command_id: CommandId,
    pub model: Option<ModelRef>,
    pub instructions: TurnInstructions,
    pub policy_revision: String,
    pub approval_mode: ApprovalMode,
    pub tool_mode: zeta_protocol::ToolMode,
    pub tool_profile: Option<zeta_protocol::ToolProfileSnapshot>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdatePlanDisposition {
    Changed,
    Unchanged,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdatePlanResult {
    pub sequence: u64,
    pub disposition: UpdatePlanDisposition,
}

/// Named inputs for preparing one immutable model invocation from durable Thread state.
pub(crate) struct PrepareModelInvocationRequest<'a> {
    pub turn_id: &'a TurnId,
    pub harness_context: &'a HarnessContext,
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

/// Client command that runs standalone manual context compaction without adding a user message.
pub struct StartContextCompactionRequest {
    pub command_id: CommandId,
    pub expected_sequence: SequenceExpectation,
    pub model: Option<ModelRef>,
    pub policy_revision: String,
    pub retention_prompt: Option<String>,
}

pub struct CreateThreadRequest {
    pub session_id: SessionId,
    pub thread_id: ThreadId,
    pub title: String,
}

pub struct StartThreadRequest {
    pub command_id: CommandId,
    pub title: String,
}

pub struct CreateBranchRequest {
    pub command_id: CommandId,
    pub session_id: SessionId,
    pub title: String,
}

pub struct ForkThreadRequest {
    pub command_id: CommandId,
    pub source_thread_id: ThreadId,
    pub title: String,
}

pub struct RewindThreadRequest {
    pub command_id: CommandId,
    pub source_thread_id: ThreadId,
    pub before_turn_id: TurnId,
    pub title: String,
}

pub struct CreateRewoundThreadRequest {
    pub session_id: SessionId,
    pub thread_id: ThreadId,
    pub title: String,
    pub source_thread_id: ThreadId,
    pub before_turn_id: TurnId,
}

pub struct CreateForkedThreadRequest {
    pub session_id: SessionId,
    pub thread_id: ThreadId,
    pub title: String,
    pub source_thread_id: ThreadId,
    pub source_sequence: u64,
}

/// Fields controlled by the Thread Goal API. `token_budget` is a double option so callers can
/// distinguish "leave unchanged" from "clear the budget".
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SetGoalRequest {
    pub objective: Option<String>,
    pub status: Option<ThreadGoalStatus>,
    pub token_budget: Option<Option<u64>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetGoalResult {
    pub goal: ThreadGoal,
    pub changed: bool,
    pub created: bool,
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

/// Retry-safe client command that appends user input to one active Turn.
pub struct SteerTurnRequest {
    pub command_id: CommandId,
    pub expected_sequence: SequenceExpectation,
    pub turn_id: TurnId,
    pub input: Vec<UserInput>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SteerTurnDisposition {
    Steered,
    Replayed,
}

pub struct SteerTurnResult {
    pub sequence: u64,
    pub disposition: SteerTurnDisposition,
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
    /// True when the response resumed an in-process Tool Call that is still executing.
    pub live_execution_woken: bool,
}

/// Execution action that closes an outstanding interaction without accepting a client response.
pub struct CancelTurnInteractionRequest {
    pub turn_id: TurnId,
    pub request_id: RequestId,
    pub reason: InteractionCancelReason,
}

pub struct CancelledTurnInteraction {
    pub sequence: u64,
    /// True when cancellation resumed an in-process Tool Call that is still executing.
    pub live_execution_woken: bool,
}

pub struct CompletedTurn {
    pub item: ThreadItem,
    pub sequence: u64,
}

pub(crate) enum CompleteModelInvocationResult {
    Completed(CompletedTurn),
    SupersededBySteer,
}

pub(crate) enum CommitModelInvocationItemsResult {
    Committed,
    SupersededBySteer,
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

enum ContextCheckpointCommitKind {
    Automatic,
    OverflowRecovery(TurnId),
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

struct ExtensionRegistries {
    fallback: Arc<zeta_extension_api::ExtensionRegistry>,
    sessions: BTreeMap<SessionId, Arc<zeta_extension_api::ExtensionRegistry>>,
}

impl Default for ExtensionRegistries {
    fn default() -> Self {
        Self {
            fallback: Arc::new(zeta_extension_api::ExtensionRegistry::default()),
            sessions: BTreeMap::new(),
        }
    }
}

/// Coordinates durable mutations for each loaded Thread.
pub struct ThreadController {
    store: Arc<dyn ThreadStore>,
    writer_lease: Option<Arc<dyn WriterLease<ThreadId>>>,
    loaded_threads: Arc<loaded_thread::LoadedThreads>,
    execution_mailboxes: mailbox::ThreadExecutionMailboxes,
    pub(crate) live_interactions: live_interaction::LiveInteractionWaiters,
    extensions: RwLock<ExtensionRegistries>,
    thread_worktree_binder: RwLock<Arc<dyn ThreadWorktreeBinder>>,
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
            live_interactions: live_interaction::LiveInteractionWaiters::default(),
            extensions: RwLock::new(ExtensionRegistries::default()),
            thread_worktree_binder: RwLock::new(Arc::new(crate::NoThreadWorktreeBinder)),
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
            live_interactions: live_interaction::LiveInteractionWaiters::default(),
            extensions: RwLock::new(ExtensionRegistries::default()),
            thread_worktree_binder: RwLock::new(Arc::new(crate::NoThreadWorktreeBinder)),
            image_attachments,
            loaded_threads,
            next_id: AtomicU64::new(1),
        }
    }

    /// Returns the canonical service used by this Thread authority and its model executors.
    pub fn image_attachments(&self) -> Arc<ImageAttachments> {
        Arc::clone(&self.image_attachments)
    }

    /// Installs the host authority that binds a Worktree before a Thread is created.
    pub fn install_thread_worktree_binder(
        &self,
        binder: Arc<dyn ThreadWorktreeBinder>,
    ) -> Result<(), CoreError> {
        *self
            .thread_worktree_binder
            .write()
            .map_err(|_| CoreError::Journal("Thread Worktree binder lock poisoned".into()))? =
            binder;
        Ok(())
    }

    /// Installs the shared agent extension registry before product Turns are accepted.
    pub fn install_extensions(
        &self,
        extensions: Arc<zeta_extension_api::ExtensionRegistry>,
    ) -> Result<(), CoreError> {
        self.extensions
            .write()
            .map_err(|_| CoreError::Journal("extension registry lock poisoned".into()))?
            .fallback = extensions;
        Ok(())
    }

    /// Installs the extension registry used when starting Turns in one Session.
    ///
    /// Profile daemons call this for each durable Session so concurrently open environments cannot
    /// replace one another's automatic Skill activation authority.
    pub fn install_session_extensions(
        &self,
        session_id: SessionId,
        extensions: Arc<zeta_extension_api::ExtensionRegistry>,
    ) -> Result<(), CoreError> {
        self.extensions
            .write()
            .map_err(|_| CoreError::Journal("extension registry lock poisoned".into()))?
            .sessions
            .insert(session_id, extensions);
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

    /// Starts one root Thread. Its Thread ID is also the Session tree identity.
    pub fn start_thread(&self, request: StartThreadRequest) -> Result<ThreadSnapshot, CoreError> {
        validate_thread_title(&request.command_id, &request.title)?;
        let thread_id = command_thread_id("thread", &request.command_id)?;
        let session_id = SessionId::new(thread_id.as_str().to_owned())
            .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
        self.bind_thread_worktree(session_id.clone(), thread_id.clone(), ThreadOrigin::Root)?;
        self.create_thread(CreateThreadRequest {
            session_id,
            thread_id,
            title: request.title,
        })
    }

    /// Creates an empty branch in an existing Session tree.
    pub fn create_branch(&self, request: CreateBranchRequest) -> Result<ThreadSnapshot, CoreError> {
        validate_thread_title(&request.command_id, &request.title)?;
        if self.list_session_threads(&request.session_id)?.is_empty() {
            return Err(CoreError::NotFound(request.session_id.to_string()));
        }
        let thread_id = command_thread_id("branch", &request.command_id)?;
        self.bind_thread_worktree(
            request.session_id.clone(),
            thread_id.clone(),
            ThreadOrigin::Root,
        )?;
        self.create_thread(CreateThreadRequest {
            session_id: request.session_id,
            thread_id,
            title: request.title,
        })
    }

    /// Forks one durable Thread at its current sequence while preserving its Session tree ID.
    pub fn fork_thread(&self, request: ForkThreadRequest) -> Result<ThreadSnapshot, CoreError> {
        validate_thread_title(&request.command_id, &request.title)?;
        let source = self.read_thread(&request.source_thread_id)?;
        let thread_id = command_thread_id("fork", &request.command_id)?;
        self.bind_thread_worktree(
            source.session_id.clone(),
            thread_id.clone(),
            ThreadOrigin::Fork {
                parent_thread_id: source.thread_id.clone(),
                parent_sequence: source.sequence,
            },
        )?;
        self.create_forked_thread(CreateForkedThreadRequest {
            session_id: source.session_id,
            thread_id,
            title: request.title,
            source_thread_id: source.thread_id,
            source_sequence: source.sequence,
        })
    }

    /// Rewinds one durable Thread into a new branch before the selected Turn.
    pub fn rewind_thread(&self, request: RewindThreadRequest) -> Result<ThreadSnapshot, CoreError> {
        validate_thread_title(&request.command_id, &request.title)?;
        let source = self.read_thread(&request.source_thread_id)?;
        if !source
            .turns
            .iter()
            .any(|turn| turn.turn_id == request.before_turn_id)
        {
            return Err(CoreError::NotFound(request.before_turn_id.to_string()));
        }
        let thread_id = command_thread_id("rewind", &request.command_id)?;
        self.bind_thread_worktree(
            source.session_id.clone(),
            thread_id.clone(),
            ThreadOrigin::Rewind {
                parent_thread_id: source.thread_id.clone(),
                parent_sequence: source.sequence,
                before_turn_id: request.before_turn_id.clone(),
            },
        )?;
        self.create_rewound_thread(CreateRewoundThreadRequest {
            session_id: source.session_id,
            thread_id,
            title: request.title,
            source_thread_id: source.thread_id,
            before_turn_id: request.before_turn_id,
        })
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
        let mut imported_turns = source.public_thread().turns[..checkpoint].to_vec();
        for turn in &mut imported_turns {
            turn.usage = zeta_protocol::ModelUsageSummary::default();
        }
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

    /// Creates a child Thread containing the source history at one exact fork point.
    ///
    /// The source prefix is replayed from durable events so Session saga recovery cannot import
    /// parent updates committed after the recorded fork sequence.
    pub fn create_forked_thread(
        &self,
        request: CreateForkedThreadRequest,
    ) -> Result<ThreadSnapshot, CoreError> {
        let source =
            self.load_snapshot_at_sequence(&request.source_thread_id, request.source_sequence)?;
        if source.session_id != request.session_id {
            return Err(CoreError::InvalidInput(
                "fork source belongs to another Session".into(),
            ));
        }
        let imported_turns = fork_snapshot_turns(source.public_thread().turns);
        let context_checkpoint = inherited_fork_checkpoint(&source, &imported_turns)?;
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
            let imported_turn_count = u64::try_from(imported_turns.len())
                .map_err(|_| CoreError::Journal("fork Turn count exceeds u64".into()))?;
            let mut events = imported_turns
                .into_iter()
                .enumerate()
                .map(|(turn_index, turn)| {
                    Ok(ThreadEvent::ForkTurnImported {
                        thread_id: event_thread_id.clone(),
                        source_thread_id: request.source_thread_id.clone(),
                        source_sequence: request.source_sequence,
                        turn_index: u64::try_from(turn_index).map_err(|_| {
                            CoreError::Journal("fork Turn index exceeds u64".into())
                        })?,
                        turn: Box::new(turn),
                    })
                })
                .collect::<Result<Vec<_>, CoreError>>()?;
            events.push(ThreadEvent::ForkHistoryImportCompleted {
                thread_id: event_thread_id,
                source_thread_id: request.source_thread_id,
                source_sequence: request.source_sequence,
                imported_turn_count,
                context_checkpoint,
            });
            self.record_batch(snapshot, events)?;
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
        request
            .instructions
            .validate()
            .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
        let normalized_input =
            user_input::normalize_images(&request.input, &self.image_attachments)?;
        let thread = self.read_thread(thread_id)?;
        let session_id = thread.session_id.clone();
        if let Some(existing) = thread
            .commands
            .into_iter()
            .find(|existing| existing.receipt.command_id == request.command_id)
        {
            let ThreadCommand::StartTurn {
                kind,
                instructions,
                model,
                activated_skills,
                host_activated_skills,
                approval_mode,
                tool_mode,
                tool_profile,
                input,
                ..
            } = &existing.receipt.command
            else {
                return Err(CoreError::CommandConflict);
            };
            if kind != &request.kind
                || instructions.as_ref() != Some(&request.instructions)
                || model != &request.model
                || replay_host_activations(host_activated_skills.as_deref(), activated_skills)
                    != request.activated_skills
                || approval_mode != &request.approval_mode
                || tool_mode != &request.tool_mode
                || tool_profile.as_deref() != request.tool_profile.as_ref()
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
        let agent_skill_ceiling = thread
            .agent_context_seed
            .as_ref()
            .map(|seed| seed.capability_scope.skills.clone());
        if request
            .activated_skills
            .iter()
            .any(|activation| !skill_is_within_ceiling(agent_skill_ceiling.as_deref(), activation))
        {
            return Err(CoreError::InvalidInput(
                "Agent Turn cannot expand its frozen Skill capability ceiling".into(),
            ));
        }
        let mut activated_skills = request.activated_skills.clone();
        let registries = self
            .extensions
            .read()
            .map_err(|_| CoreError::Journal("extension registry lock poisoned".into()))?;
        let extensions = registries
            .sessions
            .get(&session_id)
            .unwrap_or(&registries.fallback);
        let contributed_activations = extensions
            .contribute_skill_activations(zeta_extension_api::SkillActivationContext::for_session(
                &session_id,
                &normalized_input,
            ))
            .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
        for activation in contributed_activations {
            if !skill_is_within_ceiling(agent_skill_ceiling.as_deref(), &activation) {
                if activation.reason == SkillActivationReason::Automatic {
                    continue;
                }
                return Err(CoreError::InvalidInput(format!(
                    "Agent Turn cannot activate Skill '{}:{}' outside its frozen capability ceiling",
                    activation.id.source, activation.id.name
                )));
            }
            if let Some(existing) = activated_skills
                .iter()
                .find(|existing| existing.id == activation.id)
            {
                if existing.content_digest == activation.content_digest
                    && existing.catalog_generation == activation.catalog_generation
                {
                    continue;
                }
                return Err(CoreError::InvalidInput(format!(
                    "Skill '{}:{}' resolved to conflicting frozen activations",
                    activation.id.source, activation.id.name
                )));
            }
            activated_skills.push(activation);
        }
        let validated_input = user_input::validate(&normalized_input, &activated_skills)?;
        let command = ThreadCommand::StartTurn {
            kind: request.kind,
            instructions: Some(request.instructions.clone()),
            model: request.model.clone(),
            activated_skills: activated_skills.clone(),
            host_activated_skills: Some(request.activated_skills.clone()),
            approval_mode: request.approval_mode,
            tool_mode: request.tool_mode,
            tool_profile: request.tool_profile.clone().map(Box::new),
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
                kind: request.kind,
                instructions: Some(request.instructions.clone()),
                policy_revision: request.policy_revision.clone(),
                approval_mode: request.approval_mode,
                tool_mode: request.tool_mode,
                activated_skills: activated_skills.clone(),
                model: request.model.clone(),
                tool_profile: request.tool_profile.clone(),
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

    /// Starts one durable no-input continuation for the active Goal, if the Thread is idle.
    ///
    /// The mutation is guarded by the same per-Thread gate as ordinary Turns. This makes the
    /// completion-to-continuation boundary safe when a user command, recovery pass, or another
    /// executor observes the same completed Turn concurrently.
    pub fn start_goal_turn(
        &self,
        thread_id: &ThreadId,
        request: StartGoalTurnRequest,
    ) -> Result<Option<StartTurnResult>, CoreError> {
        validate_command_id(&request.command_id)?;
        validate_policy_revision(&request.policy_revision)?;
        request
            .instructions
            .validate()
            .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
        let command = ThreadCommand::StartTurn {
            kind: TurnKind::Coding,
            instructions: Some(request.instructions.clone()),
            model: request.model.clone(),
            activated_skills: Vec::new(),
            host_activated_skills: Some(Vec::new()),
            approval_mode: request.approval_mode,
            tool_mode: request.tool_mode,
            tool_profile: request.tool_profile.clone().map(Box::new),
            input: Vec::new(),
        };
        self.mutate_thread(thread_id, |snapshot| {
            let Some(goal) = snapshot.goal.as_ref() else {
                return Ok(None);
            };
            if !goal.status.is_active()
                || goal
                    .token_budget
                    .is_some_and(|budget| goal.tokens_used >= budget)
                || snapshot.turns.iter().any(|turn| {
                    !matches!(
                        turn.status,
                        TurnStatus::Completed | TurnStatus::Failed | TurnStatus::Interrupted
                    )
                })
            {
                return Ok(None);
            }
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
                        "Goal continuation command has an invalid result".into(),
                    ));
                };
                return Ok(Some(StartTurnResult {
                    turn_id: turn_id.clone(),
                    sequence: existing.response_sequence,
                    disposition: StartTurnDisposition::Replayed,
                }));
            }

            let turn_id =
                TurnId::new(self.next_identifier("turn")).expect("generated Turn ID is non-empty");
            let (next_snapshot, batch) = self.project_batch(
                Some(snapshot.clone()),
                &snapshot.thread_id,
                vec![
                    ThreadEvent::TurnAccepted {
                        thread_id: thread_id.clone(),
                        turn_id: turn_id.clone(),
                        kind: TurnKind::Coding,
                        instructions: Some(request.instructions.clone()),
                        policy_revision: request.policy_revision.clone(),
                        approval_mode: request.approval_mode,
                        tool_mode: request.tool_mode,
                        activated_skills: Vec::new(),
                        model: request.model.clone(),
                        tool_profile: request.tool_profile.clone(),
                    },
                    ThreadEvent::TurnStarted {
                        thread_id: thread_id.clone(),
                        turn_id: turn_id.clone(),
                    },
                ],
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
            Ok(Some(StartTurnResult {
                turn_id,
                sequence: snapshot.sequence,
                disposition: StartTurnDisposition::Created,
            }))
        })
    }

    pub fn get_goal(&self, thread_id: &ThreadId) -> Result<Option<ThreadGoal>, CoreError> {
        Ok(self.read_thread(thread_id)?.goal)
    }

    /// Creates or updates the single Goal owned by a Thread.
    pub fn set_goal(
        &self,
        thread_id: &ThreadId,
        request: SetGoalRequest,
    ) -> Result<SetGoalResult, CoreError> {
        self.mutate_thread(thread_id, |snapshot| {
            let Some(current) = snapshot.goal.clone() else {
                let objective = request.objective.clone().ok_or_else(|| {
                    CoreError::InvalidInput("creating a Goal requires an objective".into())
                })?;
                let goal = make_goal(
                    snapshot.thread_id.clone(),
                    self.next_identifier("goal"),
                    objective,
                    request.status.unwrap_or(ThreadGoalStatus::Active),
                    request.token_budget.flatten(),
                    0,
                )?;
                self.record_batch(
                    snapshot,
                    vec![ThreadEvent::GoalCreated {
                        thread_id: snapshot.thread_id.clone(),
                        goal: goal.clone(),
                    }],
                )?;
                return Ok(SetGoalResult {
                    goal,
                    changed: true,
                    created: true,
                });
            };

            let goal = make_goal(
                snapshot.thread_id.clone(),
                current.goal_id.clone(),
                request
                    .objective
                    .unwrap_or_else(|| current.objective.clone()),
                request.status.unwrap_or(current.status),
                request.token_budget.unwrap_or(current.token_budget),
                current.tokens_used,
            )?;
            if goal == current {
                return Ok(SetGoalResult {
                    goal,
                    changed: false,
                    created: false,
                });
            }
            self.record_batch(
                snapshot,
                vec![ThreadEvent::GoalUpdated {
                    thread_id: snapshot.thread_id.clone(),
                    goal: goal.clone(),
                }],
            )?;
            Ok(SetGoalResult {
                goal,
                changed: true,
                created: false,
            })
        })
    }

    /// Creates a new Goal, rejecting another unfinished Goal. A completed Goal is cleared and
    /// replaced atomically in the same Thread event batch.
    pub fn create_goal(
        &self,
        thread_id: &ThreadId,
        objective: String,
        token_budget: Option<u64>,
    ) -> Result<ThreadGoal, CoreError> {
        self.mutate_thread(thread_id, |snapshot| {
            if let Some(current) = snapshot.goal.as_ref()
                && !current.status.is_complete()
            {
                return Err(CoreError::InvalidInput(
                    "Thread already has an unfinished Goal".into(),
                ));
            }
            let goal = make_goal(
                snapshot.thread_id.clone(),
                self.next_identifier("goal"),
                objective,
                ThreadGoalStatus::Active,
                token_budget,
                0,
            )?;
            let mut events = Vec::with_capacity(2);
            if let Some(current) = snapshot.goal.as_ref() {
                events.push(ThreadEvent::GoalCleared {
                    thread_id: snapshot.thread_id.clone(),
                    goal_id: current.goal_id.clone(),
                });
            }
            events.push(ThreadEvent::GoalCreated {
                thread_id: snapshot.thread_id.clone(),
                goal: goal.clone(),
            });
            self.record_batch(snapshot, events)?;
            Ok(goal)
        })
    }

    pub fn clear_goal(&self, thread_id: &ThreadId) -> Result<bool, CoreError> {
        self.mutate_thread(thread_id, |snapshot| {
            let Some(goal_id) = snapshot.goal.as_ref().map(|goal| goal.goal_id.clone()) else {
                return Ok(false);
            };
            self.record_batch(
                snapshot,
                vec![ThreadEvent::GoalCleared {
                    thread_id: snapshot.thread_id.clone(),
                    goal_id,
                }],
            )?;
            Ok(true)
        })
    }

    pub fn start_context_compaction(
        &self,
        thread_id: &ThreadId,
        request: StartContextCompactionRequest,
    ) -> Result<StartTurnResult, CoreError> {
        const MAX_RETENTION_PROMPT_BYTES: usize = 8 * 1024;

        validate_command_id(&request.command_id)?;
        validate_policy_revision(&request.policy_revision)?;
        let retention_prompt = request
            .retention_prompt
            .as_deref()
            .map(str::trim)
            .filter(|prompt| !prompt.is_empty())
            .map(str::to_owned);
        if retention_prompt
            .as_ref()
            .is_some_and(|prompt| prompt.len() > MAX_RETENTION_PROMPT_BYTES)
        {
            return Err(CoreError::InvalidInput(format!(
                "context compaction retention prompt exceeds {MAX_RETENTION_PROMPT_BYTES} bytes"
            )));
        }
        let command = ThreadCommand::CompactContext {
            model: request.model.clone(),
            retention_prompt: retention_prompt.clone(),
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
                        "context compaction command has an invalid result".into(),
                    ));
                };
                return Ok(StartTurnResult {
                    turn_id: turn_id.clone(),
                    sequence: existing.response_sequence,
                    disposition: StartTurnDisposition::Replayed,
                });
            }
            validate_thread_expectation(request.expected_sequence, snapshot.sequence)?;
            if snapshot.turns.iter().any(|turn| {
                !matches!(
                    turn.status,
                    zeta_protocol::TurnStatus::Completed
                        | zeta_protocol::TurnStatus::Failed
                        | zeta_protocol::TurnStatus::Interrupted
                )
            }) {
                return Err(CoreError::InvalidInput(
                    "manual context compaction requires every existing Turn to be terminal".into(),
                ));
            }
            let turn_id =
                TurnId::new(self.next_identifier("turn")).expect("generated Turn ID is non-empty");
            let events = vec![
                ThreadEvent::TurnAccepted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    kind: TurnKind::Coding,
                    instructions: None,
                    policy_revision: request.policy_revision.clone(),
                    approval_mode: ApprovalMode::AskPermissions,
                    tool_mode: zeta_protocol::ToolMode::Direct,
                    activated_skills: Vec::new(),
                    model: request.model.clone(),
                    tool_profile: None,
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
                    kind: TurnKind::Coding,
                    instructions: None,
                    policy_revision: request.policy_revision.clone(),
                    approval_mode: request.approval_mode,
                    tool_mode: zeta_protocol::ToolMode::Direct,
                    activated_skills: Vec::new(),
                    model: None,
                    tool_profile: None,
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
        let live_key = live_interaction::LiveInteractionKey {
            thread_id: thread_id.clone(),
            turn_id: request.turn_id.clone(),
            request_id: request.request_id.clone(),
        };
        let live_response = request.response.clone();
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
                    live_execution_woken: false,
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
                live_execution_woken: false,
            })
        })?;
        if result.disposition == ResolveTurnInteractionDisposition::Resolved {
            let live_execution_woken = self.live_interactions.resolve(&live_key, live_response);
            return Ok(ResolveTurnInteractionResult {
                live_execution_woken,
                ..result
            });
        }
        Ok(result)
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
        let live_key = live_interaction::LiveInteractionKey {
            thread_id: thread_id.clone(),
            turn_id: request.turn_id.clone(),
            request_id: request.request_id.clone(),
        };
        let reason = request.reason;
        let result = self.mutate_thread(thread_id, |snapshot| {
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
                live_execution_woken: false,
            })
        })?;
        Ok(CancelledTurnInteraction {
            live_execution_woken: self.live_interactions.cancel(&live_key, reason),
            ..result
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

    /// Replaces the current Turn plan with one validated durable projection.
    pub fn update_plan(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        plan: zeta_protocol::PlanUpdate,
    ) -> Result<UpdatePlanResult, CoreError> {
        crate::turn::validate_plan_update(&plan).map_err(CoreError::InvalidInput)?;
        self.mutate_thread(thread_id, |snapshot| {
            let turn = snapshot
                .turns
                .iter()
                .find(|turn| &turn.turn_id == turn_id)
                .ok_or_else(|| CoreError::NotFound(turn_id.to_string()))?;
            if turn.status != TurnStatus::Running {
                return Err(CoreError::Execution(format!(
                    "cannot update the plan for a {:?} Turn",
                    turn.status
                )));
            }
            if turn.plan.as_ref() == Some(&plan) {
                return Ok(UpdatePlanResult {
                    sequence: snapshot.sequence,
                    disposition: UpdatePlanDisposition::Unchanged,
                });
            }
            self.record_batch(
                snapshot,
                vec![ThreadEvent::PlanUpdated {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    plan,
                }],
            )?;
            Ok(UpdatePlanResult {
                sequence: snapshot.sequence,
                disposition: UpdatePlanDisposition::Changed,
            })
        })
    }

    pub fn record_tool_result(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        request: RecordToolResultRequest,
    ) -> Result<RecordedToolResult, CoreError> {
        let (mut text, mut content, is_error) = match request.output {
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
        let item_id =
            ItemId::new(self.next_identifier("item")).expect("generated Item ID is non-empty");
        self.mutate_thread(thread_id, |snapshot| {
            let failure_count = crate::tool_repetition::next_tool_failure_count(
                &snapshot.items,
                turn_id,
                &request.tool_call_id,
                is_error,
            )?;
            if failure_count == crate::tool_repetition::TOOL_REPETITION_REMINDER_THRESHOLD {
                append_tool_repetition_reminder(&mut text, &mut content);
            }
            let item = ThreadItem::ToolResult {
                item_id,
                turn_id: turn_id.clone(),
                tool_call_id: request.tool_call_id,
                text,
                content,
                is_error,
            };
            self.record_batch(
                snapshot,
                vec![ThreadEvent::ItemCompleted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    item: item.clone(),
                }],
            )?;
            Ok(RecordedToolResult {
                item,
                sequence: snapshot.sequence,
            })
        })
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
        self.live_interactions.cancel_turn(
            thread_id,
            &turn_id_for_cancellation,
            InteractionCancelReason::TurnInterrupted,
        );
        self.cancel_turn_execution(thread_id, &turn_id_for_cancellation);
        Ok(result)
    }

    pub fn read_thread(&self, thread_id: &ThreadId) -> Result<ThreadSnapshot, CoreError> {
        self.with_loaded_thread(thread_id, |loaded| Ok(loaded.snapshot.clone()))
    }

    /// Reads every durable Thread known to this authority.
    pub fn list_threads(&self) -> Result<Vec<ThreadSnapshot>, CoreError> {
        self.store
            .list_thread_ids()?
            .into_iter()
            .map(|thread_id| self.read_thread(&thread_id))
            .collect()
    }

    /// Reads the Threads that currently share one Session tree identity.
    pub fn list_session_threads(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<ThreadSnapshot>, CoreError> {
        Ok(self
            .list_threads()?
            .into_iter()
            .filter(|thread| &thread.session_id == session_id)
            .collect())
    }

    /// Interrupts every active Turn in a Session tree and archives each Thread.
    pub fn archive_session_threads(
        &self,
        session_id: &SessionId,
        command_id: &CommandId,
        reason: zeta_protocol::ThreadArchiveReason,
    ) -> Result<Vec<ThreadSnapshot>, CoreError> {
        let threads = self.list_session_threads(session_id)?;
        if threads.is_empty() {
            return Err(CoreError::NotFound(session_id.to_string()));
        }
        for thread in threads {
            loop {
                let snapshot = self.read_thread(&thread.thread_id)?;
                let active_turns = snapshot
                    .turns
                    .iter()
                    .filter(|turn| is_interruptible_turn(turn.status))
                    .map(|turn| turn.turn_id.clone())
                    .collect::<Vec<_>>();
                if active_turns.is_empty() {
                    break;
                }
                for turn_id in active_turns {
                    let interrupt_command = CommandId::new(format!(
                        "thread-archive/{}/{}/{}",
                        command_id, thread.thread_id, turn_id
                    ))
                    .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
                    self.interrupt_turn(
                        &thread.thread_id,
                        InterruptTurnRequest {
                            command_id: interrupt_command,
                            expected_sequence: SequenceExpectation::Any,
                            turn_id,
                        },
                    )?;
                }
            }
            self.archive_thread_with_reason(&thread.thread_id, reason)?;
        }
        self.list_session_threads(session_id)
    }

    pub fn archive_thread(&self, thread_id: &ThreadId) -> Result<ThreadSnapshot, CoreError> {
        self.archive_thread_with_reason(thread_id, zeta_protocol::ThreadArchiveReason::Completed)
    }

    fn archive_thread_with_reason(
        &self,
        thread_id: &ThreadId,
        reason: zeta_protocol::ThreadArchiveReason,
    ) -> Result<ThreadSnapshot, CoreError> {
        self.mutate_thread(thread_id, |snapshot| {
            if snapshot.status == zeta_protocol::ThreadStatus::Active {
                self.record_batch(
                    snapshot,
                    vec![ThreadEvent::ThreadArchived {
                        thread_id: thread_id.clone(),
                        reason,
                    }],
                )?;
            }
            Ok(snapshot.clone())
        })
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
            ) && crate::tool_repetition::project_tool_failures(&snapshot.items, &turn.turn_id)?
                .active()
                .is_some_and(|streak| {
                    streak.count >= crate::tool_repetition::TOOL_REPETITION_FAILURE_THRESHOLD
                })
            {
                recovery_events.push(ThreadEvent::TurnFailed {
                    thread_id: thread_id.clone(),
                    turn_id: turn.turn_id,
                    error: StableTurnError::tool_repetition(),
                });
                continue;
            }
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

    fn load_snapshot_at_sequence(
        &self,
        thread_id: &ThreadId,
        sequence: u64,
    ) -> Result<ThreadSnapshot, CoreError> {
        if sequence == 0 {
            return Err(CoreError::InvalidInput(
                "fork source sequence must be positive".into(),
            ));
        }
        let events = self.store.load(thread_id)?;
        let actual = events.last().map_or(0, |event| event.sequence);
        if actual < sequence {
            return Err(CoreError::ThreadStore(ThreadStoreError::SequenceConflict {
                expected: sequence,
                actual,
            }));
        }
        events
            .iter()
            .take_while(|event| event.sequence <= sequence)
            .try_fold(None, |snapshot, event| {
                reduce_thread_event(snapshot, event).map(Some)
            })?
            .ok_or_else(|| CoreError::NotFound(thread_id.to_string()))
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

    fn bind_thread_worktree(
        &self,
        session_id: SessionId,
        thread_id: ThreadId,
        origin: ThreadOrigin,
    ) -> Result<(), CoreError> {
        self.thread_worktree_binder
            .read()
            .map_err(|_| CoreError::Journal("Thread Worktree binder lock poisoned".into()))?
            .provision(&ThreadWorktreeBindingRequest {
                session_id,
                thread_id,
                origin,
            })
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

fn make_goal(
    thread_id: ThreadId,
    goal_id: String,
    objective: String,
    mut status: ThreadGoalStatus,
    token_budget: Option<u64>,
    tokens_used: u64,
) -> Result<ThreadGoal, CoreError> {
    if status.allows_usage_accounting() && token_budget.is_some_and(|budget| tokens_used >= budget)
    {
        status = ThreadGoalStatus::BudgetLimited;
    }
    let goal = ThreadGoal {
        thread_id,
        goal_id,
        objective,
        status,
        token_budget,
        tokens_used,
    };
    goal.validate().map_err(CoreError::InvalidInput)?;
    Ok(goal)
}

fn skill_is_within_ceiling(
    ceiling: Option<&[FrozenSkillActivation]>,
    activation: &FrozenSkillActivation,
) -> bool {
    ceiling.is_none_or(|ceiling| {
        ceiling.iter().any(|allowed| {
            allowed.id == activation.id
                && allowed.content_digest == activation.content_digest
                && allowed.catalog_generation == activation.catalog_generation
        })
    })
}

pub(crate) fn replay_host_activations(
    recorded: Option<&[FrozenSkillActivation]>,
    merged: &[FrozenSkillActivation],
) -> Vec<FrozenSkillActivation> {
    recorded
        .map(<[FrozenSkillActivation]>::to_vec)
        .unwrap_or_else(|| {
            merged
                .iter()
                .filter(|activation| activation.reason == SkillActivationReason::Automatic)
                .cloned()
                .collect()
        })
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

fn append_tool_repetition_reminder(text: &mut String, content: &mut Option<Vec<ContentPart>>) {
    if !text.trim().is_empty() {
        text.push_str("\n\n");
    }
    text.push_str(crate::tool_repetition::TOOL_REPETITION_REMINDER);
    if let Some(content) = content {
        content.push(ContentPart::Text(
            crate::tool_repetition::TOOL_REPETITION_REMINDER.into(),
        ));
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

fn validate_thread_title(command_id: &CommandId, title: &str) -> Result<(), CoreError> {
    validate_command_id(command_id)?;
    if title.trim().is_empty() {
        return Err(CoreError::InvalidInput(
            "Thread title must be non-empty".into(),
        ));
    }
    Ok(())
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

fn command_thread_id(prefix: &str, command_id: &CommandId) -> Result<ThreadId, CoreError> {
    ThreadId::new(format!("{prefix}:{}", command_id.as_str()))
        .map_err(|error| CoreError::InvalidInput(error.to_string()))
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

fn is_terminal_turn_status(status: TurnStatus) -> bool {
    matches!(
        status,
        TurnStatus::Completed | TurnStatus::Failed | TurnStatus::Interrupted
    )
}

fn fork_snapshot_turns(turns: Vec<zeta_protocol::Turn>) -> Vec<zeta_protocol::Turn> {
    let mut imported = Vec::new();
    for mut turn in turns {
        let terminal = is_terminal_turn_status(turn.status);
        if !terminal {
            turn.status = TurnStatus::Interrupted;
            turn.pending_interaction = None;
            turn.error = None;
        }
        turn.usage = zeta_protocol::ModelUsageSummary::default();
        turn.context_usage = None;
        retain_complete_tool_exchanges(&mut turn.items);
        imported.push(turn);
        if !terminal {
            break;
        }
    }
    imported
}

fn retain_complete_tool_exchanges(items: &mut Vec<ThreadItem>) {
    let completed_calls = items
        .iter()
        .filter_map(|item| match item {
            ThreadItem::ToolResult { tool_call_id, .. } => Some(tool_call_id.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    items.retain(|item| {
        !matches!(
            item,
            ThreadItem::ToolCall { tool_call_id, .. }
                if !completed_calls.contains(tool_call_id)
        )
    });
}

fn inherited_fork_checkpoint(
    source: &ThreadSnapshot,
    imported_turns: &[zeta_protocol::Turn],
) -> Result<Option<zeta_protocol::ContextCheckpoint>, CoreError> {
    let Some(checkpoint) = source.context_checkpoints.last().cloned() else {
        return Ok(None);
    };
    let imported_items = imported_turns
        .iter()
        .flat_map(|turn| turn.items.iter())
        .map(ThreadItem::item_id)
        .collect::<Vec<_>>();
    if checkpoint.referenced_items.len() > imported_items.len()
        || checkpoint
            .referenced_items
            .iter()
            .zip(imported_items)
            .any(|(referenced, imported)| referenced != imported)
    {
        return Err(CoreError::Journal(
            "fork source checkpoint is not a prefix of the imported history".into(),
        ));
    }
    Ok(Some(checkpoint))
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
