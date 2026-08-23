use crate::CodexApprovalDecision;
use crate::CodexCommandApprovalRequest;
use crate::CodexFileChangeApprovalRequest;
use crate::CodexServerRequestId;
use crate::CodexThreadAccess;
use crate::CodexThreadId;
use crate::CodexTurnDriver;
use crate::CodexTurnEvent;
use crate::CodexTurnId;
use crate::CodexUserInputAnswers;
use crate::CodexUserInputRequest;
use crate::StartCodexThread;
use crate::StartCodexTurn;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::mpsc::SyncSender;
use std::sync::mpsc::sync_channel;
use std::thread;
use std::time::Duration;
use zeta_core::CoreError;
use zeta_core::RequestTurnInteraction;
use zeta_core::ThreadController;
use zeta_core::ThreadExecutionContext;
use zeta_core::ThreadUpdateSink;
use zeta_core::TurnExecutionBackend;
use zeta_protocol::ActionApprovalCapability;
use zeta_protocol::ActionApprovalCapabilityKind;
use zeta_protocol::ActionApprovalDecision;
use zeta_protocol::ActionApprovalRequest;
use zeta_protocol::AgentRequest;
use zeta_protocol::AgentResponse;
use zeta_protocol::CommandId;
use zeta_protocol::InteractionCancelReason;
use zeta_protocol::RequestId;
use zeta_protocol::RequestUserInput;
use zeta_protocol::RequestUserInputResponse;
use zeta_protocol::StableTurnError;
use zeta_protocol::ThreadCommand;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadItem;
use zeta_protocol::TurnExecutionBinding;
use zeta_protocol::TurnId;
use zeta_protocol::TurnStatus;
use zeta_protocol::UserInput;
use zeta_protocol::UserInputOption;
use zeta_protocol::UserInputQuestion;

const ROUTE_CAPACITY: usize = 256;
const ORPHAN_CAPACITY: usize = 256;
const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(100);
const CODEX_BACKEND_ID: &str = "codex-app-server";

mod options;
mod projection;
mod routing;

pub use options::CodexTurnExecutionBackendOptions;
pub use options::CodexTurnWorkspace;
pub use options::CodexTurnWorkspaceSource;
use projection::TurnProjection;
use routing::event_route_key;
use routing::pump_events;

/// Core Turn backend that delegates the complete agent loop to Codex App Server.
///
/// Core still owns durable Thread events and typed interactions. This adapter owns only the
/// process-local mapping to upstream thread/Turn/request IDs. A closed upstream connection fails
/// active local Turns and never replays their unknown external side effects.
#[derive(Clone)]
pub struct CodexTurnExecutionBackend {
    inner: Arc<BackendInner>,
}

struct BackendInner {
    driver: Arc<CodexTurnDriver>,
    threads: Arc<ThreadController>,
    updates: Arc<dyn ThreadUpdateSink>,
    options: CodexTurnExecutionBackendOptions,
    state: Mutex<BackendState>,
    state_changed: Condvar,
}

#[derive(Default)]
struct BackendState {
    remote_threads: BTreeMap<ThreadId, BoundRemoteThread>,
    routes: BTreeMap<RouteKey, SyncSender<CodexTurnEvent>>,
    orphans: VecDeque<CodexTurnEvent>,
    pending_interactions: BTreeMap<RequestId, PendingUpstreamRequest>,
    pending_turns: BTreeSet<LocalTurnKey>,
    active_turns: BTreeMap<LocalTurnKey, RouteKey>,
    runtime_closed: bool,
}

