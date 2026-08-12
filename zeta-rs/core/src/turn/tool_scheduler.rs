use super::policy_feedback::{denied_feedback, rejection_circuit_breaker, safer_action_feedback};
use super::review_context::attach_review_context;
use super::tool_execution::{
    ToolExecutionCompletion, ToolExecutionContext, ToolExecutionOrchestrator,
};
use crate::policy_service::approval_matches_review;
use crate::{
    AutoReviewedToolGrant, CoreError, NoThreadUpdates, OneTimeToolGrant, PolicyService,
    RecordToolResultRequest, RequestTurnInteraction, ThreadController, ThreadSnapshot,
    ThreadUpdateSink, ToolAuthorization, ToolCallOutput, ToolService, durable_approval_request,
};
use std::sync::Arc;
use zeta_async_utils::CancellationToken;
use zeta_policy::ExecutionDecision;
use zeta_protocol::{
    ActionApprovalDecision, AgentRequest, AgentResponse, ItemId, ThreadId, ThreadItem, ToolCall,
    ToolCallId, TurnId,
};

pub(super) enum ToolSchedulingProgress {
    Complete,
    WaitingForApproval,
}

pub(super) struct ToolScheduler {
    threads: Arc<ThreadController>,
    tools: Arc<dyn ToolService>,
    policy: Arc<dyn PolicyService>,
    updates: Arc<dyn ThreadUpdateSink>,
}

impl ToolScheduler {
    pub(super) fn new(
        threads: Arc<ThreadController>,
        tools: Arc<dyn ToolService>,
        policy: Arc<dyn PolicyService>,
    ) -> Self {
        Self {
            threads,
            tools,
            policy,
            updates: Arc::new(NoThreadUpdates),
        }
    }

    pub(super) fn with_thread_updates(mut self, updates: Arc<dyn ThreadUpdateSink>) -> Self {
        self.updates = updates;
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
            let frozen_policy_revision = snapshot
                .turns
                .iter()
                .find(|turn| &turn.turn_id == turn_id)
                .map(|turn| turn.policy_revision.as_str())
                .ok_or_else(|| CoreError::NotFound(turn_id.to_string()))?;
            let execution = ToolExecutionContext::new(
                thread_id,
                turn_id,
                &pending.item_id,
                frozen_policy_revision,
                cancellation,
            );

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
                                self.threads.as_ref(),
                                self.tools.as_ref(),
                                self.policy.as_ref(),
                                self.updates.as_ref(),
                            )
                            .execute_approved_escalation(
                                &execution,
                                pending.call,
                                &reviewed,
                                denial,
                                authorization,
                            )?;
                        } else if snapshot.started_tool_calls.contains(&pending.call.id) {
                            self.record_failure(
                                thread_id,
                                turn_id,
                                pending.call.id,
                                "tool execution outcome is unknown after process interruption; the exact call was not retried",
                            )?;
                        } else if matches!(
                            self.execute(&execution, pending.call, &reviewed, authorization)?,
                            ToolSchedulingProgress::WaitingForApproval
                        ) {
                            return Ok(ToolSchedulingProgress::WaitingForApproval);
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
            match self
                .policy
                .decide_for_turn(frozen_policy_revision, &reviewed, cancellation)?
            {
                ExecutionDecision::RunSandboxed(sandbox) => {
                    if matches!(
                        self.execute(
                            &execution,
                            pending.call,
                            &reviewed,
                            ToolAuthorization::Sandboxed(sandbox),
                        )?,
                        ToolSchedulingProgress::WaitingForApproval
                    ) {
                        return Ok(ToolSchedulingProgress::WaitingForApproval);
                    }
                }
                ExecutionDecision::RunUnsandboxed { grant_id } => {
                    if matches!(
                        self.execute(
                            &execution,
                            pending.call,
                            &reviewed,
                            ToolAuthorization::UnsandboxedGrant { grant_id },
                        )?,
                        ToolSchedulingProgress::WaitingForApproval
                    ) {
                        return Ok(ToolSchedulingProgress::WaitingForApproval);
                    }
                }
                ExecutionDecision::RunAutoReviewed(grant) => {
                    if !grant.matches(
                        reviewed.action().digest(),
                        reviewed.action().required_capabilities(),
                        reviewed.policy_revision(),
                    ) {
                        return Err(CoreError::Policy(
                            "automatic-review grant is not bound to the prepared action".into(),
                        ));
                    }
                    let authorization = ToolAuthorization::AutoReviewed(
                        AutoReviewedToolGrant::new(pending.call.id.clone(), grant),
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
    ) -> Result<zeta_policy::ActionReviewRequest, CoreError> {
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
        reviewed: &zeta_policy::ActionReviewRequest,
        authorization: ToolAuthorization,
    ) -> Result<ToolSchedulingProgress, CoreError> {
        let completion = ToolExecutionOrchestrator::new(
            self.threads.as_ref(),
            self.tools.as_ref(),
            self.policy.as_ref(),
            self.updates.as_ref(),
        )
        .execute(context, call, reviewed, authorization)?;
        match completion {
            ToolExecutionCompletion::Complete => Ok(ToolSchedulingProgress::Complete),
            ToolExecutionCompletion::PolicyRejected => {
                self.enforce_rejection_circuit_breaker(context.thread_id(), context.turn_id())?;
                Ok(ToolSchedulingProgress::Complete)
            }
            ToolExecutionCompletion::WaitingForApproval => {
                Ok(ToolSchedulingProgress::WaitingForApproval)
            }
        }
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

#[cfg(test)]
#[path = "tool_scheduler_tests.rs"]
mod tests;
