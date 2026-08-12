use super::*;
use crate::{
    CreateThreadRequest, InMemoryThreadStore, ProcessExecutionOutput, ProcessExitStatus,
    ResolveTurnInteractionRequest, SandboxDenialOutput, SequenceExpectation, StartTurnRequest,
    ToolAuthorization, ToolExecutionOutput,
};
use serde_json::json;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use zeta_async_utils::CancellationSource;
use zeta_policy::{
    ActionClassifier, ActionDigest, ActionKind, ActionProvenance, ActionReviewPhase,
    ActionReviewRequest, ActionSource, ApprovalRequest, AssessmentId, Capability, CapabilityKind,
    CapabilitySet, ClassifierAssessment, ClassifierRecommendation, ExecutionDecision, PolicyEngine,
    PolicyRevision, ResolvedAction, ReviewEvidence, ReviewEvidenceKind, ReviewEvidenceTrust,
    ReviewFailurePolicy, RiskLevel, SandboxCompatibility, UserAuthorization,
};
use zeta_protocol::{
    ActionApprovalDecision, ActionApprovalResponse, AgentRequest, AgentResponse, CommandId,
    SessionId, ThreadId, ThreadItem, ToolCallId, ToolDefinition, ToolExecutionAuthority, ToolName,
    TurnStatus, UserInput,
};
use zeta_sandboxing::{FileSystemAccess, NetworkAccess, SandboxPolicy};

#[test]
fn approve_once_resumes_the_exact_tool_call() {
    let fixture = fixture();
    assert!(matches!(
        fixture.scheduler.run_pending(
            &fixture.thread_id,
            &fixture.turn_id,
            &CancellationSource::new().token()
        ),
        Ok(ToolSchedulingProgress::WaitingForApproval)
    ));
    resolve(&fixture, ActionApprovalDecision::ApproveOnce);

    fixture
        .scheduler
        .run_pending(
            &fixture.thread_id,
            &fixture.turn_id,
            &CancellationSource::new().token(),
        )
        .unwrap();

    let authorizations = fixture.tools.authorizations.lock().unwrap();
    assert!(matches!(
        authorizations.as_slice(),
        [ToolAuthorization::ApprovedOnce(grant)]
            if grant.tool_call_id() == &fixture.call_id
    ));
    assert!(
        fixture
            .threads
            .read_thread(&fixture.thread_id)
            .unwrap()
            .items
            .iter()
            .any(|item| matches!(
                item,
                ThreadItem::ToolResult {
                    tool_call_id,
                    text,
                    is_error: false,
                    ..
                } if tool_call_id == &fixture.call_id && text == "executed"
            ))
    );
}

#[test]
fn decline_records_a_tool_failure_without_execution() {
    let fixture = fixture();
    fixture
        .scheduler
        .run_pending(
            &fixture.thread_id,
            &fixture.turn_id,
            &CancellationSource::new().token(),
        )
        .unwrap();
    resolve(&fixture, ActionApprovalDecision::Decline);

    fixture
        .scheduler
        .run_pending(
            &fixture.thread_id,
            &fixture.turn_id,
            &CancellationSource::new().token(),
        )
        .unwrap();

    assert!(fixture.tools.authorizations.lock().unwrap().is_empty());
    assert!(
        fixture
            .threads
            .read_thread(&fixture.thread_id)
            .unwrap()
            .items
            .iter()
            .any(|item| matches!(
                item,
                ThreadItem::ToolResult {
                    tool_call_id,
                    text,
                    is_error: true,
                    ..
                } if tool_call_id == &fixture.call_id && text.contains("declined")
            ))
    );
}