#[derive(Clone)]
struct BoundRemoteThread {
    thread_id: CodexThreadId,
    execution_scope: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct RouteKey {
    thread_id: CodexThreadId,
    turn_id: CodexTurnId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct LocalTurnKey {
    thread_id: ThreadId,
    turn_id: TurnId,
}

struct PendingUpstreamRequest {
    request_id: CodexServerRequestId,
    kind: PendingUpstreamRequestKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PendingUpstreamRequestKind {
    Approval,
    UserInput,
}

impl CodexTurnExecutionBackend {
    pub fn new(
        driver: Arc<CodexTurnDriver>,
        events: Receiver<CodexTurnEvent>,
        threads: Arc<ThreadController>,
        updates: Arc<dyn ThreadUpdateSink>,
        options: CodexTurnExecutionBackendOptions,
    ) -> Result<Self, CoreError> {
        let backend = Self {
            inner: Arc::new(BackendInner {
                driver,
                threads,
                updates,
                options,
                state: Mutex::new(BackendState::default()),
                state_changed: Condvar::new(),
            }),
        };
        let weak = Arc::downgrade(&backend.inner);
        thread::Builder::new()
            .name("zeta-codex-turn-events".into())
            .spawn(move || pump_events(weak, events))
            .map_err(|_| CoreError::Execution("could not start Codex Turn event routing".into()))?;
        Ok(backend)
    }

    fn execute(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        execution: &ThreadExecutionContext,
    ) -> Result<(), CoreError> {
        execution.check_current()?;
        let snapshot = self.inner.threads.read_thread(thread_id)?;
        let turn = snapshot
            .turns
            .iter()
            .find(|turn| &turn.turn_id == turn_id)
            .ok_or_else(|| CoreError::NotFound(turn_id.to_string()))?;
        if turn.status != TurnStatus::Running {
            return Err(CoreError::Execution(
                "Codex backend can execute only a running Turn".into(),
            ));
        }
        let compaction_retention = context_compaction_retention(&snapshot, turn_id);
        let prompt = match &compaction_retention {
            Some(_) => None,
            None => Some(turn_prompt(&snapshot.items, turn_id)?),
        };
        if compaction_retention
            .as_ref()
            .is_some_and(|retention| retention.is_some())
        {
            return Err(CoreError::Execution(
                "Codex subscription context compaction does not support a retention prompt".into(),
            ));
        }
        let workspace = self.inner.options.workspace.current_workspace()?;
        validate_workspace(&workspace)?;
        let remote_thread = self.remote_thread(thread_id, turn.model.as_ref(), &workspace)?;
        execution.check_current()?;
        let remote_turn = match compaction_retention {
            Some(Some(_)) => unreachable!("retention prompt is rejected before remote execution"),
            Some(None) => {
                self.inner
                    .driver
                    .compact_thread(&remote_thread)
                    .map_err(codex_error)?;
                self.await_compaction_started(&remote_thread, execution)?
            }
            None => self
                .inner
                .driver
                .start_turn(
                    &StartCodexTurn::text(
                        remote_thread.clone(),
                        prompt.expect("ordinary Codex Turn has a prompt"),
                    )
                    .map_err(codex_error)?,
                )
                .map_err(codex_error)?,
        };
        let key = RouteKey {
            thread_id: remote_thread,
            turn_id: remote_turn.clone(),
        };
        let events = self.register_route(key.clone())?;
        if let Err(error) = self.activate_local_turn(thread_id, turn_id, key.clone()) {
            self.unregister_route(&key);
            return Err(error);
        }
        let mut projection = TurnProjection::new(
            Arc::clone(&self.inner.threads),
            Arc::clone(&self.inner.updates),
            snapshot.session_id,
            thread_id.clone(),
            turn_id.clone(),
            snapshot.sequence,
        );
        let result = self.consume_events(&key, &remote_turn, events, execution, &mut projection);
        self.unregister_route(&key);
        if result.is_ok() {
            self.persist_remote_thread(thread_id, &key.thread_id, &workspace.execution_scope)?;
        }
        result
    }

    fn consume_events(
        &self,
        key: &RouteKey,
        remote_turn: &CodexTurnId,
        events: Receiver<CodexTurnEvent>,
        execution: &ThreadExecutionContext,
        projection: &mut TurnProjection,
    ) -> Result<(), CoreError> {
        loop {
            if execution.cancellation().is_cancelled() {
                let _ = self.inner.driver.interrupt(&key.thread_id, remote_turn);
                return Err(CoreError::Cancelled("Codex Turn was interrupted".into()));
            }
            let event = match events.recv_timeout(EVENT_POLL_INTERVAL) {
                Ok(event) => event,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(CoreError::Execution(
                        "Codex Turn event route closed before completion".into(),
                    ));
                }
            };
            match event {
                CodexTurnEvent::Started { .. } => {}
                CodexTurnEvent::AgentMessageDelta { delta, .. } => {
                    projection.agent_delta(delta);
                }
                CodexTurnEvent::ReasoningSummaryDelta { delta, .. }
                | CodexTurnEvent::ReasoningDelta { delta, .. } => {
                    projection.reasoning_delta(delta);
                }
                CodexTurnEvent::DiffUpdated { .. } => {}
                CodexTurnEvent::CommandApprovalRequested(request) => {
                    self.request_command_approval(projection, request)?;
                }
                CodexTurnEvent::FileChangeApprovalRequested(request) => {
                    self.request_file_change_approval(projection, request)?;
                }
                CodexTurnEvent::UserInputRequested(request) => {
                    self.request_user_input(projection, request)?;
                }
                CodexTurnEvent::Completed { status, .. } => {
                    return projection.finish(status);
                }
                CodexTurnEvent::ProtocolError { .. } => {
                    return Err(CoreError::Execution(
                        "Codex App Server Turn protocol became unavailable".into(),
                    ));
                }
            }
        }
    }

    fn remote_thread(
        &self,
        local_thread_id: &ThreadId,
        model: Option<&zeta_protocol::ModelRef>,
        workspace: &CodexTurnWorkspace,
    ) -> Result<CodexThreadId, CoreError> {
        if let Some(bound) = self
            .inner
            .state
            .lock()
            .map_err(state_error)?
            .remote_threads
            .get(local_thread_id)
            .cloned()
        {
            if bound.execution_scope != workspace.execution_scope {
                return Err(CoreError::Execution(
                    "Thread is bound to a different Codex workspace authority".into(),
                ));
            }
            return Ok(bound.thread_id);
        }
        let snapshot = self.inner.threads.read_thread(local_thread_id)?;
        if let Some(binding) = snapshot.turn_execution_binding {
            if binding.backend != CODEX_BACKEND_ID {
                return Err(CoreError::Execution(
                    "Thread is bound to a different Turn execution backend".into(),
                ));
            }
            if binding.execution_scope != workspace.execution_scope {
                return Err(CoreError::Execution(
                    "Thread is bound to a different Codex workspace authority".into(),
                ));
            }
            let remote = CodexThreadId::new(binding.remote_thread_id).map_err(codex_error)?;
            let resumed = match self.inner.options.access {
                CodexThreadAccess::ReadOnly => self.inner.driver.resume_read_only_thread(
                    &remote,
                    &workspace.path,
                    model.map(|model| model.model.as_str()),
                ),
                CodexThreadAccess::WorkspaceWrite => {
                    self.inner.driver.resume_workspace_write_thread(
                        &remote,
                        &workspace.path,
                        model.map(|model| model.model.as_str()),
                    )
                }
            }
            .map_err(codex_error)?;
            self.inner
                .state
                .lock()
                .map_err(state_error)?
                .remote_threads
                .insert(
                    local_thread_id.clone(),
                    BoundRemoteThread {
                        thread_id: resumed.clone(),
                        execution_scope: workspace.execution_scope.clone(),
                    },
                );
            return Ok(resumed);
        }
        let request = match self.inner.options.access {
            CodexThreadAccess::ReadOnly => StartCodexThread::read_only(&workspace.path),
            CodexThreadAccess::WorkspaceWrite => StartCodexThread::workspace_write(&workspace.path),
        }
        .map_err(codex_error)?;
        let request = match model {
            Some(model) => request
                .with_model(model.model.as_str())
                .map_err(codex_error)?,
            None => request,
        };
        let remote = self
            .inner
            .driver
            .start_thread(&request)
            .map_err(codex_error)?;
        self.inner
            .state
            .lock()
            .map_err(state_error)?
            .remote_threads
            .insert(
                local_thread_id.clone(),
                BoundRemoteThread {
                    thread_id: remote.clone(),
                    execution_scope: workspace.execution_scope.clone(),
                },
            );
        Ok(remote)
    }

    fn persist_remote_thread(
        &self,
        local_thread_id: &ThreadId,
        remote_thread_id: &CodexThreadId,
        execution_scope: &str,
    ) -> Result<(), CoreError> {
        let before = self.inner.threads.read_thread(local_thread_id)?.sequence;
        self.inner.threads.bind_turn_execution(
            local_thread_id,
            TurnExecutionBinding {
                backend: CODEX_BACKEND_ID.into(),
                remote_thread_id: remote_thread_id.as_str().into(),
                execution_scope: execution_scope.into(),
            },
        )?;
        publish_committed_after(
            self.inner.threads.as_ref(),
            self.inner.updates.as_ref(),
            local_thread_id,
            before,
        );
        Ok(())
    }

    fn request_command_approval(
        &self,
        projection: &mut TurnProjection,
        request: CodexCommandApprovalRequest,
    ) -> Result<(), CoreError> {
        if !request.available_decisions.is_empty()
            && !request
                .available_decisions
                .contains(&CodexApprovalDecision::Accept)
        {
            let _ = self.inner.driver.reject_server_request(&request.request_id);
            return Err(CoreError::Execution(
                "Codex command approval does not offer an approve-once decision".into(),
            ));
        }
        let capability = ActionApprovalCapability {
            kind: ActionApprovalCapabilityKind::ProcessSpawn,
            scope: request
                .cwd
                .as_ref()
                .map(|cwd| format!("{cwd}: {}", request.command))
                .unwrap_or_else(|| request.command.clone()),
        };
        let reason = request
            .reason
            .clone()
            .unwrap_or_else(|| "Codex requested permission to execute a command".into());
        self.request_approval(projection, request.request_id, capability, reason)
    }

    fn request_file_change_approval(
        &self,
        projection: &mut TurnProjection,
        request: CodexFileChangeApprovalRequest,
    ) -> Result<(), CoreError> {
        let capability = ActionApprovalCapability {
            kind: ActionApprovalCapabilityKind::FileWrite,
            scope: request
                .grant_root
                .clone()
                .or_else(|| {
                    self.inner
                        .options
                        .workspace
                        .current_workspace()
                        .ok()
                        .map(|workspace| workspace.path.display().to_string())
                })
                .unwrap_or_else(|| "current Codex workspace".into()),
        };
        let reason = request
            .reason
            .clone()
            .unwrap_or_else(|| "Codex requested permission to change workspace files".into());
        self.request_approval(projection, request.request_id, capability, reason)
    }

    fn request_approval(
        &self,
        projection: &mut TurnProjection,
        upstream_id: CodexServerRequestId,
        capability: ActionApprovalCapability,
        reason: String,
    ) -> Result<(), CoreError> {
        let request_id = self.inner.threads.next_interaction_request_id();
        let action_digest = approval_digest(&capability, &reason);
        let result = self.inner.threads.request_turn_interaction(
            &projection.thread_id,
            &projection.turn_id,
            RequestTurnInteraction {
                request_id: request_id.clone(),
                item_id: None,
                request: AgentRequest::Approval {
                    request: ActionApprovalRequest {
                        action_digest,
                        policy_revision: projection.policy_revision()?,
                        capabilities: vec![capability],
                        reason,
                        sandbox_denial: None,
                    },
                },
                deadline: None,
            },
        );
        let requested = match result {
            Ok(requested) => requested,
            Err(error) => {
                let _ = self.inner.driver.reject_server_request(&upstream_id);
                return Err(error);
            }
        };
        self.inner
            .state
            .lock()
            .map_err(state_error)?
            .pending_interactions
            .insert(
                request_id,
                PendingUpstreamRequest {
                    request_id: upstream_id,
                    kind: PendingUpstreamRequestKind::Approval,
                },
            );
        projection.publish_committed_after(requested.sequence.saturating_sub(1));
        projection.durable_sequence = requested.sequence;
        Ok(())
    }

    fn request_user_input(
        &self,
        projection: &mut TurnProjection,
        request: CodexUserInputRequest,
    ) -> Result<(), CoreError> {
        if request.questions.iter().any(|question| question.is_secret) {
            let _ = self.inner.driver.reject_server_request(&request.request_id);
            return Err(CoreError::Execution(
                "Codex requested secret input that cannot be durably stored".into(),
            ));
        }
        let request_id = self.inner.threads.next_interaction_request_id();
        let questions = request
            .questions
            .into_iter()
            .map(|question| {
                let allow_free_form = question.allows_other || question.options.is_empty();
                UserInputQuestion {
                    id: question.id,
                    header: question.header,
                    question: question.question,
                    options: question
                        .options
                        .into_iter()
                        .map(|option| UserInputOption {
                            label: option.label,
                            description: option.description,
                        })
                        .collect(),
                    allow_free_form,
                }
            })
            .collect();
        let result = self.inner.threads.request_turn_interaction(
            &projection.thread_id,
            &projection.turn_id,
            RequestTurnInteraction {
                request_id: request_id.clone(),
                item_id: None,
                request: AgentRequest::UserInput {
                    request: RequestUserInput { questions },
                },
                deadline: None,
            },
        );
        let requested = match result {
            Ok(requested) => requested,
            Err(error) => {
                let _ = self.inner.driver.reject_server_request(&request.request_id);
                return Err(error);
            }
        };
        self.inner
            .state
            .lock()
            .map_err(state_error)?
            .pending_interactions
            .insert(
                request_id,
                PendingUpstreamRequest {
                    request_id: request.request_id,
                    kind: PendingUpstreamRequestKind::UserInput,
                },
            );
        projection.publish_committed_after(requested.sequence.saturating_sub(1));
        projection.durable_sequence = requested.sequence;
        Ok(())
    }

    fn register_route(&self, key: RouteKey) -> Result<Receiver<CodexTurnEvent>, CoreError> {
        let (sender, receiver) = sync_channel(ROUTE_CAPACITY);
        let mut state = self.inner.state.lock().map_err(state_error)?;
        if state.runtime_closed {
            return Err(CoreError::Execution(
                "Codex Turn event runtime is closed".into(),
            ));
        }
        if state.routes.contains_key(&key) {
            return Err(CoreError::Execution(
                "Codex Turn route is already registered".into(),
            ));
        }
        state.routes.insert(key.clone(), sender.clone());
        let mut retained = VecDeque::with_capacity(state.orphans.len());
        while let Some(event) = state.orphans.pop_front() {
            if event_route_key(&event).as_ref() == Some(&key) {
                let _ = sender.send(event);
            } else {
                retained.push_back(event);
            }
        }
        state.orphans = retained;
        Ok(receiver)
    }

    fn await_compaction_started(
        &self,
        thread_id: &CodexThreadId,
        execution: &ThreadExecutionContext,
    ) -> Result<CodexTurnId, CoreError> {
        let mut state = self.inner.state.lock().map_err(state_error)?;
        loop {
            execution.check_current()?;
            if state.runtime_closed {
                return Err(CoreError::Execution(
                    "Codex context compaction event runtime is closed".into(),
                ));
            }
            if let Some(index) = state.orphans.iter().position(|event| {
                matches!(
                    event,
                    CodexTurnEvent::Started {
                        thread_id: event_thread_id,
                        ..
                    } if event_thread_id == thread_id
                )
            }) {
                let Some(CodexTurnEvent::Started { turn_id, .. }) = state.orphans.remove(index)
                else {
                    unreachable!("matched orphan is a Codex Turn start")
                };
                return Ok(turn_id);
            }
            let (next, _) = self
                .inner
                .state_changed
                .wait_timeout(state, EVENT_POLL_INTERVAL)
                .map_err(state_error)?;
            state = next;
        }
    }

    fn unregister_route(&self, key: &RouteKey) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.routes.remove(key);
        }
    }

