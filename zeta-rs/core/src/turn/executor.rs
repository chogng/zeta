use super::tool_scheduler::{ToolScheduler, ToolSchedulingProgress};
use crate::action_policy_service::UnavailableActionPolicyService;
use crate::context::ContextMeasurementDisposition;
use crate::context::ContextMeasurementPolicy;
use crate::context::ModelContextCompactionService;
use crate::context::ModelInvocationPreparation;
use crate::thread_controller::CommitContextCheckpointRequest;
use crate::thread_controller::PrepareModelInvocationRequest;
use crate::turn::TurnExecutionBackend;
use crate::{
    ActionPolicyService, CompletedTurn, ContextAssembler, ContextCompactionRequest,
    ContextCompactionService, CoreError, HarnessInstructions, HarnessInstructionsProvider,
    HookService, ModelSelection, ModelService, ModelStreamSink, NoHooks, NoThreadUpdates, NoTools,
    ThreadController, ThreadUpdateSink, ToolService, TurnCompletedHookRequest,
};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zeta_async_utils::{Cancellation, CancellationReason, CancellationToken};
use zeta_context_engine::ContextTokenMeasurementOutcome;
use zeta_protocol::{
    ItemId, ModelResponse, ModelStreamEvent, ResponseItem, StableTurnError, StreamCursor,
    StreamInstanceId, ThreadCommand, ThreadId, ThreadItem, ThreadUpdate, ThreadUpdateEnvelope,
    ToolCall, TurnId, TurnStatus,
};

/// Executes provider-independent model and tool steps for one already-started Turn.
///
/// The executor derives every invocation from the latest durable Thread snapshot. It delegates
/// all mutations to [`ThreadController`], so model and tool I/O never owns the Thread projection
/// lock or writer lease. The loop continues while the model requests follow-up work; it does not
/// impose a process-local model-invocation count that would reset after approval or recovery.
#[derive(Clone)]
pub struct TurnExecutor {
    threads: Arc<ThreadController>,
    model: Arc<dyn ModelService>,
    tools: Arc<dyn ToolService>,
    policy: Arc<dyn ActionPolicyService>,
    compaction: Arc<dyn ContextCompactionService>,
    updates: Arc<dyn ThreadUpdateSink>,
    instructions: Arc<dyn HarnessInstructionsProvider>,
    context_source: Arc<dyn crate::ContextSource>,
    hooks: Arc<dyn HookService>,
    extensions: Arc<zeta_extension_api::ExtensionRegistry>,
}

struct FixedHarnessInstructions {
    snapshot: Arc<HarnessInstructions>,
}

impl HarnessInstructionsProvider for FixedHarnessInstructions {
    fn snapshot(&self) -> Arc<HarnessInstructions> {
        Arc::clone(&self.snapshot)
    }
}

/// Terminal result of one executor run.
pub enum TurnExecutionOutcome {
    Completed(CompletedTurn),
    ShellCompleted { sequence: u64 },
    WaitingForApproval,
    WaitingForCapability,
}

impl TurnExecutor {
    /// Freezes the exact durable binding for a host-created Tool Call.
    pub fn bind_tool_call(
        &self,
        call: &ToolCall,
        caller: zeta_protocol::ToolCallCaller,
    ) -> Result<zeta_protocol::ToolCallBinding, CoreError> {
        self.tools
            .bind_call(call, caller)?
            .ok_or_else(|| CoreError::Execution("tool service did not return a binding".into()))
    }

    /// Creates and durably commits one code-mode nested call through the ordinary scheduler path.
    ///
    /// The code runtime supplies only stable cell/runtime identities and canonical arguments. Core
    /// allocates the nested Tool Call identity, freezes the ordinary binding, and records the call;
    /// approval, execution, cancellation, and uncertain-outcome handling remain unchanged.
    pub fn record_code_mode_nested_call(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        parent_tool_call_id: &zeta_protocol::ToolCallId,
        cell_id: impl Into<String>,
        runtime_call_id: impl Into<String>,
        name: zeta_protocol::ToolName,
        arguments: serde_json::Value,
    ) -> Result<zeta_protocol::ToolCallId, CoreError> {
        let runtime_call_id = runtime_call_id.into();
        let call_id = zeta_protocol::ToolCallId::new(format!(
            "code-{}-{}",
            parent_tool_call_id, runtime_call_id
        ))
        .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
        let call = ToolCall {
            id: call_id.clone(),
            name: name.clone(),
            arguments,
        };
        let binding = self.bind_tool_call(
            &call,
            zeta_protocol::ToolCallCaller::CodeMode {
                parent_tool_call_id: parent_tool_call_id.clone(),
                cell_id: cell_id.into(),
                runtime_call_id,
            },
        )?;
        self.threads.record_tool_call(
            thread_id,
            turn_id,
            crate::RecordToolCallRequest {
                tool_call_id: Some(call_id.clone()),
                name,
                arguments_json: serde_json::to_string(&call.arguments)
                    .map_err(|error| CoreError::Context(error.to_string()))?,
                binding: Some(binding),
            },
        )?;
        Ok(call_id)
    }

