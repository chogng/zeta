use super::policy_feedback::{denied_feedback, safer_action_feedback};
use crate::policy_service::durable_sandbox_escalation_approval_request;
use crate::thread_controller::{RecordToolExecutionEscalation, RecordToolExecutionStart};
use crate::{
    AutoReviewedToolGrant, CoreError, PermissionBypassToolGrant, PolicyService,
    RecordToolResultRequest, RequestTurnInteraction, SandboxDenialOutput, ThreadController,
    ThreadUpdateSink, ToolAuthorization, ToolCallOutput, ToolExecutionFacts, ToolExecutionOutput,
    ToolOutputSink, ToolReplaySafety, ToolService,
};
use zeta_async_utils::CancellationToken;
use zeta_policy::{ActionReviewRequest, ExecutionDecision, SandboxDenialEvidence};
use zeta_protocol::{
    AgentRequest, ApprovalMode, ItemId, StreamCursor, StreamInstanceId, ThreadId, ThreadUpdate,
    ThreadUpdateEnvelope, ToolCall, ToolCallId, ToolExecutionAuthority,
    ToolExecutionAuthority::Sandboxed, ToolOutputStream, TurnId,
};

const MAX_DENIAL_REASON_CHARS: usize = 500;
const MAX_DENIAL_OUTPUT_CHARS: usize = 2_000;

pub(super) enum ToolExecutionCompletion {
    Complete,
    PolicyRejected,
    WaitingForApproval,
}

pub(super) struct ToolExecutionContext<'a> {
    thread_id: &'a ThreadId,
    turn_id: &'a TurnId,
    item_id: &'a ItemId,
    frozen_policy_revision: &'a str,
    approval_mode: ApprovalMode,
    cancellation: &'a CancellationToken,
}

impl<'a> ToolExecutionContext<'a> {
    pub(super) fn new(
        thread_id: &'a ThreadId,
        turn_id: &'a TurnId,
        item_id: &'a ItemId,
        frozen_policy_revision: &'a str,
        approval_mode: ApprovalMode,
        cancellation: &'a CancellationToken,
    ) -> Self {
        Self {
            thread_id,
            turn_id,
            item_id,
            frozen_policy_revision,
            approval_mode,
            cancellation,
        }
    }

    pub(super) fn thread_id(&self) -> &ThreadId {
        self.thread_id
    }

    pub(super) fn turn_id(&self) -> &TurnId {
        self.turn_id
    }

    pub(super) fn item_id(&self) -> &ItemId {
        self.item_id
    }
}

enum ToolAttempt {
    Commit {
        output: ToolCallOutput,
        completion: ToolExecutionCompletion,
    },
    WaitingForApproval,
}

pub(super) struct ToolExecutionOrchestrator<'a> {
    threads: &'a ThreadController,
    tools: &'a dyn ToolService,
    policy: &'a dyn PolicyService,
    updates: &'a dyn ThreadUpdateSink,
}

impl<'a> ToolExecutionOrchestrator<'a> {
    pub(super) fn new(
        threads: &'a ThreadController,
        tools: &'a dyn ToolService,
        policy: &'a dyn PolicyService,
        updates: &'a dyn ThreadUpdateSink,
    ) -> Self {
        Self {
            threads,
            tools,
            policy,
            updates,
        }
    }

    pub(super) fn execute(
        &self,
        context: &ToolExecutionContext<'_>,
        call: ToolCall,
        reviewed: &ActionReviewRequest,
        authorization: ToolAuthorization,
    ) -> Result<ToolExecutionCompletion, CoreError> {
        self.threads.record_tool_execution_started(
            context.thread_id,
            context.turn_id,
            RecordToolExecutionStart {
                tool_call_id: call.id.clone(),
                action_digest: reviewed.action().digest().as_str().to_owned(),
                policy_revision: reviewed.policy_revision().as_str().to_owned(),
                authority: execution_authority(&authorization),
            },
        )?;

        let (output, completion) =
            match self.execute_initial_attempt(context, &call, reviewed, authorization) {
                Ok(ToolAttempt::WaitingForApproval) => {
                    return Ok(ToolExecutionCompletion::WaitingForApproval);
                }
                Ok(ToolAttempt::Commit { output, completion }) => (output, completion),
                Err(error @ CoreError::Cancelled(_)) => return Err(error),
                Err(error) => (
                    ToolCallOutput::Failure(format!(
                        "tool execution outcome is unknown after execution started: {error}"
                    )),
                    ToolExecutionCompletion::Complete,
                ),
            };
        self.threads.record_tool_result(
            context.thread_id,
            context.turn_id,
            RecordToolResultRequest {
                tool_call_id: call.id,
                output,
            },
        )?;
        Ok(completion)
    }