    fn register_pending_turn(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
    ) -> Result<(), CoreError> {
        let key = LocalTurnKey {
            thread_id: thread_id.clone(),
            turn_id: turn_id.clone(),
        };
        let mut state = self.inner.state.lock().map_err(state_error)?;
        if state.runtime_closed || !state.pending_turns.insert(key) {
            return Err(CoreError::Execution(
                "Codex Turn execution route is unavailable".into(),
            ));
        }
        self.inner.state_changed.notify_all();
        Ok(())
    }

    fn activate_local_turn(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        route: RouteKey,
    ) -> Result<(), CoreError> {
        let key = LocalTurnKey {
            thread_id: thread_id.clone(),
            turn_id: turn_id.clone(),
        };
        let mut state = self.inner.state.lock().map_err(state_error)?;
        if !state.pending_turns.contains(&key) || state.active_turns.insert(key, route).is_some() {
            return Err(CoreError::Execution(
                "Codex Turn execution route is inconsistent".into(),
            ));
        }
        self.inner.state_changed.notify_all();
        Ok(())
    }

    fn finish_local_turn(&self, thread_id: &ThreadId, turn_id: &TurnId) {
        let key = LocalTurnKey {
            thread_id: thread_id.clone(),
            turn_id: turn_id.clone(),
        };
        if let Ok(mut state) = self.inner.state.lock() {
            state.active_turns.remove(&key);
            state.pending_turns.remove(&key);
            self.inner.state_changed.notify_all();
        }
    }