    pub fn new(
        threads: Arc<ThreadController>,
        model: Arc<dyn ModelService>,
        tools: Arc<dyn ToolService>,
        policy: Arc<dyn ActionPolicyService>,
    ) -> Self {
        let model: Arc<dyn ModelService> = Arc::new(
            crate::attachment_model_service::AttachmentModelService::new(
                model,
                threads.image_attachments(),
            ),
        );
        let compaction = Arc::new(ModelContextCompactionService::new(model.clone()));
        Self {
            threads,
            model,
            tools,
            policy,
            compaction,
            updates: Arc::new(NoThreadUpdates),
            instructions: Arc::new(FixedHarnessInstructions {
                snapshot: Arc::new(HarnessInstructions::default()),
            }),
            context_source: Arc::new(crate::NoContextSource),
            hooks: Arc::new(NoHooks),
            extensions: Arc::new(zeta_extension_api::ExtensionRegistry::default()),
        }
    }

    pub fn without_tools(threads: Arc<ThreadController>, model: Arc<dyn ModelService>) -> Self {
        Self::new(
            threads,
            model,
            Arc::new(NoTools),
            Arc::new(UnavailableActionPolicyService),
        )
    }

    /// Adds the outer transport sink for durable and transient Thread updates.
    pub fn with_thread_updates(mut self, updates: Arc<dyn ThreadUpdateSink>) -> Self {
        self.updates = updates;
        self
    }

    /// Uses immutable prompt additions captured by the host for this Workspace runtime.
    pub fn with_instructions(mut self, instructions: Arc<HarnessInstructions>) -> Self {
        self.instructions = Arc::new(FixedHarnessInstructions {
            snapshot: instructions,
        });
        self
    }

    /// Resolves Instruction snapshots at model-invocation boundaries.
    pub fn with_instructions_provider(
        mut self,
        instructions: Arc<dyn HarnessInstructionsProvider>,
    ) -> Self {
        self.instructions = instructions;
        self
    }

    /// Installs an optional low-trust evidence source evaluated at the first model invocation.
    pub fn with_context_source(mut self, context_source: Arc<dyn crate::ContextSource>) -> Self {
        self.context_source = context_source;
        self
    }

    /// Overrides checkpoint generation while retaining Core-owned provenance and commit checks.
    pub fn with_context_compaction_service(
        mut self,
        compaction: Arc<dyn ContextCompactionService>,
    ) -> Self {
        self.compaction = compaction;
        self
    }

    /// Installs the host-owned Hook runtime used at Core safe-points.
    pub fn with_hooks(mut self, hooks: Arc<dyn HookService>) -> Self {
        self.hooks = hooks;
        self
    }

    /// Installs the shared agent extension registry used at model-invocation safe points.
    pub fn with_extensions(
        mut self,
        extensions: Arc<zeta_extension_api::ExtensionRegistry>,
    ) -> Self {
        self.extensions = extensions;
        self
    }

    pub fn policy_revision(&self) -> String {
        self.policy.revision()
    }

    /// Enqueues one Turn on its Thread-owned execution mailbox and returns after acceptance.
    ///
    /// The mailbox runs model and tool I/O away from the caller and from the Thread projection
    /// lock. Completion, failure, and cancellation remain durable Core transitions.
    pub fn start(&self, thread_id: &ThreadId, turn_id: &TurnId) -> Result<(), CoreError> {
        let executor = self.clone();
        let queued_thread_id = thread_id.clone();
        let queued_turn_id = turn_id.clone();
        self.threads
            .enqueue_turn_execution(thread_id, turn_id, move |execution| {
                if execution.check_current().is_ok() {
                    let _ = executor.execute(
                        &queued_thread_id,
                        &queued_turn_id,
                        execution.cancellation(),
                    );
                }
            })
    }