    pub(super) fn start_execution_interaction(
        &self,
        context: &ToolExecutionContext<'_>,
        call: &ToolCall,
        reviewed: &ActionReviewRequest,
        authorization: &ToolAuthorization,
    ) -> Result<(), CoreError> {
        self.threads.record_tool_execution_started(
            context.thread_id,
            context.turn_id,
            RecordToolExecutionStart {
                tool_call_id: call.id.clone(),
                action_digest: reviewed.action().digest().as_str().to_owned(),
                policy_revision: reviewed.policy_revision().as_str().to_owned(),
                authority: execution_authority(authorization),
            },
        )?;
        Ok(())
    }

    pub(super) fn complete_execution_interaction(
        &self,
        context: &ToolExecutionContext<'_>,
        call: ToolCall,
        output: ToolExecutionOutput,
    ) -> Result<ToolExecutionCompletion, CoreError> {
        let output = match output {
            ToolExecutionOutput::Success(text) => ToolCallOutput::Success(text),
            ToolExecutionOutput::Failure(text) => ToolCallOutput::Failure(text),
            ToolExecutionOutput::OutcomeUnknown(reason) => ToolCallOutput::Failure(format!(
                "dynamic tool execution outcome is unknown: {reason}"
            )),
            ToolExecutionOutput::SandboxDenied(denial) => ToolCallOutput::Failure(format!(
                "dynamic tool interaction returned an invalid sandbox denial: {}",
                denial.reason()
            )),
        };
        self.threads.record_tool_result(
            context.thread_id,
            context.turn_id,
            RecordToolResultRequest {
                tool_call_id: call.id,
                output,
            },
        )?;
        Ok(ToolExecutionCompletion::Complete)
    }

    pub(super) fn execute_approved_escalation(
        &self,
        context: &ToolExecutionContext<'_>,
        call: ToolCall,
        reviewed: &ActionReviewRequest,
        denial: SandboxDenialOutput,
        authorization: ToolAuthorization,
    ) -> Result<ToolExecutionCompletion, CoreError> {
        context
            .cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        self.threads.record_tool_execution_escalated(
            context.thread_id,
            context.turn_id,
            RecordToolExecutionEscalation {
                tool_call_id: call.id.clone(),
                action_digest: reviewed.action().digest().as_str().to_owned(),
                policy_revision: reviewed.policy_revision().as_str().to_owned(),
                denial,
                authority: execution_authority(&authorization),
            },
        )?;
        let output = match self.execute_service(context, &call, &authorization) {
            Ok(ToolExecutionOutput::Success(text)) => ToolCallOutput::Success(text),
            Ok(ToolExecutionOutput::Failure(text)) => ToolCallOutput::Failure(text),
            Ok(ToolExecutionOutput::OutcomeUnknown(reason)) => ToolCallOutput::Failure(format!(
                "approved outside-sandbox retry outcome is unknown: {reason}"
            )),
            Ok(ToolExecutionOutput::SandboxDenied(denial)) => ToolCallOutput::Failure(format!(
                "tool service reported a sandbox denial during an approved outside-sandbox \
                 retry: {}",
                denial.reason()
            )),
            Err(error @ CoreError::Cancelled(_)) => return Err(error),
            Err(error) => ToolCallOutput::Failure(format!(
                "approved outside-sandbox retry outcome is unknown after execution started: \
                 {error}"
            )),
        };
        self.threads.record_tool_result(
            context.thread_id,
            context.turn_id,
            RecordToolResultRequest {
                tool_call_id: call.id,
                output,
            },
        )?;
        Ok(ToolExecutionCompletion::Complete)
    }

