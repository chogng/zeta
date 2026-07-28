use super::policy_feedback::{denied_feedback, safer_action_feedback};
use crate::thread_controller::{RecordToolExecutionEscalation, RecordToolExecutionStart};
use crate::{
    AutoReviewedToolGrant, CoreError, PolicyService, RecordToolResultRequest, SandboxDenialOutput,
    ThreadController, ToolAuthorization, ToolCallOutput, ToolExecutionOutput, ToolReplaySafety,
    ToolService,
};
use zeta_async_utils::CancellationToken;
use zeta_policy::{ActionReviewRequest, ExecutionDecision, SandboxDenialEvidence};
use zeta_protocol::{
    ThreadId, ToolCall, ToolExecutionAuthority, ToolExecutionAuthority::Sandboxed, TurnId,
};

const MAX_DENIAL_REASON_CHARS: usize = 500;
const MAX_DENIAL_OUTPUT_CHARS: usize = 2_000;

pub(super) enum ToolExecutionCompletion {
    Complete,
    PolicyRejected,
}

pub(super) struct ToolExecutionOrchestrator<'a> {
    threads: &'a ThreadController,
    tools: &'a dyn ToolService,
    policy: &'a dyn PolicyService,
}

impl<'a> ToolExecutionOrchestrator<'a> {
    pub(super) fn new(
        threads: &'a ThreadController,
        tools: &'a dyn ToolService,
        policy: &'a dyn PolicyService,
    ) -> Self {
        Self {
            threads,
            tools,
            policy,
        }
    }

    pub(super) fn execute(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        call: ToolCall,
        reviewed: &ActionReviewRequest,
        authorization: ToolAuthorization,
        cancellation: &CancellationToken,
    ) -> Result<ToolExecutionCompletion, CoreError> {
        self.threads.record_tool_execution_started(
            thread_id,
            turn_id,
            RecordToolExecutionStart {
                tool_call_id: call.id.clone(),
                action_digest: reviewed.action().digest().as_str().to_owned(),
                policy_revision: reviewed.policy_revision().as_str().to_owned(),
                authority: execution_authority(&authorization),
            },
        )?;

        let (output, completion) = match self.execute_initial_attempt(
            thread_id,
            turn_id,
            &call,
            reviewed,
            authorization,
            cancellation,
        ) {
            Ok(result) => result,
            Err(error) => (
                ToolCallOutput::Failure(format!(
                    "tool execution outcome is unknown after execution started: {error}"
                )),
                ToolExecutionCompletion::Complete,
            ),
        };
        self.threads.record_tool_result(
            thread_id,
            turn_id,
            RecordToolResultRequest {
                tool_call_id: call.id,
                output,
            },
        )?;
        Ok(completion)
    }

    fn execute_initial_attempt(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        call: &ToolCall,
        reviewed: &ActionReviewRequest,
        authorization: ToolAuthorization,
        cancellation: &CancellationToken,
    ) -> Result<(ToolCallOutput, ToolExecutionCompletion), CoreError> {
        let result = self.tools.execute(call, &authorization, cancellation);
        match result {
            Ok(ToolExecutionOutput::Success(text)) => Ok((
                ToolCallOutput::Success(text),
                ToolExecutionCompletion::Complete,
            )),
            Ok(ToolExecutionOutput::Failure(text)) => Ok((
                ToolCallOutput::Failure(text),
                ToolExecutionCompletion::Complete,
            )),
            Ok(ToolExecutionOutput::OutcomeUnknown(reason)) => Ok((
                ToolCallOutput::Failure(format!("tool execution outcome is unknown: {reason}")),
                ToolExecutionCompletion::Complete,
            )),
            Ok(ToolExecutionOutput::SandboxDenied(denial)) => {
                if !matches!(authorization, ToolAuthorization::Sandboxed(_)) {
                    return Ok((
                        ToolCallOutput::Failure(format!(
                            "tool service reported a sandbox denial for an execution that was not \
                             sandboxed: {}",
                            denial.reason()
                        )),
                        ToolExecutionCompletion::Complete,
                    ));
                }
                if denial.replay_safety() == ToolReplaySafety::MayHaveSideEffects {
                    return Ok((
                        ToolCallOutput::Failure(format!(
                            "sandbox denied the action, but the attempt may have produced side \
                             effects, so the exact call was not retried; reason: {}; output: {}",
                            denial.reason(),
                            denial.output().aggregated_output()
                        )),
                        ToolExecutionCompletion::Complete,
                    ));
                }
                self.review_denial_and_retry(
                    thread_id,
                    turn_id,
                    call,
                    reviewed,
                    denial,
                    cancellation,
                )
            }
            Err(error) => Err(error),
        }
    }