    /// Enqueues one already-started model-free Shell Turn.
    pub fn start_shell(&self, thread_id: &ThreadId, turn_id: &TurnId) -> Result<(), CoreError> {
        let executor = self.clone();
        let queued_thread_id = thread_id.clone();
        let queued_turn_id = turn_id.clone();
        self.threads
            .enqueue_turn_execution(thread_id, turn_id, move |execution| {
                if execution.check_current().is_ok() {
                    let _ = executor.execute_shell(
                        &queued_thread_id,
                        &queued_turn_id,
                        execution.cancellation(),
                    );
                }
            })
    }

    /// Resumes a waiting or recovered Turn through the executor matching its durable command.
    pub fn resume(&self, thread_id: &ThreadId, turn_id: &TurnId) -> Result<(), CoreError> {
        if self.is_shell_turn(thread_id, turn_id)? {
            self.start_shell(thread_id, turn_id)
        } else {
            self.start(thread_id, turn_id)
        }
    }

    /// Enqueues every recovered running Turn that owns an unresolved durable Tool Call.
    ///
    /// Hosts should call this only after installing the same tool and policy services used for
    /// normal execution. A call that crossed its durable execution-start boundary is converted
    /// into an unknown-outcome Tool failure and is never replayed.
    pub fn resume_recovered_tool_continuations(&self) -> Result<usize, CoreError> {
        let mut resumed = 0;
        for snapshot in self.threads.list_threads()? {
            for turn in &snapshot.turns {
                if turn.status == TurnStatus::Running
                    && snapshot.has_resumable_tool_continuation(&turn.turn_id)
                {
                    self.resume(&snapshot.thread_id, &turn.turn_id)?;
                    resumed += 1;
                }
            }
        }
        Ok(resumed)
    }

    fn execute_shell(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        cancellation: &CancellationToken,
    ) -> Result<TurnExecutionOutcome, CoreError> {
        let sequence_before_execution = self
            .threads
            .read_thread(thread_id)
            .map(|snapshot| snapshot.sequence)
            .unwrap_or(0);
        let result = match self.execute_shell_steps(thread_id, turn_id, cancellation) {
            Ok(completion) => Ok(completion),
            Err(ExecutionFailure::Cancelled(error)) | Err(ExecutionFailure::Interrupted(error)) => {
                self.threads.interrupt_execution(thread_id, turn_id)?;
                Err(error)
            }
            Err(ExecutionFailure::Failed { error, stable }) => {
                self.threads.fail_turn(thread_id, turn_id, stable)?;
                Err(error)
            }
        };
        self.publish_committed_after(thread_id, sequence_before_execution);
        result
    }

    fn execute_shell_steps(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        cancellation: &CancellationToken,
    ) -> Result<TurnExecutionOutcome, ExecutionFailure> {
        check_cancellation(cancellation)?;
        self.require_running_turn(thread_id, turn_id)?;
        match self
            .tool_scheduler()
            .run_pending(thread_id, turn_id, cancellation)
            .map_err(ExecutionFailure::service)?
        {
            ToolSchedulingProgress::Complete => {}
            ToolSchedulingProgress::WaitingForApproval => {
                return Ok(TurnExecutionOutcome::WaitingForApproval);
            }
            ToolSchedulingProgress::WaitingForCapability => {
                return Ok(TurnExecutionOutcome::WaitingForCapability);
            }
        }
        let sequence = self
            .threads
            .complete_turn_without_agent_message(thread_id, turn_id)
            .map_err(ExecutionFailure::persistence)?;
        let _ = self.hooks.turn_completed(
            &TurnCompletedHookRequest {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
            },
            cancellation,
        );
        Ok(TurnExecutionOutcome::ShellCompleted { sequence })
    }

    fn is_shell_turn(&self, thread_id: &ThreadId, turn_id: &TurnId) -> Result<bool, CoreError> {
        let snapshot = self.threads.read_thread(thread_id)?;
        Ok(snapshot.commands.iter().any(|command| {
            matches!(
                (&command.result, &command.receipt.command),
                (
                    crate::ThreadCommandResult::TurnAccepted {
                        turn_id: command_turn_id,
                    },
                    ThreadCommand::StartShellTurn { .. },
                ) if command_turn_id == turn_id
            )
        }))
    }

    fn execute(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        cancellation: &CancellationToken,
    ) -> Result<TurnExecutionOutcome, CoreError> {
        let sequence_before_execution = self
            .threads
            .read_thread(thread_id)
            .map(|snapshot| snapshot.sequence)
            .unwrap_or(0);
        let result = match self.execute_steps(thread_id, turn_id, cancellation) {
            Ok(completion) => Ok(completion),
            Err(ExecutionFailure::Cancelled(error)) => {
                self.threads.interrupt_execution(thread_id, turn_id)?;
                Err(error)
            }
            Err(ExecutionFailure::Interrupted(error)) => {
                self.threads.interrupt_execution(thread_id, turn_id)?;
                Err(error)
            }
            Err(ExecutionFailure::Failed { error, stable }) => {
                self.threads.fail_turn(thread_id, turn_id, stable)?;
                Err(error)
            }
        };
        self.publish_committed_after(thread_id, sequence_before_execution);
        result
    }