#[test]
fn reviewer_approval_executes_with_bound_authority_and_user_context() {
    let observed = Arc::new(Mutex::new(None));
    let tools = Arc::new(ReviewTool {
        requires_escalation: true,
        evidence: vec![ReviewEvidence::new(
            ReviewEvidenceKind::WorkspaceFile,
            ReviewEvidenceTrust::UntrustedContent,
            "script.py",
            "print('safe')",
        )],
        ..ReviewTool::default()
    });
    let engine = PolicyEngine::new(
        PolicyRevision::new("policy-v1"),
        ContextApprovingClassifier {
            observed: observed.clone(),
        },
        ReviewFailurePolicy::Block,
    );
    let fixture = fixture_with(tools, Arc::new(engine));

    assert!(matches!(
        fixture.scheduler.run_pending(
            &fixture.thread_id,
            &fixture.turn_id,
            &CancellationSource::new().token()
        ),
        Ok(ToolSchedulingProgress::Complete)
    ));

    let authorizations = fixture.tools.authorizations.lock().unwrap();
    assert!(matches!(
        authorizations.as_slice(),
        [ToolAuthorization::AutoReviewed(grant)]
            if grant.tool_call_id() == &fixture.call_id
                && grant.policy_grant().assessment_id()
                    == &AssessmentId::new("auto-assessment")
    ));
    let context = observed.lock().unwrap();
    let context = context.as_ref().unwrap();
    assert_eq!(context.user_intent(), "run");
    assert_eq!(context.evidence().len(), 1);
    assert_eq!(context.evidence()[0].source(), "script.py");
}

#[test]
fn safe_sandbox_denial_is_reviewed_and_retried_once() {
    let observed = Arc::new(Mutex::new(Vec::new()));
    let tools = Arc::new(ReviewTool {
        outputs: Mutex::new(VecDeque::from([
            ToolExecutionOutput::SandboxDenied(SandboxDenialOutput::safe_to_retry(
                "network access denied",
                ProcessExecutionOutput::from_captured_streams(
                    ProcessExitStatus::Code(1),
                    "",
                    "connect: operation not permitted",
                ),
            )),
            ToolExecutionOutput::Success("executed outside sandbox".into()),
        ])),
        ..ReviewTool::default()
    });
    let engine = PolicyEngine::new(
        PolicyRevision::new("policy-v1"),
        DenialApprovingClassifier {
            observed: observed.clone(),
        },
        ReviewFailurePolicy::Block,
    );
    let fixture = fixture_with(tools, Arc::new(engine));

    fixture
        .scheduler
        .run_pending(
            &fixture.thread_id,
            &fixture.turn_id,
            &CancellationSource::new().token(),
        )
        .unwrap();

    let authorizations = fixture.tools.authorizations.lock().unwrap();
    assert!(matches!(
        authorizations.as_slice(),
        [ToolAuthorization::Sandboxed(_), ToolAuthorization::AutoReviewed(grant)]
            if grant.tool_call_id() == &fixture.call_id
    ));
    let phases = observed.lock().unwrap();
    assert!(matches!(
        phases.as_slice(),
        [ActionReviewPhase::SandboxDenial(denial)]
            if denial.reason() == "network access denied"
                && denial.output() == "connect: operation not permitted"
    ));
    let snapshot = fixture.threads.read_thread(&fixture.thread_id).unwrap();
    assert!(snapshot.escalated_tool_calls.contains(&fixture.call_id));
    assert!(snapshot.items.iter().any(|item| matches!(
        item,
        ThreadItem::ToolResult {
            tool_call_id,
            text,
            is_error: false,
            ..
        } if tool_call_id == &fixture.call_id && text == "executed outside sandbox"
    )));
    let recovered = ThreadController::with_store(fixture.store.clone())
        .recover_thread(&fixture.thread_id)
        .unwrap();
    assert!(recovered.escalated_tool_calls.contains(&fixture.call_id));
    assert_eq!(
        recovered
            .tool_execution_starts
            .get(&fixture.call_id)
            .unwrap()
            .action_digest,
        snapshot
            .tool_execution_starts
            .get(&fixture.call_id)
            .unwrap()
            .action_digest
    );
}