    fn review_denial_and_retry(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        call: &ToolCall,
        reviewed: &ActionReviewRequest,
        denial: SandboxDenialOutput,
        cancellation: &CancellationToken,
    ) -> Result<(ToolCallOutput, ToolExecutionCompletion), CoreError> {
        let denial_reason = truncate(denial.reason(), MAX_DENIAL_REASON_CHARS);
        let denial_output = truncate(denial.output().aggregated_output(), MAX_DENIAL_OUTPUT_CHARS);
        let second_review = reviewed
            .clone()
            .after_sandbox_denial(SandboxDenialEvidence::new(
                denial_reason.clone(),
                denial_output,
            ));
        let decision = match self.policy.decide(&second_review, cancellation) {
            Ok(decision) => decision,
            Err(error) => {
                return Ok((
                    ToolCallOutput::Failure(format!(
                        "sandbox denied the action and secondary review failed closed; the exact \
                         call was not retried: {error}"
                    )),
                    ToolExecutionCompletion::Complete,
                ));
            }
        };

        let authorization = match decision {
            ExecutionDecision::RunUnsandboxed { grant_id } => {
                ToolAuthorization::UnsandboxedGrant { grant_id }
            }
            ExecutionDecision::RunAutoReviewed(grant) => {
                if !grant.matches(
                    reviewed.action().digest(),
                    reviewed.action().required_capabilities(),
                    reviewed.policy_revision(),
                ) {
                    return Err(CoreError::Policy(
                        "automatic-review retry grant is not bound to the prepared action".into(),
                    ));
                }
                ToolAuthorization::AutoReviewed(AutoReviewedToolGrant::new(call.id.clone(), grant))
            }
            ExecutionDecision::RunSandboxed(_) => {
                return Ok((
                    ToolCallOutput::Failure(
                        "sandbox-denial review did not authorize execution outside the sandbox; \
                         the exact call was not retried"
                            .into(),
                    ),
                    ToolExecutionCompletion::Complete,
                ));
            }
            ExecutionDecision::ReviseAction(revision) => {
                return Ok((
                    ToolCallOutput::Failure(safer_action_feedback(&revision)),
                    ToolExecutionCompletion::PolicyRejected,
                ));
            }
            ExecutionDecision::AskUser(approval) => {
                return Ok((
                    ToolCallOutput::Failure(format!(
                        "sandbox denied the action and review requires user approval before any \
                         outside-sandbox retry; the exact call was not retried: {}",
                        approval.reason()
                    )),
                    ToolExecutionCompletion::Complete,
                ));
            }
            ExecutionDecision::Block(reason) => {
                let feedback = denied_feedback(&reason);
                let completion = if feedback.is_some() {
                    ToolExecutionCompletion::PolicyRejected
                } else {
                    ToolExecutionCompletion::Complete
                };
                return Ok((
                    ToolCallOutput::Failure(feedback.unwrap_or_else(|| {
                        format!(
                            "sandbox denied the action and policy blocked an outside-sandbox \
                             retry: {reason:?}"
                        )
                    })),
                    completion,
                ));
            }
        };

        if let Err(error) = cancellation.check() {
            return Ok((
                ToolCallOutput::Failure(format!(
                    "sandbox denied the action and execution was cancelled before the \
                     outside-sandbox retry: {}",
                    error.reason()
                )),
                ToolExecutionCompletion::Complete,
            ));
        }
        let authority = execution_authority(&authorization);
        self.threads.record_tool_execution_escalated(
            thread_id,
            turn_id,
            RecordToolExecutionEscalation {
                tool_call_id: call.id.clone(),
                action_digest: reviewed.action().digest().as_str().to_owned(),
                policy_revision: reviewed.policy_revision().as_str().to_owned(),
                denial,
                authority,
            },
        )?;

        let output = match self.tools.execute(call, &authorization, cancellation) {
            Ok(ToolExecutionOutput::Success(text)) => ToolCallOutput::Success(text),
            Ok(ToolExecutionOutput::Failure(text)) => ToolCallOutput::Failure(text),
            Ok(ToolExecutionOutput::OutcomeUnknown(reason)) => ToolCallOutput::Failure(format!(
                "outside-sandbox retry outcome is unknown: {reason}"
            )),
            Ok(ToolExecutionOutput::SandboxDenied(denial)) => ToolCallOutput::Failure(format!(
                "tool service reported another sandbox denial after outside-sandbox \
                     authorization: {}",
                denial.reason()
            )),
            Err(error) => return Err(error),
        };
        Ok((output, ToolExecutionCompletion::Complete))
    }
}

fn execution_authority(authorization: &ToolAuthorization) -> ToolExecutionAuthority {
    match authorization {
        ToolAuthorization::Sandboxed(_) => Sandboxed,
        ToolAuthorization::UnsandboxedGrant { grant_id } => {
            ToolExecutionAuthority::UnsandboxedGrant {
                grant_id: grant_id.as_str().to_owned(),
            }
        }
        ToolAuthorization::AutoReviewed(grant) => ToolExecutionAuthority::AutoReviewed {
            assessment_id: grant.policy_grant().assessment_id().as_str().to_owned(),
        },
        ToolAuthorization::ApprovedOnce(grant) => ToolExecutionAuthority::ApprovedOnce {
            request_id: grant.request_id().clone(),
        },
    }
}

fn truncate(value: &str, maximum_chars: usize) -> String {
    if value.chars().count() <= maximum_chars {
        return value.to_owned();
    }
    let mut truncated = value.chars().take(maximum_chars).collect::<String>();
    truncated.push('…');
    truncated
}