    fn execute_steps(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        cancellation: &CancellationToken,
    ) -> Result<TurnExecutionOutcome, ExecutionFailure> {
        check_cancellation(cancellation)?;
        self.require_running_turn(thread_id, turn_id)?;
        match self
            .tool_scheduler()
            .run_pending(thread_id, turn_id, cancellation)
            .map_err(ExecutionFailure::service)?
        {
            ToolSchedulingProgress::Complete => {}
            ToolSchedulingProgress::WaitingForApproval => {
                return Ok(TurnExecutionOutcome::WaitingForApproval);
            }
            ToolSchedulingProgress::WaitingForCapability => {
                return Ok(TurnExecutionOutcome::WaitingForCapability);
            }
        }
        let mut measurement_policy = ContextMeasurementPolicy::default();
        let mut first_invocation_evidence = None;
        loop {
            check_cancellation(cancellation)?;
            let snapshot = self
                .threads
                .read_thread(thread_id)
                .map_err(ExecutionFailure::model)?;
            let activated = activated_tool_names(self.tools.as_ref(), &snapshot.items, turn_id)
                .map_err(ExecutionFailure::model)?;
            let tool_catalog = self
                .tools
                .model_catalog_snapshot(&activated)
                .map_err(ExecutionFailure::model)?;
            let tools = tool_catalog.definitions().to_vec();
            let turn = snapshot
                .turns
                .iter()
                .find(|turn| &turn.turn_id == turn_id)
                .ok_or_else(|| ExecutionFailure::model(CoreError::NotFound(turn_id.to_string())))?;
            let model = match turn.model.as_ref() {
                Some(model) => ModelSelection::Session(model),
                None => ModelSelection::ConfiguredDefault,
            };
            let configured_budget = self
                .model
                .context_budget(model)
                .map_err(ExecutionFailure::model)?;
            let budget = measurement_policy.adjusted_budget(configured_budget);
            let instructions = self.instructions.snapshot();
            let evidence = if is_first_model_invocation(&snapshot, turn_id) {
                if first_invocation_evidence.is_none() {
                    let query = current_turn_query(&snapshot, turn_id);
                    first_invocation_evidence = Some(match query {
                        Some(query) => match self.context_source.collect(
                            &crate::ContextSourceRequest {
                                session_id: &snapshot.session_id,
                                thread_id,
                                turn_id,
                                query: &query,
                            },
                            cancellation,
                        ) {
                            Ok(evidence) => evidence,
                            Err(CoreError::Cancelled(message)) => {
                                return Err(ExecutionFailure::Cancelled(CoreError::Cancelled(
                                    message,
                                )));
                            }
                            Err(_) => Vec::new(),
                        },
                        None => Vec::new(),
                    });
                }
                first_invocation_evidence.clone().unwrap_or_default()
            } else {
                Vec::new()
            };
            let extension_fragments = self
                .extensions
                .contribute_turn_input(zeta_extension_api::TurnInputContext::new(
                    thread_id,
                    turn_id,
                    &turn.activated_skills,
                ))
                .map_err(|error| ExecutionFailure::model(CoreError::Context(error.to_string())))?;
            let invocation = match self
                .threads
                .prepare_model_invocation(
                    thread_id,
                    PrepareModelInvocationRequest {
                        turn_id,
                        instructions: &instructions,
                        extension_fragments,
                        evidence,
                        tools: tools.clone(),
                        budget,
                    },
                )
                .map_err(ExecutionFailure::model)?
            {
                ModelInvocationPreparation::Ready(invocation) => invocation,
                ModelInvocationPreparation::NeedsCompaction { model, plan } => {
                    let request = ContextCompactionRequest::from_plan(&plan, &model);
                    let result = self
                        .compaction
                        .compact(&request, cancellation)
                        .map_err(ExecutionFailure::model)?;
                    check_cancellation(cancellation)?;
                    self.threads
                        .commit_context_checkpoint(
                            thread_id,
                            CommitContextCheckpointRequest {
                                source_thread_sequence: request.source_thread_sequence(),
                                covered: request.covered(),
                                summary: result.summary().into(),
                                schema_revision: result.schema_revision().into(),
                                prompt_revision: result.prompt_revision().into(),
                                context_policy_revision: result.context_policy_revision().into(),
                                generator_model: request.generator_model().cloned(),
                            },
                        )
                        .map_err(ExecutionFailure::persistence)?;
                    measurement_policy.note_compaction();
                    continue;
                }
            };
            let request = ContextAssembler::assemble(invocation.context())
                .map_err(ExecutionFailure::model)?;
            let model = invocation.model().as_service_selection();
            let estimated_input = invocation.context().budget().total_input();
            let measurement_capability = self
                .model
                .input_token_measurement_capability(model)
                .map_err(ExecutionFailure::service)?;
            let should_measure = measurement_policy
                .should_measure(configured_budget, estimated_input, measurement_capability)
                .map_err(|error| ExecutionFailure::model(CoreError::Context(error.to_string())))?;
            if should_measure {
                match self
                    .model
                    .measure_input(model, &request, cancellation)
                    .map_err(ExecutionFailure::service)?
                {
                    ContextTokenMeasurementOutcome::Unavailable => {}
                    ContextTokenMeasurementOutcome::Measured(measurement) => {
                        let disposition = measurement_policy
                            .assess(configured_budget, estimated_input, measurement)
                            .map_err(|error| {
                                ExecutionFailure::model(CoreError::Context(error.to_string()))
                            })?;
                        if disposition == ContextMeasurementDisposition::Replan {
                            continue;
                        }
                    }
                }
            }
            let mut transient_attempt = 0;
            let mut empty_attempt = false;
            let (response, mut stream) = loop {
                let mut stream = InvocationStream::new(
                    self.threads.clone(),
                    self.updates.clone(),
                    invocation.session_id().clone(),
                    invocation.thread_id().clone(),
                    invocation.turn_id().clone(),
                    invocation.context().source_thread_sequence(),
                    cancellation.clone(),
                );
                match self
                    .model
                    .stream(model, &request, cancellation, &mut stream)
                {
                    Ok(response) => {
                        check_cancellation(cancellation)?;
                        let tool_calls = response.tool_calls().count();
                        let text = final_text(&response, &stream);
                        if tool_calls == 0
                            && text.trim().is_empty()
                            && response_refusal_message(&response).is_none()
                        {
                            if !empty_attempt {
                                empty_attempt = true;
                                continue;
                            }
                            return Err(ExecutionFailure::model(CoreError::Execution(
                                "model returned no final text or Tool Call".into(),
                            )));
                        }
                        break (response, stream);
                    }
                    Err(CoreError::ModelTransient(error)) if transient_attempt < 3 => {
                        wait_for_model_retry(cancellation, transient_attempt, &error)?;
                        transient_attempt += 1;
                    }
                    Err(error) => return Err(ExecutionFailure::service(error)),
                }
            };
            measurement_policy.finish_invocation();

            let tool_calls = response.tool_calls().cloned().collect::<Vec<_>>();
            validate_model_tool_calls(&tool_calls, &request.tools)
                .map_err(ExecutionFailure::model)?;
            self.record_reasoning(thread_id, turn_id, &response, &mut stream)?;
            let text = final_text(&response, &stream);
            if tool_calls.is_empty() {
                let text = response_refusal_message(&response).unwrap_or(text);
                if text.trim().is_empty() {
                    return Err(ExecutionFailure::model(CoreError::Execution(
                        response_failure_message(&response),
                    )));
                }
                let completion = match stream.text_item_id() {
                    Some(item_id) => self
                        .threads
                        .complete_turn_with_agent_message(thread_id, turn_id, item_id, text),
                    None => self.threads.complete_turn(thread_id, turn_id, text),
                };
                let completion = completion
                    .map(TurnExecutionOutcome::Completed)
                    .map_err(ExecutionFailure::persistence)?;
                let _ = self.hooks.turn_completed(
                    &TurnCompletedHookRequest {
                        thread_id: thread_id.clone(),
                        turn_id: turn_id.clone(),
                    },
                    cancellation,
                );
                return Ok(completion);
            }

            if !text.trim().is_empty() {
                match stream.text_item_id() {
                    Some(item_id) => self
                        .threads
                        .record_agent_message_with_id(thread_id, turn_id, item_id, text),
                    None => self.threads.record_agent_message(thread_id, turn_id, text),
                }
                .map_err(ExecutionFailure::persistence)?;
            }
            match self.execute_tools(
                thread_id,
                turn_id,
                &tool_calls,
                &tool_catalog,
                cancellation,
            )? {
                ToolSchedulingProgress::Complete => {}
                ToolSchedulingProgress::WaitingForApproval => {
                    return Ok(TurnExecutionOutcome::WaitingForApproval);
                }
                ToolSchedulingProgress::WaitingForCapability => {
                    return Ok(TurnExecutionOutcome::WaitingForCapability);
                }
            }
        }
    }