#[test]
fn safe_sandbox_denial_waits_for_one_time_approval_and_resumes_after_recovery() {
    let tools = Arc::new(ReviewTool {
        outputs: Mutex::new(VecDeque::from([
            safe_sandbox_denial(),
            ToolExecutionOutput::Success("approved outside sandbox".into()),
        ])),
        ..ReviewTool::default()
    });
    let engine = PolicyEngine::new(
        PolicyRevision::new("policy-v1"),
        DenialAskingClassifier,
        ReviewFailurePolicy::Block,
    );
    let fixture = fixture_with(tools, Arc::new(engine));

    assert!(matches!(
        fixture.scheduler.run_pending(
            &fixture.thread_id,
            &fixture.turn_id,
            &CancellationSource::new().token()
        ),
        Ok(ToolSchedulingProgress::WaitingForApproval)
    ));
    assert!(matches!(
        fixture.tools.authorizations.lock().unwrap().as_slice(),
        [ToolAuthorization::Sandboxed(_)]
    ));
    let snapshot = fixture.threads.read_thread(&fixture.thread_id).unwrap();
    let interaction = snapshot
        .turns
        .last()
        .and_then(|turn| turn.pending_interaction.as_ref())
        .unwrap();
    let AgentRequest::Approval { request } = &interaction.request else {
        panic!("sandbox escalation must request approval");
    };
    let denial = request.sandbox_denial.as_ref().unwrap();
    assert_eq!(denial.reason(), "network access denied");
    assert_eq!(denial.replay_safety(), crate::ToolReplaySafety::SafeToRetry);

    resolve(&fixture, ActionApprovalDecision::ApproveOnce);
    let recovered = Arc::new(ThreadController::with_store(fixture.store.clone()));
    recovered.recover_thread(&fixture.thread_id).unwrap();
    ToolScheduler::new(
        recovered.clone(),
        fixture.tools.clone(),
        Arc::new(AskPolicy),
    )
    .run_pending(
        &fixture.thread_id,
        &fixture.turn_id,
        &CancellationSource::new().token(),
    )
    .unwrap();

    let authorizations = fixture.tools.authorizations.lock().unwrap();
    assert!(matches!(
        authorizations.as_slice(),
        [ToolAuthorization::Sandboxed(_), ToolAuthorization::ApprovedOnce(grant)]
            if grant.tool_call_id() == &fixture.call_id
    ));
    let snapshot = recovered.read_thread(&fixture.thread_id).unwrap();
    assert!(snapshot.escalated_tool_calls.contains(&fixture.call_id));
    assert!(snapshot.items.iter().any(|item| matches!(
        item,
        ThreadItem::ToolResult {
            tool_call_id,
            text,
            is_error: false,
            ..
        } if tool_call_id == &fixture.call_id && text == "approved outside sandbox"
    )));
}

#[test]
fn declining_sandbox_escalation_does_not_retry_the_tool() {
    let tools = Arc::new(ReviewTool {
        outputs: Mutex::new(VecDeque::from([safe_sandbox_denial()])),
        ..ReviewTool::default()
    });
    let engine = PolicyEngine::new(
        PolicyRevision::new("policy-v1"),
        DenialAskingClassifier,
        ReviewFailurePolicy::Block,
    );
    let fixture = fixture_with(tools, Arc::new(engine));

    assert!(matches!(
        fixture.scheduler.run_pending(
            &fixture.thread_id,
            &fixture.turn_id,
            &CancellationSource::new().token()
        ),
        Ok(ToolSchedulingProgress::WaitingForApproval)
    ));
    resolve(&fixture, ActionApprovalDecision::Decline);
    fixture
        .scheduler
        .run_pending(
            &fixture.thread_id,
            &fixture.turn_id,
            &CancellationSource::new().token(),
        )
        .unwrap();

    assert!(matches!(
        fixture.tools.authorizations.lock().unwrap().as_slice(),
        [ToolAuthorization::Sandboxed(_)]
    ));
    let snapshot = fixture.threads.read_thread(&fixture.thread_id).unwrap();
    assert!(!snapshot.escalated_tool_calls.contains(&fixture.call_id));
    assert!(snapshot.items.iter().any(|item| matches!(
        item,
        ThreadItem::ToolResult {
            tool_call_id,
            text,
            is_error: true,
            ..
        } if tool_call_id == &fixture.call_id && text.contains("declined")
    )));
}