    fn execute_initial_attempt(
        &self,
        context: &ToolExecutionContext<'_>,
        call: &ToolCall,
        reviewed: &ActionReviewRequest,
        authorization: ToolAuthorization,
    ) -> Result<ToolAttempt, CoreError> {
        let result = self.execute_service(context, call, &authorization);
        match result {
            Ok(ToolExecutionOutput::Success(text)) => Ok(ToolAttempt::Commit {
                output: ToolCallOutput::Success(text),
                completion: ToolExecutionCompletion::Complete,
            }),
            Ok(ToolExecutionOutput::Failure(text)) => Ok(ToolAttempt::Commit {
                output: ToolCallOutput::Failure(text),
                completion: ToolExecutionCompletion::Complete,
            }),
            Ok(ToolExecutionOutput::OutcomeUnknown(reason)) => Ok(ToolAttempt::Commit {
                output: ToolCallOutput::Failure(format!(
                    "tool execution outcome is unknown: {reason}"
                )),
                completion: ToolExecutionCompletion::Complete,
            }),
            Ok(ToolExecutionOutput::SandboxDenied(denial)) => {
                if !matches!(authorization, ToolAuthorization::Sandboxed(_)) {
                    return Ok(ToolAttempt::Commit {
                        output: ToolCallOutput::Failure(format!(
                            "tool service reported a sandbox denial for an execution that was not \
                             sandboxed: {}",
                            denial.reason()
                        )),
                        completion: ToolExecutionCompletion::Complete,
                    });
                }
                if denial.replay_safety() == ToolReplaySafety::MayHaveSideEffects {
                    return Ok(ToolAttempt::Commit {
                        output: ToolCallOutput::Failure(format!(
                            "sandbox denied the action, but the attempt may have produced side \
                             effects, so the exact call was not retried; reason: {}; output: {}",
                            denial.reason(),
                            denial.output().aggregated_output()
                        )),
                        completion: ToolExecutionCompletion::Complete,
                    });
                }
                self.review_denial_and_retry(context, call, reviewed, denial)
            }
            Err(error) => Err(error),
        }
    }

    fn execute_service(
        &self,
        context: &ToolExecutionContext<'_>,
        call: &ToolCall,
        authorization: &ToolAuthorization,
    ) -> Result<ToolExecutionOutput, CoreError> {
        let snapshot = self.threads.read_thread(context.thread_id)?;
        let facts = ToolExecutionFacts::for_turn(
            &snapshot,
            context.turn_id,
            self.tools
                .definitions()
                .into_iter()
                .map(|definition| definition.name),
        )?;
        let mut stream = ToolUpdateStream {
            updates: self.updates,
            session_id: snapshot.session_id,
            thread_id: context.thread_id.clone(),
            turn_id: context.turn_id.clone(),
            tool_call_id: call.id.clone(),
            durable_sequence: snapshot.sequence,
            stream_instance_id: self.threads.next_stream_instance_id(),
            next_sequence: 0,
        };
        self.tools.execute_streaming_with_facts(
            call,
            authorization,
            context.cancellation,
            &facts,
            &mut stream,
        )
    }