    fn require_running_turn(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
    ) -> Result<(), ExecutionFailure> {
        let snapshot = self
            .threads
            .read_thread(thread_id)
            .map_err(ExecutionFailure::model)?;
        let status = snapshot
            .turns
            .iter()
            .find(|turn| &turn.turn_id == turn_id)
            .map(|turn| turn.status)
            .ok_or_else(|| ExecutionFailure::model(CoreError::NotFound(turn_id.to_string())))?;
        if status == TurnStatus::Running {
            Ok(())
        } else {
            Err(ExecutionFailure::model(CoreError::Execution(format!(
                "cannot execute a {status:?} Turn"
            ))))
        }
    }

    fn record_reasoning(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        response: &ModelResponse,
        stream: &mut InvocationStream,
    ) -> Result<(), ExecutionFailure> {
        if let Some((item_id, text)) = stream.take_reasoning() {
            if !text.trim().is_empty() {
                self.threads
                    .record_reasoning_with_id(thread_id, turn_id, item_id, text)
                    .map_err(ExecutionFailure::persistence)?;
            }
            return Ok(());
        }
        for item in &response.output {
            if let ResponseItem::Reasoning(text) = item
                && !text.trim().is_empty()
            {
                self.threads
                    .record_reasoning(thread_id, turn_id, text.clone())
                    .map_err(ExecutionFailure::persistence)?;
            }
        }
        Ok(())
    }