#[test]
fn interrupted_approved_sandbox_escalation_is_not_retried() {
    let tools = Arc::new(ReviewTool {
        outputs: Mutex::new(VecDeque::from([safe_sandbox_denial()])),
        ..ReviewTool::default()
    });
    let engine = PolicyEngine::new(
        PolicyRevision::new("policy-v1"),
        DenialAskingClassifier,
        ReviewFailurePolicy::Block,
    );
    let fixture = fixture_with(tools, Arc::new(engine));
    fixture
        .scheduler
        .run_pending(
            &fixture.thread_id,
            &fixture.turn_id,
            &CancellationSource::new().token(),
        )
        .unwrap();
    let snapshot = fixture.threads.read_thread(&fixture.thread_id).unwrap();
    let interaction = snapshot
        .turns
        .last()
        .and_then(|turn| turn.pending_interaction.clone())
        .unwrap();
    let denial = match &interaction.request {
        AgentRequest::Approval { request } => request.sandbox_denial.clone().unwrap(),
        _ => panic!("sandbox escalation must request approval"),
    };
    resolve(&fixture, ActionApprovalDecision::ApproveOnce);
    let reviewed = review_request(&durable_call(&fixture), false);
    fixture
        .threads
        .record_tool_execution_escalated(
            &fixture.thread_id,
            &fixture.turn_id,
            crate::thread_controller::RecordToolExecutionEscalation {
                tool_call_id: fixture.call_id.clone(),
                action_digest: reviewed.action().digest().as_str().to_owned(),
                policy_revision: reviewed.policy_revision().as_str().to_owned(),
                denial,
                authority: ToolExecutionAuthority::ApprovedOnce {
                    request_id: interaction.request_id,
                },
            },
        )
        .unwrap();

    fixture
        .scheduler
        .run_pending(
            &fixture.thread_id,
            &fixture.turn_id,
            &CancellationSource::new().token(),
        )
        .unwrap();

    assert!(matches!(
        fixture.tools.authorizations.lock().unwrap().as_slice(),
        [ToolAuthorization::Sandboxed(_)]
    ));
    assert!(
        fixture
            .threads
            .read_thread(&fixture.thread_id)
            .unwrap()
            .items
            .iter()
            .any(|item| matches!(
                item,
                ThreadItem::ToolResult {
                    tool_call_id,
                    text,
                    is_error: true,
                    ..
                } if tool_call_id == &fixture.call_id
                    && text.contains("process interruption")
                    && text.contains("not retried")
            ))
    );
}

fn safe_sandbox_denial() -> ToolExecutionOutput {
    ToolExecutionOutput::SandboxDenied(SandboxDenialOutput::safe_to_retry(
        "network access denied",
        ProcessExecutionOutput::from_captured_streams(
            ProcessExitStatus::Code(1),
            "",
            "connect: operation not permitted",
        ),
    ))
}

