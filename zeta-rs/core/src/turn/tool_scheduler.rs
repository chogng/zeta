use super::policy_feedback::{denied_feedback, rejection_circuit_breaker, safer_action_feedback};
use super::review_context::attach_review_context;
use super::tool_execution::{
    ToolExecutionCompletion, ToolExecutionContext, ToolExecutionOrchestrator,
};
use crate::action_policy_service::approval_matches_review;
use crate::{
    ActionPolicyService, AutoReviewedToolGrant, CoreError, ExecPolicyToolGrant, HookEvent,
    HookOutcome, HookService, NoHooks, NoThreadUpdates, OneTimeToolGrant,
    PermissionBypassToolGrant, RecordToolResultRequest, RequestTurnInteraction, ThreadController,
    ThreadSnapshot, ThreadUpdateSink, ToolAuthorization, ToolCallOutput, ToolService,
    durable_approval_request,
};
use std::sync::Arc;
use zeta_action_policy::ExecutionDecision;
use zeta_async_utils::CancellationToken;
use zeta_protocol::{
    ActionApprovalDecision, AgentRequest, AgentResponse, ItemId, ThreadId, ThreadItem, ToolCall,
    ToolCallId, TurnId,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ToolSchedulingProgress {
    Complete,
    WaitingForApproval,
    WaitingForCapability,
}

pub(super) struct ToolScheduler {
    threads: Arc<ThreadController>,
    tools: Arc<dyn ToolService>,
    policy: Arc<dyn ActionPolicyService>,
    hooks: Arc<dyn HookService>,
    updates: Arc<dyn ThreadUpdateSink>,
}

impl ToolScheduler {
    pub(super) fn new(
        threads: Arc<ThreadController>,
        tools: Arc<dyn ToolService>,
        policy: Arc<dyn ActionPolicyService>,
    ) -> Self {
        Self {
            threads,
            tools,
            policy,
            hooks: Arc::new(NoHooks),
            updates: Arc::new(NoThreadUpdates),
        }
    }

    pub(super) fn with_thread_updates(mut self, updates: Arc<dyn ThreadUpdateSink>) -> Self {
        self.updates = updates;
        self
    }

    pub(super) fn with_hooks(mut self, hooks: Arc<dyn HookService>) -> Self {
        self.hooks = hooks;
        self
    }

    pub(super) fn run_pending(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        cancellation: &CancellationToken,
    ) -> Result<ToolSchedulingProgress, CoreError> {
        loop {
            cancellation
                .check()
                .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
            let snapshot = self.threads.read_thread(thread_id)?;
            let Some(pending) = next_pending_call(&snapshot.items, turn_id)? else {
                return Ok(ToolSchedulingProgress::Complete);
            };
            if let Err(error) = self
                .tools
                .validate_call_binding(&pending.call, pending.binding.as_ref())
            {
                let message = if snapshot.started_tool_calls.contains(&pending.call.id) {
                    format!(
                        "tool execution outcome is unknown after its durable binding became unavailable: {error}"
                    )
                } else {
                    format!("durable tool binding is unavailable: {error}")
                };
                self.record_failure(thread_id, turn_id, pending.call.id, message)?;
                continue;
            }
            let frozen_policy_revision = snapshot
                .turns
                .iter()
                .find(|turn| &turn.turn_id == turn_id)
                .map(|turn| turn.policy_revision.as_str())
                .ok_or_else(|| CoreError::NotFound(turn_id.to_string()))?;
            let approval_mode = snapshot
                .turns
                .iter()
                .find(|turn| &turn.turn_id == turn_id)
                .map(|turn| turn.approval_mode)
                .ok_or_else(|| CoreError::NotFound(turn_id.to_string()))?;
            let execution = ToolExecutionContext::new(
                thread_id,
                turn_id,
                &pending.item_id,
                frozen_policy_revision,
                approval_mode,
                cancellation,
            );

            if snapshot.started_tool_calls.contains(&pending.call.id)
                && let Some((request, response)) = resolved_execution_interaction(
                    &snapshot.resolved_interactions,
                    turn_id,
                    &pending.item_id,
                    &pending.call,
                )
            {
                let output = self
                    .tools
                    .resolve_execution_interaction(&pending.call, request, response)?
                    .ok_or_else(|| {
                        CoreError::Execution(format!(
                            "resolved execution interaction is not owned by tool {}",
                            pending.call.name
                        ))
                    })?;
                ToolExecutionOrchestrator::new(
                    Arc::clone(&self.threads),
                    Arc::clone(&self.tools),
                    Arc::clone(&self.policy),
                    Arc::clone(&self.updates),
                )
                .complete_execution_interaction(
                    &execution,
                    pending.call,
                    output,
                )?;
                continue;
            }

            if let Some((request_id, request, decision)) =
                resolved_approval(&snapshot.resolved_interactions, turn_id, &pending.item_id)
            {
                match decision {
                    ActionApprovalDecision::Decline => {
                        self.record_failure(
                            thread_id,
                            turn_id,
                            pending.call.id,
                            "user declined the requested one-time tool authorization",
                        )?;
                    }
                    ActionApprovalDecision::ApproveOnce => {
                        let reviewed = self.prepare_review(
                            &snapshot,
                            turn_id,
                            &pending.item_id,
                            &pending.call,
                        )?;
                        if !approval_matches_review(request, &reviewed) {
                            self.record_failure(
                                thread_id,
                                turn_id,
                                pending.call.id,
                                "approved action no longer matches the exact prepared tool call",
                            )?;
                            continue;
                        }
                        let authorization = ToolAuthorization::ApprovedOnce(OneTimeToolGrant::new(
                            request_id.clone(),
                            pending.call.id.clone(),
                            request.clone(),
                        ));
                        if let Some(denial) = request.sandbox_denial.clone() {
                            if !snapshot.started_tool_calls.contains(&pending.call.id) {
                                self.record_failure(
                                    thread_id,
                                    turn_id,
                                    pending.call.id,
                                    "sandbox escalation approval does not reference a started Tool Call",
                                )?;
                                continue;
                            }
                            if snapshot.escalated_tool_calls.contains(&pending.call.id) {
                                self.record_failure(
                                    thread_id,
                                    turn_id,
                                    pending.call.id,
                                    "approved outside-sandbox retry outcome is unknown after process interruption; the exact call was not retried",
                                )?;
                                continue;
                            }
                            ToolExecutionOrchestrator::new(
                                Arc::clone(&self.threads),
                                Arc::clone(&self.tools),
                                Arc::clone(&self.policy),
                                Arc::clone(&self.updates),
                            )
                            .execute_approved_escalation(
                                &execution,
                                pending.call.clone(),
                                &reviewed,
                                denial,
                                authorization,
                            )?;
                            self.run_after_tool(
                                &execution,
                                &pending.call.id,
                                pending.call.name.to_string(),
                            )?;
                        } else if snapshot.started_tool_calls.contains(&pending.call.id) {
                            self.record_failure(
                                thread_id,
                                turn_id,
                                pending.call.id,
                                "tool execution outcome is unknown after process interruption; the exact call was not retried",
                            )?;
                        } else {
                            let progress =
                                self.execute(&execution, pending.call, &reviewed, authorization)?;
                            if progress != ToolSchedulingProgress::Complete {
                                return Ok(progress);
                            }
                        }
                    }
                }
                continue;
            }

            if snapshot.started_tool_calls.contains(&pending.call.id) {
                self.record_failure(
                    thread_id,
                    turn_id,
                    pending.call.id,
                    "tool execution outcome is unknown after process interruption; the exact call was not retried",
                )?;
                continue;
            }

            let reviewed =
                self.prepare_review(&snapshot, turn_id, &pending.item_id, &pending.call)?;
            match self.policy.decide_for_turn_with_approval_mode(
                frozen_policy_revision,
                approval_mode,
                &reviewed,
                cancellation,
            )? {
                ExecutionDecision::RunSandboxed(sandbox) => {
                    let progress = self.execute(
                        &execution,
                        pending.call,
                        &reviewed,
                        ToolAuthorization::Sandboxed(sandbox),
                    )?;
                    if progress != ToolSchedulingProgress::Complete {
                        return Ok(progress);
                    }
                }
                ExecutionDecision::RunUnsandboxed { grant_id } => {
                    let progress = self.execute(
                        &execution,
                        pending.call,
                        &reviewed,
                        ToolAuthorization::UnsandboxedGrant { grant_id },
                    )?;
                    if progress != ToolSchedulingProgress::Complete {
                        return Ok(progress);
                    }
                }
                ExecutionDecision::RunExecPolicyGranted(grant) => {
                    if !grant.matches(
                        reviewed.action().digest(),
                        reviewed.action().required_capabilities(),
                        reviewed.action_policy_revision(),
                    ) {
                        return Err(CoreError::Policy(
                            "execution-policy grant is not bound to the prepared action".into(),
                        ));
                    }
                    let authorization = ToolAuthorization::ExecPolicyGranted(
                        ExecPolicyToolGrant::new(pending.call.id.clone(), grant),
                    );
                    let progress =
                        self.execute(&execution, pending.call, &reviewed, authorization)?;
                    if progress != ToolSchedulingProgress::Complete {
                        return Ok(progress);
                    }
                }
                ExecutionDecision::RunAutoReviewed(grant) => {
                    if !grant.matches(
                        reviewed.action().digest(),
                        reviewed.action().required_capabilities(),
                        reviewed.action_policy_revision(),
                    ) {
                        return Err(CoreError::Policy(
                            "automatic-review grant is not bound to the prepared action".into(),
                        ));
                    }
                    let authorization = ToolAuthorization::AutoReviewed(
                        AutoReviewedToolGrant::new(pending.call.id.clone(), grant),
                    );
                    let progress =
                        self.execute(&execution, pending.call, &reviewed, authorization)?;
                    if progress != ToolSchedulingProgress::Complete {
                        return Ok(progress);
                    }
                }
                ExecutionDecision::RunWithPermissionBypass(grant) => {
                    if !grant.matches(
                        reviewed.action().digest(),
                        reviewed.action().required_capabilities(),
                        reviewed.action_policy_revision(),
                    ) {
                        return Err(CoreError::Policy(
                            "permission-bypass grant is not bound to the prepared action".into(),
                        ));
                    }
                    let authorization = ToolAuthorization::PermissionBypassed(
                        PermissionBypassToolGrant::new(pending.call.id.clone(), grant),
                    );
                    if matches!(
                        self.execute(&execution, pending.call, &reviewed, authorization)?,
                        ToolSchedulingProgress::WaitingForApproval
                    ) {
                        return Ok(ToolSchedulingProgress::WaitingForApproval);
                    }
                }
                ExecutionDecision::ReviseAction(revision) => {
                    self.record_failure(
                        thread_id,
                        turn_id,
                        pending.call.id,
                        safer_action_feedback(&revision),
                    )?;
                    self.enforce_rejection_circuit_breaker(thread_id, turn_id)?;
                }
                ExecutionDecision::AskUser(approval) => {
                    let request = durable_approval_request(&reviewed, &approval)?;
                    self.threads.request_turn_interaction(
                        thread_id,
                        turn_id,
                        RequestTurnInteraction {
                            request_id: self.threads.next_interaction_request_id(),
                            item_id: Some(pending.item_id),
                            request: AgentRequest::Approval { request },
                            deadline: None,
                        },
                    )?;
                    return Ok(ToolSchedulingProgress::WaitingForApproval);
                }
                ExecutionDecision::Block(reason) => {
                    let feedback = denied_feedback(&reason);
                    self.record_failure(
                        thread_id,
                        turn_id,
                        pending.call.id,
                        feedback.clone().unwrap_or_else(|| {
                            format!("tool execution blocked by policy: {reason:?}")
                        }),
                    )?;
                    if feedback.is_some() {
                        self.enforce_rejection_circuit_breaker(thread_id, turn_id)?;
                    }
                }
            }
        }
    }

    fn prepare_review(
        &self,
        snapshot: &ThreadSnapshot,
        turn_id: &TurnId,
        item_id: &ItemId,
        call: &ToolCall,
    ) -> Result<zeta_action_policy::ActionReviewRequest, CoreError> {
        let request = self.tools.prepare(call)?;
        let evidence = self.tools.review_evidence(call)?;
        Ok(attach_review_context(
            request,
            &snapshot.items,
            turn_id,
            item_id,
            evidence,
        ))
    }

    fn enforce_rejection_circuit_breaker(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
    ) -> Result<(), CoreError> {
        let snapshot = self.threads.read_thread(thread_id)?;
        match rejection_circuit_breaker(&snapshot.items, turn_id) {
            Some(reason) => Err(CoreError::PolicyCircuitBreaker(reason)),
            None => Ok(()),
        }
    }

    fn execute(
        &self,
        context: &ToolExecutionContext<'_>,
        call: ToolCall,
        reviewed: &zeta_action_policy::ActionReviewRequest,
        authorization: ToolAuthorization,
    ) -> Result<ToolSchedulingProgress, CoreError> {
        let tool_name = call.name.to_string();
        let tool_call_id = call.id.clone();
        let orchestrator = ToolExecutionOrchestrator::new(
            Arc::clone(&self.threads),
            Arc::clone(&self.tools),
            Arc::clone(&self.policy),
            Arc::clone(&self.updates),
        );
        if let Some(request) = self.tools.execution_interaction(&call)? {
            if !matches!(request, AgentRequest::DynamicTool { call: ref dynamic } if dynamic.call_id == call.id && dynamic.name == call.name)
            {
                return Err(CoreError::Policy(
                    "tool execution interaction must preserve Tool Call identity".into(),
                ));
            }
            orchestrator.start_execution_interaction(context, &call, reviewed, &authorization)?;
            self.threads.request_turn_interaction(
                context.thread_id(),
                context.turn_id(),
                RequestTurnInteraction {
                    request_id: self.threads.next_interaction_request_id(),
                    item_id: Some(context.item_id().clone()),
                    request,
                    deadline: None,
                },
            )?;
            return Ok(ToolSchedulingProgress::WaitingForCapability);
        }
        self.hooks.run(
            &HookEvent::BeforeTool {
                tool_name: tool_name.clone(),
            },
            context.cancellation(),
        )?;
        let completion = orchestrator.execute(context, call, reviewed, authorization)?;
        match completion {
            ToolExecutionCompletion::Complete => {
                self.run_after_tool(context, &tool_call_id, tool_name)?;
                Ok(ToolSchedulingProgress::Complete)
            }
            ToolExecutionCompletion::PolicyRejected => {
                self.run_after_tool(context, &tool_call_id, tool_name)?;
                self.enforce_rejection_circuit_breaker(context.thread_id(), context.turn_id())?;
                Ok(ToolSchedulingProgress::Complete)
            }
            ToolExecutionCompletion::WaitingForApproval => {
                Ok(ToolSchedulingProgress::WaitingForApproval)
            }
        }
    }

    fn run_after_tool(
        &self,
        context: &ToolExecutionContext<'_>,
        tool_call_id: &ToolCallId,
        tool_name: String,
    ) -> Result<(), CoreError> {
        let outcome = self
            .threads
            .read_thread(context.thread_id())?
            .items
            .iter()
            .rev()
            .find_map(|item| match item {
                zeta_protocol::ThreadItem::ToolResult {
                    tool_call_id: result_id,
                    is_error,
                    ..
                } if result_id == tool_call_id => Some(if *is_error {
                    HookOutcome::Failed
                } else {
                    HookOutcome::Succeeded
                }),
                _ => None,
            })
            .unwrap_or(HookOutcome::Failed);
        self.hooks.run(
            &HookEvent::AfterTool { tool_name, outcome },
            context.cancellation(),
        )
    }

    fn record_failure(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        tool_call_id: ToolCallId,
        message: impl Into<String>,
    ) -> Result<(), CoreError> {
        self.threads.record_tool_result(
            thread_id,
            turn_id,
            RecordToolResultRequest {
                tool_call_id,
                output: ToolCallOutput::Failure(message.into()),
            },
        )?;
        Ok(())
    }
}

struct PendingToolCall {
    item_id: ItemId,
    call: ToolCall,
    binding: Option<zeta_protocol::ToolCallBinding>,
}

fn next_pending_call(
    items: &[ThreadItem],
    turn_id: &TurnId,
) -> Result<Option<PendingToolCall>, CoreError> {
    for item in items {
        let ThreadItem::ToolCall {
            item_id,
            turn_id: item_turn_id,
            tool_call_id,
            name,
            arguments_json,
            binding,
        } = item
        else {
            continue;
        };
        if item_turn_id != turn_id
            || items.iter().any(|candidate| {
                matches!(
                    candidate,
                    ThreadItem::ToolResult {
                        tool_call_id: result_call_id,
                        ..
                    } if result_call_id == tool_call_id
                )
            })
        {
            continue;
        }
        let arguments = serde_json::from_str(arguments_json).map_err(|error| {
            CoreError::Journal(format!(
                "durable Tool Call {} has invalid arguments: {error}",
                tool_call_id
            ))
        })?;
        return Ok(Some(PendingToolCall {
            item_id: item_id.clone(),
            call: ToolCall {
                id: tool_call_id.clone(),
                name: name.clone(),
                arguments,
            },
            binding: binding.clone(),
        }));
    }
    Ok(None)
}

fn resolved_approval<'a>(
    resolved: &'a [crate::ResolvedTurnInteraction],
    turn_id: &TurnId,
    item_id: &ItemId,
) -> Option<(
    &'a zeta_protocol::RequestId,
    &'a zeta_protocol::ActionApprovalRequest,
    ActionApprovalDecision,
)> {
    resolved.iter().rev().find_map(|resolved| {
        if &resolved.turn_id != turn_id || resolved.interaction.item_id.as_ref() != Some(item_id) {
            return None;
        }
        match (&resolved.interaction.request, &resolved.response) {
            (AgentRequest::Approval { request }, AgentResponse::Approval { response }) => {
                Some((&resolved.interaction.request_id, request, response.decision))
            }
            _ => None,
        }
    })
}

fn resolved_execution_interaction<'a>(
    resolved: &'a [crate::ResolvedTurnInteraction],
    turn_id: &TurnId,
    item_id: &ItemId,
    call: &ToolCall,
) -> Option<(&'a AgentRequest, &'a AgentResponse)> {
    resolved.iter().rev().find_map(|resolved| {
        if &resolved.turn_id != turn_id || resolved.interaction.item_id.as_ref() != Some(item_id) {
            return None;
        }
        match (&resolved.interaction.request, &resolved.response) {
            (
                AgentRequest::DynamicTool { call: request },
                response @ AgentResponse::DynamicTool { response: result },
            ) if request.call_id == call.id
                && request.name == call.name
                && result.call_id == call.id =>
            {
                Some((&resolved.interaction.request, response))
            }
            _ => None,
        }
    })
}

#[cfg(test)]
#[path = "tool_scheduler_tests.rs"]
mod tests;
