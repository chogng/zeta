use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;

use zeta_action_policy::ActionReviewRequest;
use zeta_action_policy::ReviewEvidence;
use zeta_async_utils::CancellationToken;
use zeta_core::CoreError;
use zeta_core::ToolAuthorization;
use zeta_core::ToolExecutionFacts;
use zeta_core::ToolOutputSink;
use zeta_protocol::ContentPart;
use zeta_protocol::ToolCall;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolExecutionOutput;
use zeta_protocol::ToolOutputStream;
use zeta_protocol::TurnId;
use zeta_tools::DEFAULT_TOOL_OUTPUT_MAX_BYTES;
use zeta_tools::ToolBinding;
use zeta_tools::ToolContent;
use zeta_tools::ToolEnvironmentId;
use zeta_tools::ToolExecutionContext;
use zeta_tools::ToolExecutionOutcome;
use zeta_tools::ToolExecutor;
use zeta_tools::ToolOperationId;
use zeta_tools::ToolOutput;
use zeta_tools::ToolOutputStatus;
use zeta_tools::ToolOutputTruncationPolicy;
use zeta_tools::ToolPayload;
use zeta_tools::ToolRuntimeAuthority;

/// Materializes the security review owned by one executable tool contribution.
///
/// Implementations must resolve every security-relevant field before returning and must not
/// perform the requested side effect. The executor receives authority only after Core reviews the
/// returned action.
pub(crate) trait ToolExecutorReviewer: Send + Sync {
    fn prepare(&self, call: &ToolCall) -> Result<PreparedToolExecution, CoreError>;

    fn evidence(&self, _: &ToolCall) -> Result<Vec<ReviewEvidence>, CoreError> {
        Ok(Vec::new())
    }
}

/// Frozen review and payload produced before Core selects execution authority.
pub(crate) struct PreparedToolExecution {
    review: ActionReviewRequest,
    payload: ToolPayload,
}

impl PreparedToolExecution {
    pub(crate) fn new(review: ActionReviewRequest, payload: ToolPayload) -> Self {
        Self { review, payload }
    }
}

pub(crate) struct ToolExecutorRuntime {
    executor: Arc<dyn ToolExecutor>,
    environment_id: ToolEnvironmentId,
    reviewer: Arc<dyn ToolExecutorReviewer>,
    prepared: Mutex<BTreeMap<ToolCallId, ToolPayload>>,
}

impl ToolExecutorRuntime {
    pub(crate) fn new(
        executor: Arc<dyn ToolExecutor>,
        environment_id: ToolEnvironmentId,
        reviewer: Arc<dyn ToolExecutorReviewer>,
    ) -> Self {
        Self {
            executor,
            environment_id,
            reviewer,
            prepared: Mutex::new(BTreeMap::new()),
        }
    }

    pub(crate) fn executor(&self) -> &dyn ToolExecutor {
        self.executor.as_ref()
    }

    pub(crate) fn prepare(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        let prepared = self.reviewer.prepare(call)?;
        self.prepared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(call.id.clone(), prepared.payload);
        Ok(prepared.review)
    }

    pub(crate) fn evidence(&self, call: &ToolCall) -> Result<Vec<ReviewEvidence>, CoreError> {
        self.reviewer.evidence(call)
    }

    pub(crate) fn execute(
        &self,
        binding: &ToolBinding,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
        facts: &ToolExecutionFacts,
        sink: &mut dyn ToolOutputSink,
    ) -> Result<ToolExecutionOutput, CoreError> {
        let identity = facts.execution_identity().ok_or_else(|| {
            CoreError::Execution(
                "ToolExecutor invocation requires durable Thread/Turn execution facts".into(),
            )
        })?;
        self.execute_for_turn(
            binding,
            call,
            authorization,
            cancellation,
            identity.turn_id(),
            sink,
        )
    }

    fn execute_for_turn(
        &self,
        binding: &ToolBinding,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
        turn_id: &TurnId,
        sink: &mut dyn ToolOutputSink,
    ) -> Result<ToolExecutionOutput, CoreError> {
        let operation_id = ToolOperationId::new(format!("{turn_id}:{}", call.id))
            .map_err(|error| CoreError::Execution(error.to_string()))?;
        let payload = self
            .prepared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&call.id)
            .cloned()
            .ok_or_else(|| {
                CoreError::Execution(format!(
                    "ToolExecutor call {} has no frozen prepared payload",
                    call.id
                ))
            })?;
        let authority = match authorization {
            ToolAuthorization::Sandboxed(policy) => ToolRuntimeAuthority::Sandboxed(*policy),
            ToolAuthorization::UnsandboxedGrant { .. }
            | ToolAuthorization::ExecPolicyGranted(_)
            | ToolAuthorization::AutoReviewed(_)
            | ToolAuthorization::PermissionBypassed(_)
            | ToolAuthorization::ApprovedOnce(_) => ToolRuntimeAuthority::Unrestricted,
        };
        let invocation = zeta_tools::ToolInvocation::new(
            operation_id,
            call.id.clone(),
            turn_id.clone(),
            binding.clone(),
            payload,
            ToolExecutionContext::new(self.environment_id.clone(), cancellation.clone(), authority),
        );
        let outcome = pollster::block_on(self.executor.execute(invocation));
        if !matches!(outcome, ToolExecutionOutcome::SandboxDenied(_)) {
            self.prepared
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&call.id);
        }
        protocol_execution_output(outcome, sink)
    }
}

#[cfg(test)]
#[path = "tool_executor_adapter_tests.rs"]
mod tests;

fn protocol_execution_output(
    outcome: ToolExecutionOutcome,
    sink: &mut dyn ToolOutputSink,
) -> Result<ToolExecutionOutput, CoreError> {
    match outcome {
        ToolExecutionOutcome::Returned(output) => returned_output(output, sink),
        ToolExecutionOutcome::NotStarted(failure) => {
            Ok(ToolExecutionOutput::Failure(failure.message().to_owned()))
        }
        ToolExecutionOutcome::SandboxDenied(denial) => {
            Ok(ToolExecutionOutput::SandboxDenied(denial))
        }
        ToolExecutionOutcome::OutcomeUncertain(uncertain) => Ok(
            ToolExecutionOutput::OutcomeUnknown(uncertain.message().to_owned()),
        ),
    }
}

fn returned_output(
    output: ToolOutput,
    sink: &mut dyn ToolOutputSink,
) -> Result<ToolExecutionOutput, CoreError> {
    returned_output_with_policy(
        output,
        sink,
        ToolOutputTruncationPolicy::Bytes(DEFAULT_TOOL_OUTPUT_MAX_BYTES),
    )
}

fn returned_output_with_policy(
    output: ToolOutput,
    sink: &mut dyn ToolOutputSink,
    policy: ToolOutputTruncationPolicy,
) -> Result<ToolExecutionOutput, CoreError> {
    let output = output.truncate_text(policy);
    let content = output
        .content()
        .iter()
        .map(|content| match content {
            ToolContent::Text(text) => {
                sink.emit(ToolOutputStream::Stdout, text.clone())?;
                Ok(ContentPart::Text(text.clone()))
            }
            ToolContent::Image { url, detail } => Ok(ContentPart::ImageUrl {
                url: url.clone(),
                detail: *detail,
            }),
        })
        .collect::<Result<Vec<_>, CoreError>>()?;
    Ok(match output.status() {
        ToolOutputStatus::Success => ToolExecutionOutput::SuccessContent(content),
        ToolOutputStatus::Error => ToolExecutionOutput::FailureContent(content),
    })
}