#[test]
fn sandbox_denial_with_possible_side_effects_is_not_retried() {
    let observed = Arc::new(Mutex::new(Vec::new()));
    let tools = Arc::new(ReviewTool {
        outputs: Mutex::new(VecDeque::from([ToolExecutionOutput::SandboxDenied(
            SandboxDenialOutput::may_have_side_effects(
                "write denied after process launch",
                ProcessExecutionOutput::from_captured_streams(
                    ProcessExitStatus::Code(1),
                    "partial output",
                    "",
                ),
            ),
        )])),
        ..ReviewTool::default()
    });
    let engine = PolicyEngine::new(
        PolicyRevision::new("policy-v1"),
        DenialApprovingClassifier {
            observed: observed.clone(),
        },
        ReviewFailurePolicy::Block,
    );
    let fixture = fixture_with(tools, Arc::new(engine));

    fixture
        .scheduler
        .run_pending(
            &fixture.thread_id,
            &fixture.turn_id,
            &CancellationSource::new().token(),
        )
        .unwrap();

    assert!(matches!(
        fixture.tools.authorizations.lock().unwrap().as_slice(),
        [ToolAuthorization::Sandboxed(_)]
    ));
    assert!(observed.lock().unwrap().is_empty());
    let snapshot = fixture.threads.read_thread(&fixture.thread_id).unwrap();
    assert!(!snapshot.escalated_tool_calls.contains(&fixture.call_id));
    assert!(snapshot.items.iter().any(|item| matches!(
        item,
        ThreadItem::ToolResult {
            tool_call_id,
            text,
            is_error: true,
            ..
        } if tool_call_id == &fixture.call_id
            && text.contains("may have produced side effects")
            && text.contains("partial output")
    )));
}

#[test]
fn reviewer_revision_returns_structured_safer_path_feedback() {
    let tools = Arc::new(ReviewTool {
        requires_escalation: true,
        ..ReviewTool::default()
    });
    let engine = PolicyEngine::new(
        PolicyRevision::new("policy-v1"),
        RevisingClassifier,
        ReviewFailurePolicy::Block,
    );
    let fixture = fixture_with(tools, Arc::new(engine));

    fixture
        .scheduler
        .run_pending(
            &fixture.thread_id,
            &fixture.turn_id,
            &CancellationSource::new().token(),
        )
        .unwrap();

    assert!(fixture.tools.authorizations.lock().unwrap().is_empty());
    assert!(
        fixture
            .threads
            .read_thread(&fixture.thread_id)
            .unwrap()
            .items
            .iter()
            .any(|item| matches!(
                item,
                ThreadItem::ToolResult {
                    text,
                    is_error: true,
                    ..
                } if text.starts_with("zeta_policy_feedback:")
                    && text.contains("\"kind\":\"revise_action\"")
                    && text.contains("\"maximum_capabilities\"")
            ))
    );
}

#[test]
fn started_call_without_a_result_is_not_retried() {
    let fixture = fixture();
    let call = durable_call(&fixture);
    let reviewed = review_request(&call, false);
    fixture
        .threads
        .record_tool_execution_started(
            &fixture.thread_id,
            &fixture.turn_id,
            crate::thread_controller::RecordToolExecutionStart {
                tool_call_id: fixture.call_id.clone(),
                action_digest: reviewed.action().digest().as_str().to_owned(),
                policy_revision: reviewed.policy_revision().as_str().to_owned(),
                authority: zeta_protocol::ToolExecutionAuthority::Sandboxed,
            },
        )
        .unwrap();

    fixture
        .scheduler
        .run_pending(
            &fixture.thread_id,
            &fixture.turn_id,
            &CancellationSource::new().token(),
        )
        .unwrap();

    assert!(fixture.tools.authorizations.lock().unwrap().is_empty());
    assert!(
        fixture
            .threads
            .read_thread(&fixture.thread_id)
            .unwrap()
            .items
            .iter()
            .any(|item| matches!(
                item,
                ThreadItem::ToolResult {
                    tool_call_id,
                    text,
                    is_error: true,
                    ..
                } if tool_call_id == &fixture.call_id && text.contains("not retried")
            ))
    );
}

fn durable_call(fixture: &Fixture) -> ToolCall {
    let snapshot = fixture.threads.read_thread(&fixture.thread_id).unwrap();
    snapshot
        .items
        .iter()
        .find_map(|item| match item {
            ThreadItem::ToolCall {
                tool_call_id,
                name,
                arguments_json,
                ..
            } if tool_call_id == &fixture.call_id => Some(ToolCall {
                id: tool_call_id.clone(),
                name: name.clone(),
                arguments: serde_json::from_str(arguments_json).unwrap(),
            }),
            _ => None,
        })
        .unwrap()
}