    fn await_active_route(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
    ) -> Result<RouteKey, CoreError> {
        let key = LocalTurnKey {
            thread_id: thread_id.clone(),
            turn_id: turn_id.clone(),
        };
        let mut state = self.inner.state.lock().map_err(state_error)?;
        loop {
            if let Some(route) = state.active_turns.get(&key) {
                return Ok(route.clone());
            }
            if state.runtime_closed || !state.pending_turns.contains(&key) {
                return Err(CoreError::Execution(
                    "Codex Turn is no longer available for steering".into(),
                ));
            }
            state = self.inner.state_changed.wait(state).map_err(state_error)?;
        }
    }

    fn resume_interaction(&self, thread_id: &ThreadId, turn_id: &TurnId) -> Result<(), CoreError> {
        let snapshot = self.inner.threads.read_thread(thread_id)?;
        let resolved = snapshot
            .resolved_interactions
            .iter()
            .rev()
            .find(|resolved| &resolved.turn_id == turn_id)
            .ok_or_else(|| CoreError::Execution("Codex Turn has no resolved interaction".into()))?;
        let pending = self
            .inner
            .state
            .lock()
            .map_err(state_error)?
            .pending_interactions
            .remove(&resolved.interaction.request_id)
            .ok_or_else(|| {
                CoreError::Execution(
                    "Codex interaction binding is unavailable after runtime recovery".into(),
                )
            })?;
        match (&pending.kind, &resolved.response) {
            (PendingUpstreamRequestKind::Approval, AgentResponse::Approval { response }) => {
                let decision = match response.decision {
                    ActionApprovalDecision::ApproveOnce => CodexApprovalDecision::Accept,
                    ActionApprovalDecision::Decline => CodexApprovalDecision::Decline,
                };
                self.inner
                    .driver
                    .resolve_approval(&pending.request_id, decision)
                    .map_err(codex_error)
            }
            (PendingUpstreamRequestKind::UserInput, AgentResponse::UserInput { response }) => {
                let answers = codex_answers(response)?;
                self.inner
                    .driver
                    .submit_user_input(&pending.request_id, &answers)
                    .map_err(codex_error)
            }
            _ => Err(CoreError::Execution(
                "Codex interaction response kind does not match its upstream request".into(),
            )),
        }
    }
}