    fn publish_committed_after(&self, thread_id: &ThreadId, sequence: u64) {
        if let Ok(updates) = self.threads.thread_updates_after(thread_id, sequence) {
            for update in updates {
                self.updates.publish(update);
            }
        }
    }

    fn execute_tools(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        calls: &[ToolCall],
        catalog: &crate::ModelToolCatalogSnapshot,
        cancellation: &CancellationToken,
    ) -> Result<ToolSchedulingProgress, ExecutionFailure> {
        for call in calls {
            let caller = zeta_protocol::ToolCallCaller::Direct;
            let binding = match catalog.bind_call(call, caller.clone()) {
                Some(binding) => binding,
                None => self.tools.bind_call(call, caller),
            }
            .map_err(ExecutionFailure::service)?
            .ok_or_else(|| {
                ExecutionFailure::service(CoreError::Execution(format!(
                    "tool service did not freeze a durable binding for {}",
                    call.name
                )))
            })?;
            self.threads
                .record_model_tool_call(thread_id, turn_id, call, binding)
                .map_err(ExecutionFailure::persistence)?;
        }
        self.tool_scheduler()
            .run_pending(thread_id, turn_id, cancellation)
            .map_err(ExecutionFailure::service)
    }

    fn tool_scheduler(&self) -> ToolScheduler {
        ToolScheduler::new(
            self.threads.clone(),
            self.tools.clone(),
            self.policy.clone(),
        )
        .with_thread_updates(self.updates.clone())
        .with_hooks(self.hooks.clone())
    }
}

impl TurnExecutionBackend for TurnExecutor {
    fn start(&self, thread_id: &ThreadId, turn_id: &TurnId) -> Result<(), CoreError> {
        TurnExecutor::start(self, thread_id, turn_id)
    }

    fn resume(&self, thread_id: &ThreadId, turn_id: &TurnId) -> Result<(), CoreError> {
        TurnExecutor::resume(self, thread_id, turn_id)
    }
}

fn is_first_model_invocation(snapshot: &crate::ThreadSnapshot, turn_id: &TurnId) -> bool {
    !snapshot.items.iter().any(|item| {
        item.turn_id() == turn_id
            && matches!(
                item,
                zeta_protocol::ThreadItem::AgentMessage { .. }
                    | zeta_protocol::ThreadItem::ToolCall { .. }
                    | zeta_protocol::ThreadItem::ToolResult { .. }
            )
    })
}