#[test]
fn resolved_approval_survives_recovery_as_a_resumable_continuation() {
    let fixture = fixture();
    fixture
        .scheduler
        .run_pending(
            &fixture.thread_id,
            &fixture.turn_id,
            &CancellationSource::new().token(),
        )
        .unwrap();
    resolve(&fixture, ActionApprovalDecision::ApproveOnce);

    let recovered = Arc::new(ThreadController::with_store(fixture.store.clone()));
    let snapshot = recovered.recover_thread(&fixture.thread_id).unwrap();
    assert_eq!(snapshot.turns.last().unwrap().status, TurnStatus::Running);

    ToolScheduler::new(
        recovered.clone(),
        fixture.tools.clone(),
        Arc::new(AskPolicy),
    )
    .run_pending(
        &fixture.thread_id,
        &fixture.turn_id,
        &CancellationSource::new().token(),
    )
    .unwrap();
    assert!(matches!(
        fixture.tools.authorizations.lock().unwrap().as_slice(),
        [ToolAuthorization::ApprovedOnce(_)]
    ));
}

#[test]
fn recovered_tool_continuation_fails_closed_after_policy_revision_changes() {
    let fixture = fixture();
    let recovered = Arc::new(ThreadController::with_store(fixture.store.clone()));
    recovered.recover_thread(&fixture.thread_id).unwrap();

    let result = ToolScheduler::new(
        recovered.clone(),
        fixture.tools.clone(),
        Arc::new(ChangedPolicy),
    )
    .run_pending(
        &fixture.thread_id,
        &fixture.turn_id,
        &CancellationSource::new().token(),
    );
    let error = match result {
        Ok(_) => panic!("recovered continuation must reject a changed policy revision"),
        Err(error) => error,
    };

    assert!(
        matches!(error, CoreError::Policy(message) if message.contains("changed from test-policy-v1 to test-policy-v2"))
    );
    assert!(fixture.tools.authorizations.lock().unwrap().is_empty());
    assert!(
        recovered
            .read_thread(&fixture.thread_id)
            .unwrap()
            .turns
            .last()
            .unwrap()
            .pending_interaction
            .is_none()
    );
}

struct Fixture {
    store: Arc<InMemoryThreadStore>,
    threads: Arc<ThreadController>,
    scheduler: ToolScheduler,
    tools: Arc<ReviewTool>,
    thread_id: ThreadId,
    turn_id: TurnId,
    call_id: ToolCallId,
}

fn fixture() -> Fixture {
    fixture_with(Arc::new(ReviewTool::default()), Arc::new(AskPolicy))
}

fn fixture_with(tools: Arc<ReviewTool>, policy: Arc<dyn PolicyService>) -> Fixture {
    let store = Arc::new(InMemoryThreadStore::default());
    let threads = Arc::new(ThreadController::with_store(store.clone()));
    let policy_revision = policy.revision();
    let thread_id = ThreadId::new("thread").unwrap();
    threads
        .create_thread(CreateThreadRequest {
            session_id: SessionId::new("session").unwrap(),
            thread_id: thread_id.clone(),
            title: "scheduler".into(),
        })
        .unwrap();
    let turn_id = threads
        .start_turn(
            &thread_id,
            StartTurnRequest {
                command_id: CommandId::new("start").unwrap(),
                expected_sequence: SequenceExpectation::Any,
                model: None,
                policy_revision,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text { text: "run".into() }],
            },
        )
        .unwrap()
        .turn_id;
    let call_id = ToolCallId::new("call-1").unwrap();
    threads
        .record_model_tool_call(
            &thread_id,
            &turn_id,
            &ToolCall {
                id: call_id.clone(),
                name: ToolName::new("reviewed").unwrap(),
                arguments: json!({"value": 1}),
            },
        )
        .unwrap();
    let scheduler = ToolScheduler::new(threads.clone(), tools.clone(), policy);
    Fixture {
        store,
        threads,
        scheduler,
        tools,
        thread_id,
        turn_id,
        call_id,
    }
}