impl TurnExecutionBackend for CodexTurnExecutionBackend {
    fn start(&self, thread_id: &ThreadId, turn_id: &TurnId) -> Result<(), CoreError> {
        let snapshot = self.inner.threads.read_thread(thread_id)?;
        let turn = snapshot
            .turns
            .iter()
            .find(|turn| &turn.turn_id == turn_id)
            .ok_or_else(|| CoreError::NotFound(turn_id.to_string()))?;
        if turn.execution_backend_attempt.is_some() {
            self.fail_turn(thread_id, turn_id);
            return Err(CoreError::Execution(
                "Codex Turn has an unknown prior execution outcome and cannot be replayed".into(),
            ));
        }
        let before = snapshot.sequence;
        let sequence = self.inner.threads.record_turn_execution_attempt(
            thread_id,
            turn_id,
            CODEX_BACKEND_ID.into(),
        )?;
        let updates = self.inner.threads.thread_updates_after(thread_id, before)?;
        for update in updates {
            self.inner.updates.publish(update);
        }
        debug_assert_eq!(sequence, before + 1);
        if let Err(error) = self.register_pending_turn(thread_id, turn_id) {
            self.fail_turn(thread_id, turn_id);
            return Err(error);
        }
        let backend = self.clone();
        let queued_thread_id = thread_id.clone();
        let queued_turn_id = turn_id.clone();
        if let Err(error) =
            self.inner
                .threads
                .enqueue_turn_execution(thread_id, turn_id, move |execution| {
                    let result = backend.execute(&queued_thread_id, &queued_turn_id, &execution);
                    backend.finish_local_turn(&queued_thread_id, &queued_turn_id);
                    if let Err(error) = result
                        && !matches!(error, CoreError::Cancelled(_))
                    {
                        backend.fail_turn(&queued_thread_id, &queued_turn_id);
                    }
                })
        {
            self.finish_local_turn(thread_id, turn_id);
            self.fail_turn(thread_id, turn_id);
            return Err(error);
        }
        Ok(())
    }