    fn review_denial_and_retry(
        &self,
        context: &ToolExecutionContext<'_>,
        call: &ToolCall,
        reviewed: &ActionReviewRequest,
        denial: SandboxDenialOutput,
    ) -> Result<ToolAttempt, CoreError> {
        let denial_reason = truncate(denial.reason(), MAX_DENIAL_REASON_CHARS);
        let denial_output = truncate(denial.output().aggregated_output(), MAX_DENIAL_OUTPUT_CHARS);
        let second_review = reviewed
            .clone()
            .after_sandbox_denial(SandboxDenialEvidence::new(
                denial_reason.clone(),
                denial_output,
            ));
        let decision = match self.policy.decide_for_turn_with_approval_mode(
            context.frozen_policy_revision,
            context.approval_mode,
            &second_review,
            context.cancellation,
        ) {
            Ok(decision) => decision,
            Err(error @ CoreError::Cancelled(_)) => return Err(error),
            Err(error) => {
                return Ok(ToolAttempt::Commit {
                    output: ToolCallOutput::Failure(format!(
                        "sandbox denied the action and secondary review failed closed; the exact \
                         call was not retried: {error}"
                    )),
                    completion: ToolExecutionCompletion::Complete,
                });
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
            ExecutionDecision::RunWithPermissionBypass(grant) => {
                if !grant.matches(
                    reviewed.action().digest(),
                    reviewed.action().required_capabilities(),
                    reviewed.policy_revision(),
                ) {
                    return Err(CoreError::Policy(
                        "permission-bypass retry grant is not bound to the prepared action".into(),
                    ));
                }
                ToolAuthorization::PermissionBypassed(PermissionBypassToolGrant::new(
                    call.id.clone(),
                    grant,
                ))
            }
            ExecutionDecision::RunSandboxed(_) => {
                return Ok(ToolAttempt::Commit {
                    output: ToolCallOutput::Failure(
                        "sandbox-denial review did not authorize execution outside the sandbox; \
                         the exact call was not retried"
                            .into(),
                    ),
                    completion: ToolExecutionCompletion::Complete,
                });
            }
            ExecutionDecision::ReviseAction(revision) => {
                return Ok(ToolAttempt::Commit {
                    output: ToolCallOutput::Failure(safer_action_feedback(&revision)),
                    completion: ToolExecutionCompletion::PolicyRejected,
                });
            }
            ExecutionDecision::AskUser(approval) => {
                let request =
                    durable_sandbox_escalation_approval_request(&second_review, &approval, denial)?;
                self.threads.request_turn_interaction(
                    context.thread_id,
                    context.turn_id,
                    RequestTurnInteraction {
                        request_id: self.threads.next_interaction_request_id(),
                        item_id: Some(context.item_id.clone()),
                        request: AgentRequest::Approval { request },
                        deadline: None,
                    },
                )?;
                return Ok(ToolAttempt::WaitingForApproval);
            }
            ExecutionDecision::Block(reason) => {
                let feedback = denied_feedback(&reason);
                let completion = if feedback.is_some() {
                    ToolExecutionCompletion::PolicyRejected
                } else {
                    ToolExecutionCompletion::Complete
                };
                return Ok(ToolAttempt::Commit {
                    output: ToolCallOutput::Failure(feedback.unwrap_or_else(|| {
                        format!(
                            "sandbox denied the action and policy blocked an outside-sandbox \
                             retry: {reason:?}"
                        )
                    })),
                    completion,
                });
            }
        };

        context
            .cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        let authority = execution_authority(&authorization);
        self.threads.record_tool_execution_escalated(
            context.thread_id,
            context.turn_id,
            RecordToolExecutionEscalation {
                tool_call_id: call.id.clone(),
                action_digest: reviewed.action().digest().as_str().to_owned(),
                policy_revision: reviewed.policy_revision().as_str().to_owned(),
                denial,
                authority,
            },
        )?;

        let output = match self.execute_service(context, call, &authorization) {
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
        Ok(ToolAttempt::Commit {
            output,
            completion: ToolExecutionCompletion::Complete,
        })
    }
}

struct ToolUpdateStream<'a> {
    updates: &'a dyn ThreadUpdateSink,
    session_id: zeta_protocol::SessionId,
    thread_id: ThreadId,
    turn_id: TurnId,
    tool_call_id: ToolCallId,
    durable_sequence: u64,
    stream_instance_id: StreamInstanceId,
    next_sequence: u64,
}

impl ToolOutputSink for ToolUpdateStream<'_> {
    fn emit(&mut self, stream: ToolOutputStream, text: String) -> Result<(), CoreError> {
        if text.is_empty() {
            return Ok(());
        }
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.updates.publish(ThreadUpdateEnvelope {
            session_id: self.session_id.clone(),
            thread_id: self.thread_id.clone(),
            durable_sequence: self.durable_sequence,
            stream_cursor: Some(StreamCursor {
                stream_instance_id: self.stream_instance_id.clone(),
                sequence: self.next_sequence,
            }),
            update: ThreadUpdate::ToolOutputDelta {
                turn_id: self.turn_id.clone(),
                tool_call_id: self.tool_call_id.clone(),
                stream,
                text,
            },
        });
        Ok(())
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
        ToolAuthorization::PermissionBypassed(_) => ToolExecutionAuthority::PermissionBypassed,
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