fn resolve(fixture: &Fixture, decision: ActionApprovalDecision) {
    let snapshot = fixture.threads.read_thread(&fixture.thread_id).unwrap();
    let interaction = snapshot
        .turns
        .iter()
        .find(|turn| turn.turn_id == fixture.turn_id)
        .and_then(|turn| turn.pending_interaction.clone())
        .unwrap();
    fixture
        .threads
        .resolve_turn_interaction(
            &fixture.thread_id,
            ResolveTurnInteractionRequest {
                command_id: CommandId::new(format!("resolve-{decision:?}")).unwrap(),
                expected_sequence: SequenceExpectation::Exact(snapshot.sequence),
                turn_id: fixture.turn_id.clone(),
                request_id: interaction.request_id,
                response: AgentResponse::Approval {
                    response: ActionApprovalResponse { decision },
                },
            },
        )
        .unwrap();
    assert_eq!(
        fixture
            .threads
            .read_thread(&fixture.thread_id)
            .unwrap()
            .turns
            .last()
            .unwrap()
            .status,
        TurnStatus::Running
    );
}

struct ReviewTool {
    authorizations: Mutex<Vec<ToolAuthorization>>,
    outputs: Mutex<VecDeque<ToolExecutionOutput>>,
    requires_escalation: bool,
    evidence: Vec<ReviewEvidence>,
}

impl Default for ReviewTool {
    fn default() -> Self {
        Self {
            authorizations: Mutex::new(Vec::new()),
            outputs: Mutex::new(VecDeque::new()),
            requires_escalation: false,
            evidence: Vec::new(),
        }
    }
}

impl ToolService for ReviewTool {
    fn definitions(&self) -> Vec<ToolDefinition> {
        Vec::new()
    }

    fn prepare(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        Ok(review_request(call, self.requires_escalation))
    }

    fn review_evidence(&self, _: &ToolCall) -> Result<Vec<ReviewEvidence>, CoreError> {
        Ok(self.evidence.clone())
    }

    fn execute(
        &self,
        _: &ToolCall,
        authorization: &ToolAuthorization,
        _: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        self.authorizations
            .lock()
            .unwrap()
            .push(authorization.clone());
        Ok(self
            .outputs
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| ToolExecutionOutput::Success("executed".into())))
    }
}

struct AskPolicy;

struct ChangedPolicy;

impl PolicyService for ChangedPolicy {
    fn revision(&self) -> String {
        "test-policy-v2".into()
    }

    fn decide(
        &self,
        _: &ActionReviewRequest,
        _: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        panic!("revision mismatch must fail before evaluating the changed policy")
    }
}

impl PolicyService for AskPolicy {
    fn revision(&self) -> String {
        "test-policy-v1".into()
    }

    fn decide(
        &self,
        request: &ActionReviewRequest,
        _: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        Ok(ExecutionDecision::AskUser(ApprovalRequest::new(
            request.action().digest().clone(),
            request.action().required_capabilities().clone(),
            "external mutation requires approval",
        )))
    }
}

struct ContextApprovingClassifier {
    observed: Arc<Mutex<Option<zeta_policy::ReviewContext>>>,
}

struct DenialApprovingClassifier {
    observed: Arc<Mutex<Vec<ActionReviewPhase>>>,
}

struct DenialAskingClassifier;

#[derive(Debug)]
struct ContextClassifierError;

impl std::fmt::Display for ContextClassifierError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("context classifier failed")
    }
}

impl std::error::Error for ContextClassifierError {}

impl ActionClassifier for ContextApprovingClassifier {
    type Error = ContextClassifierError;

