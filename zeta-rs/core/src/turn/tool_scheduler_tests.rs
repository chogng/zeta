use super::*;
use crate::{
    CreateThreadRequest, InMemoryThreadStore, ResolveTurnInteractionRequest, SequenceExpectation,
    StartTurnRequest, ToolAuthorization,
};
use serde_json::json;
use std::sync::{Arc, Mutex};
use zeta_async_utils::CancellationSource;
use zeta_policy::{
    ActionClassifier, ActionDigest, ActionKind, ActionProvenance, ActionReviewRequest,
    ActionSource, ApprovalRequest, AssessmentId, Capability, CapabilityKind, CapabilitySet,
    ClassifierAssessment, ClassifierRecommendation, ExecutionDecision, PolicyEngine,
    PolicyRevision, ResolvedAction, ReviewEvidence, ReviewEvidenceKind, ReviewEvidenceTrust,
    ReviewFailurePolicy, RiskLevel, SandboxCompatibility, UserAuthorization,
};
use zeta_protocol::{
    ActionApprovalDecision, ActionApprovalResponse, AgentResponse, CommandId, SessionId, ThreadId,
    ThreadItem, ToolCallId, ToolDefinition, ToolName, TurnStatus, UserInput,
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
    requires_escalation: bool,
    evidence: Vec<ReviewEvidence>,
}

impl Default for ReviewTool {
    fn default() -> Self {
        Self {
            authorizations: Mutex::new(Vec::new()),
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
        Ok(ToolExecutionOutput::Success("executed".into()))
    }
}

struct AskPolicy;

impl PolicyService for AskPolicy {
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