fn current_turn_query(snapshot: &crate::ThreadSnapshot, turn_id: &TurnId) -> Option<String> {
    let query = snapshot
        .items
        .iter()
        .filter_map(|item| match item {
            zeta_protocol::ThreadItem::UserMessage {
                turn_id: item_turn,
                text,
                ..
            } if item_turn == turn_id => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    (!query.trim().is_empty()).then_some(query)
}

fn validate_model_tool_calls(
    calls: &[ToolCall],
    definitions: &[zeta_protocol::ToolDefinition],
) -> Result<(), CoreError> {
    let allowed = definitions
        .iter()
        .map(|definition| &definition.name)
        .collect::<BTreeSet<_>>();
    if let Some(call) = calls.iter().find(|call| !allowed.contains(&call.name)) {
        return Err(CoreError::Policy(format!(
            "model requested a Tool outside the current invocation capability scope: {}",
            call.name
        )));
    }
    Ok(())
}

fn activated_tool_names(
    tools: &dyn ToolService,
    items: &[ThreadItem],
    turn_id: &TurnId,
) -> Result<BTreeSet<zeta_protocol::ToolName>, CoreError> {
    let mut calls = BTreeMap::new();
    for item in items {
        if let ThreadItem::ToolCall {
            turn_id: item_turn_id,
            tool_call_id,
            name,
            arguments_json,
            ..
        } = item
            && item_turn_id == turn_id
        {
            let arguments = serde_json::from_str(arguments_json).map_err(|error| {
                CoreError::Execution(format!(
                    "durable Tool Call arguments could not be reconstructed: {error}"
                ))
            })?;
            calls.insert(
                tool_call_id.clone(),
                ToolCall {
                    id: tool_call_id.clone(),
                    name: name.clone(),
                    arguments,
                },
            );
        }
    }

    let mut activated = BTreeSet::new();
    for item in items {
        if let ThreadItem::ToolResult {
            turn_id: item_turn_id,
            tool_call_id,
            text,
            is_error: false,
            ..
        } = item
            && item_turn_id == turn_id
            && let Some(call) = calls.get(tool_call_id)
        {
            activated.extend(tools.activated_tool_names(call, text)?);
        }
    }
    Ok(activated)
}

struct InvocationStream {
    threads: Arc<ThreadController>,
    updates: Arc<dyn ThreadUpdateSink>,
    session_id: zeta_protocol::SessionId,
    thread_id: ThreadId,
    turn_id: TurnId,
    durable_sequence: u64,
    stream_instance_id: StreamInstanceId,
    next_stream_sequence: u64,
    cancellation: CancellationToken,
    text_item_id: Option<ItemId>,
    reasoning_item_id: Option<ItemId>,
    text: String,
    reasoning: String,
}

impl InvocationStream {
    #[allow(clippy::too_many_arguments)]
    fn new(
        threads: Arc<ThreadController>,
        updates: Arc<dyn ThreadUpdateSink>,
        session_id: zeta_protocol::SessionId,
        thread_id: ThreadId,
        turn_id: TurnId,
        durable_sequence: u64,
        cancellation: CancellationToken,
    ) -> Self {
        let stream_instance_id = threads.next_stream_instance_id();
        Self {
            threads,
            updates,
            session_id,
            thread_id,
            turn_id,
            durable_sequence,
            stream_instance_id,
            next_stream_sequence: 0,
            cancellation,
            text_item_id: None,
            reasoning_item_id: None,
            text: String::new(),
            reasoning: String::new(),
        }
    }

    fn text_item_id(&self) -> Option<ItemId> {
        self.text_item_id.clone()
    }

    fn take_reasoning(&mut self) -> Option<(ItemId, String)> {
        self.reasoning_item_id
            .take()
            .map(|item_id| (item_id, std::mem::take(&mut self.reasoning)))
    }

    fn publish(&mut self, update: ThreadUpdate) {
        self.next_stream_sequence = self.next_stream_sequence.saturating_add(1);
        self.updates.publish(ThreadUpdateEnvelope {
            session_id: self.session_id.clone(),
            thread_id: self.thread_id.clone(),
            durable_sequence: self.durable_sequence,
            stream_cursor: Some(StreamCursor {
                stream_instance_id: self.stream_instance_id.clone(),
                sequence: self.next_stream_sequence,
            }),
            update,
        });
    }

    fn start_text_item(&mut self) -> ItemId {
        if let Some(item_id) = &self.text_item_id {
            return item_id.clone();
        }
        let item_id = self.threads.next_stream_item_id();
        self.publish(ThreadUpdate::ItemStarted {
            turn_id: self.turn_id.clone(),
            item: ThreadItem::AgentMessage {
                item_id: item_id.clone(),
                turn_id: self.turn_id.clone(),
                text: String::new(),
            },
        });
        self.text_item_id = Some(item_id.clone());
        item_id
    }

    fn start_reasoning_item(&mut self) -> ItemId {
        if let Some(item_id) = &self.reasoning_item_id {
            return item_id.clone();
        }
        let item_id = self.threads.next_stream_item_id();
        self.publish(ThreadUpdate::ItemStarted {
            turn_id: self.turn_id.clone(),
            item: ThreadItem::Reasoning {
                item_id: item_id.clone(),
                turn_id: self.turn_id.clone(),
                text: String::new(),
            },
        });
        self.reasoning_item_id = Some(item_id.clone());
        item_id
    }
}

impl ModelStreamSink for InvocationStream {
    fn emit(&mut self, event: ModelStreamEvent) -> Result<(), CoreError> {
        self.cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        match event {
            ModelStreamEvent::TextDelta(text) if !text.is_empty() => {
                let item_id = self.start_text_item();
                self.text.push_str(&text);
                self.publish(ThreadUpdate::ItemDelta {
                    turn_id: self.turn_id.clone(),
                    item_id,
                    delta: zeta_protocol::ItemDelta::AgentMessage { text },
                });
            }
            ModelStreamEvent::ReasoningDelta(text) if !text.is_empty() => {
                let item_id = self.start_reasoning_item();
                self.reasoning.push_str(&text);
                self.publish(ThreadUpdate::ItemDelta {
                    turn_id: self.turn_id.clone(),
                    item_id,
                    delta: zeta_protocol::ItemDelta::Reasoning { text },
                });
            }
            ModelStreamEvent::TextDelta(_) | ModelStreamEvent::ReasoningDelta(_) => {}
        }
        Ok(())
    }
}

fn final_text(response: &ModelResponse, stream: &InvocationStream) -> String {
    let text = response.text();
    if text.is_empty() {
        stream.text.clone()
    } else {
        text
    }
}

enum ExecutionFailure {
    Cancelled(CoreError),
    Interrupted(CoreError),
    Failed {
        error: CoreError,
        stable: StableTurnError,
    },
}

impl ExecutionFailure {
    fn model(error: CoreError) -> Self {
        Self::Failed {
            error,
            stable: StableTurnError::model_invocation_failed(),
        }
    }

    fn service(error: CoreError) -> Self {
        match error {
            CoreError::Cancelled(_) => Self::Cancelled(error),
            CoreError::PolicyCircuitBreaker(_) => Self::Interrupted(error),
            _ => Self::model(error),
        }
    }

    fn persistence(error: CoreError) -> Self {
        Self::Failed {
            error,
            stable: StableTurnError::completion_persistence_failed(),
        }
    }
}

fn check_cancellation(cancellation: &CancellationToken) -> Result<(), ExecutionFailure> {
    cancellation
        .check()
        .map_err(|signal| ExecutionFailure::Cancelled(cancelled_error(&signal)))
}

fn cancelled_error(signal: &Cancellation<CancellationReason>) -> CoreError {
    CoreError::Cancelled(signal.reason().to_string())
}

fn response_failure_message(response: &zeta_protocol::ModelResponse) -> String {
    response
        .output
        .iter()
        .find_map(|item| match item {
            ResponseItem::Refusal(message) => Some(format!("model refused the request: {message}")),
            _ => None,
        })
        .unwrap_or_else(|| "model returned no final text or Tool Call".into())
}

fn response_refusal_message(response: &zeta_protocol::ModelResponse) -> Option<String> {
    response.output.iter().find_map(|item| match item {
        ResponseItem::Refusal(message) => Some(message.clone()),
        _ => None,
    })
}

fn wait_for_model_retry(
    cancellation: &CancellationToken,
    attempt: u32,
    error: &str,
) -> Result<(), ExecutionFailure> {
    let retry_after = error
        .strip_prefix("model API rate limited; retry after ")
        .and_then(|value| value.strip_suffix(" ms"))
        .and_then(|value| value.parse::<u64>().ok())
        .map(|value| value.min(60_000));
    let delay_ms = retry_after
        .unwrap_or_else(|| {
            let base = 1_000_u64.saturating_mul(2_u64.saturating_pow(attempt));
            let jitter = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.subsec_nanos() as u64 % 51)
                .unwrap_or(25);
            base.saturating_mul(75 + jitter) / 100
        })
        .min(30_000);
    let mut remaining = delay_ms;
    while remaining > 0 {
        check_cancellation(cancellation)?;
        let step = remaining.min(100);
        std::thread::sleep(Duration::from_millis(step));
        remaining -= step;
    }
    Ok(())
}

#[cfg(test)]
#[path = "executor_tests.rs"]
mod tests;