    fn classify(
        &self,
        request: &ActionReviewRequest,
        _: &CancellationToken,
    ) -> Result<ClassifierAssessment, Self::Error> {
        *self.observed.lock().unwrap() = Some(request.context().clone());
        Ok(ClassifierAssessment::new(
            AssessmentId::new("auto-assessment"),
            request.action().digest().clone(),
            request.policy_revision().clone(),
            "test-prompt",
            ClassifierRecommendation::Approve {
                capabilities: request.action().required_capabilities().clone(),
                risk: RiskLevel::Medium,
                user_authorization: UserAuthorization::Implicit,
                reason: "matches the requested operation".into(),
            },
        ))
    }
}

impl ActionClassifier for DenialApprovingClassifier {
    type Error = ContextClassifierError;

    fn classify(
        &self,
        request: &ActionReviewRequest,
        _: &CancellationToken,
    ) -> Result<ClassifierAssessment, Self::Error> {
        self.observed.lock().unwrap().push(request.phase().clone());
        Ok(ClassifierAssessment::new(
            AssessmentId::new("denial-assessment"),
            request.action().digest().clone(),
            request.policy_revision().clone(),
            "test-prompt",
            ClassifierRecommendation::Approve {
                capabilities: request.action().required_capabilities().clone(),
                risk: RiskLevel::Medium,
                user_authorization: UserAuthorization::Implicit,
                reason: "outside-sandbox retry is allowed".into(),
            },
        ))
    }
}

impl ActionClassifier for DenialAskingClassifier {
    type Error = ContextClassifierError;

    fn classify(
        &self,
        request: &ActionReviewRequest,
        _: &CancellationToken,
    ) -> Result<ClassifierAssessment, Self::Error> {
        assert!(matches!(
            request.phase(),
            ActionReviewPhase::SandboxDenial(_)
        ));
        Ok(ClassifierAssessment::new(
            AssessmentId::new("denial-user-assessment"),
            request.action().digest().clone(),
            request.policy_revision().clone(),
            "test-prompt",
            ClassifierRecommendation::Approve {
                capabilities: request.action().required_capabilities().clone(),
                risk: RiskLevel::High,
                user_authorization: UserAuthorization::Implicit,
                reason: "outside-sandbox retry requires explicit user approval".into(),
            },
        ))
    }
}

struct RevisingClassifier;

impl ActionClassifier for RevisingClassifier {
    type Error = ContextClassifierError;

    fn classify(
        &self,
        request: &ActionReviewRequest,
        _: &CancellationToken,
    ) -> Result<ClassifierAssessment, Self::Error> {
        Ok(ClassifierAssessment::new(
            AssessmentId::new("revise-assessment"),
            request.action().digest().clone(),
            request.policy_revision().clone(),
            "test-prompt",
            ClassifierRecommendation::ReviseAction {
                maximum_capabilities: CapabilitySet::default(),
                reason: "use a read-only inspection first".into(),
            },
        ))
    }
}

fn review_request(call: &ToolCall, requires_escalation: bool) -> ActionReviewRequest {
    let capabilities = CapabilitySet::new([Capability::new(
        CapabilityKind::ExternalMutation,
        "reviewed/value/1",
    )]);
    ActionReviewRequest::new(
        ResolvedAction::new(
            ActionDigest::from_canonical_bytes(format!("{}:{}", call.name, call.arguments)),
            ActionKind::ExternalServiceMutation,
            "mutate reviewed value",
            capabilities,
        ),
        ActionProvenance::new(ActionSource::BuiltInTool, "reviewed"),
        if requires_escalation {
            SandboxCompatibility::NotApplicable {
                reason: "external mutation".into(),
            }
        } else {
            SandboxCompatibility::Supported(SandboxPolicy::new(
                FileSystemAccess::ReadOnly,
                NetworkAccess::Denied,
            ))
        },
        PolicyRevision::new("policy-v1"),
    )
}