    fn resume(&self, thread_id: &ThreadId, turn_id: &TurnId) -> Result<(), CoreError> {
        match self.resume_interaction(thread_id, turn_id) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.fail_turn(thread_id, turn_id);
                Err(error)
            }
        }
    }

    fn steer(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        _: &CommandId,
        input: &[UserInput],
    ) -> Result<(), CoreError> {
        let text = input
            .iter()
            .map(|input| match input {
                UserInput::Text { text } => Ok(text.clone()),
                _ => Err(CoreError::InvalidInput(
                    "Codex Turn steering currently accepts text input only".into(),
                )),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let route = self.await_active_route(thread_id, turn_id)?;
        self.inner
            .driver
            .steer_turn(&route.thread_id, &route.turn_id, &text)
            .map_err(codex_error)
    }
}

impl CodexTurnExecutionBackend {
    fn fail_turn(&self, thread_id: &ThreadId, turn_id: &TurnId) {
        let before = self
            .inner
            .threads
            .read_thread(thread_id)
            .map(|snapshot| snapshot.sequence)
            .unwrap_or(0);
        if let Ok(snapshot) = self.inner.threads.read_thread(thread_id)
            && let Some(turn) = snapshot.turns.iter().find(|turn| &turn.turn_id == turn_id)
        {
            if let Some(interaction) = &turn.pending_interaction {
                let _ = self.inner.threads.cancel_turn_interaction(
                    thread_id,
                    zeta_core::CancelTurnInteractionRequest {
                        turn_id: turn_id.clone(),
                        request_id: interaction.request_id.clone(),
                        reason: InteractionCancelReason::OwnerDisconnected,
                    },
                );
            }
            let _ = self.inner.threads.fail_turn(
                thread_id,
                turn_id,
                StableTurnError::model_invocation_failed(),
            );
        }
        publish_committed_after(
            self.inner.threads.as_ref(),
            self.inner.updates.as_ref(),
            thread_id,
            before,
        );
    }
}

fn turn_prompt(items: &[ThreadItem], turn_id: &TurnId) -> Result<String, CoreError> {
    let mut messages = Vec::new();
    for item in items.iter().filter(|item| item.turn_id() == turn_id) {
        match item {
            ThreadItem::UserMessage { text, .. } => messages.push(text.clone()),
            ThreadItem::UserImage { .. } | ThreadItem::UserImageAttachment { .. } => {
                return Err(CoreError::Execution(
                    "Codex Turn backend does not yet support local image inputs".into(),
                ));
            }
            ThreadItem::AgentMessage { .. }
            | ThreadItem::Reasoning { .. }
            | ThreadItem::Plan { .. }
            | ThreadItem::ToolCall { .. }
            | ThreadItem::ToolResult { .. } => {}
        }
    }
    let prompt = messages.join("\n\n");
    if prompt.trim().is_empty() {
        Err(CoreError::InvalidInput(
            "Codex Turn requires a non-empty user message".into(),
        ))
    } else {
        Ok(prompt)
    }
}

fn context_compaction_retention(
    snapshot: &zeta_core::ThreadSnapshot,
    turn_id: &TurnId,
) -> Option<Option<String>> {
    snapshot.commands.iter().find_map(|command| {
        matches!(
            &command.result,
            zeta_core::ThreadCommandResult::TurnAccepted {
                turn_id: command_turn_id,
            } if command_turn_id == turn_id
        )
        .then(|| match &command.receipt.command {
            ThreadCommand::CompactContext {
                retention_prompt, ..
            } => Some(retention_prompt.clone()),
            _ => None,
        })
        .flatten()
    })
}

fn validate_workspace(workspace: &CodexTurnWorkspace) -> Result<(), CoreError> {
    if !workspace.path.is_absolute() || workspace.execution_scope.trim().is_empty() {
        return Err(CoreError::Execution(
            "Codex workspace source returned an invalid authority boundary".into(),
        ));
    }
    Ok(())
}

fn approval_digest(capability: &ActionApprovalCapability, reason: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{:?}", capability.kind));
    hasher.update([0]);
    hasher.update(capability.scope.as_bytes());
    hasher.update([0]);
    hasher.update(reason.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn codex_answers(response: &RequestUserInputResponse) -> Result<CodexUserInputAnswers, CoreError> {
    response
        .answers
        .iter()
        .try_fold(CodexUserInputAnswers::new(), |answers, (id, answer)| {
            answers
                .answer(id.clone(), vec![answer.value.clone()])
                .map_err(codex_error)
        })
}

fn publish_committed_after(
    threads: &ThreadController,
    updates: &dyn ThreadUpdateSink,
    thread_id: &ThreadId,
    sequence: u64,
) {
    if let Ok(committed) = threads.thread_updates_after(thread_id, sequence) {
        for update in committed {
            updates.publish(update);
        }
    }
}

fn codex_error(_: crate::CodexTurnError) -> CoreError {
    CoreError::Execution("Codex App Server Turn operation failed".into())
}

fn state_error<T>(_: std::sync::PoisonError<T>) -> CoreError {
    CoreError::Execution("Codex Turn backend state was unavailable".into())
}
