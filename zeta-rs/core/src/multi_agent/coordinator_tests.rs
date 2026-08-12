use super::CompleteDelegationRequest;
use super::JoinAgentsRequest;
use super::MultiAgentCoordinator;
use super::SendAgentMessageRequest;
use super::SpawnAgentRequest;
use super::build_context_seed;
use crate::AgentTreeLimits;
use crate::CommandDisposition;
use crate::ContextBudget;
use crate::CreateSessionRequest;
use crate::CreateSessionThreadRequest;
use crate::HarnessInstructions;
use crate::InMemorySessionStore;
use crate::InMemoryThreadStore;
use crate::SequenceExpectation;
use crate::SessionCoordinator;
use crate::StartTurnRequest;
use crate::ThreadController;
use crate::context::ModelInvocationPreparation;
use crate::thread_controller::PrepareModelInvocationRequest;
use std::sync::Arc;
use zeta_protocol::AgentContextMode;
use zeta_protocol::AgentContextSource;
use zeta_protocol::AgentJoin;
use zeta_protocol::AgentJoinId;
use zeta_protocol::AgentJoinPolicy;
use zeta_protocol::AgentJoinStatus;
use zeta_protocol::AgentMessageId;
use zeta_protocol::AgentMessageProvenance;
use zeta_protocol::AgentRoleSnapshot;
use zeta_protocol::CommandId;
use zeta_protocol::ContentDigest;
use zeta_protocol::DelegatedCapabilityScope;
use zeta_protocol::DelegatedPolicyCeiling;
use zeta_protocol::DelegatedTask;
use zeta_protocol::DelegationId;
use zeta_protocol::DelegationResult;
use zeta_protocol::DelegationResultDigest;
use zeta_protocol::DelegationResultStatus;
use zeta_protocol::ForkedAgentContext;
use zeta_protocol::FrozenSkillActivation;
use zeta_protocol::SessionId;
use zeta_protocol::SkillActivationReason;
use zeta_protocol::SkillId;
use zeta_protocol::SkillName;
use zeta_protocol::SkillSourceId;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadItem;
use zeta_protocol::ThreadOrigin;
use zeta_protocol::ThreadSequenceRange;
use zeta_protocol::ToolDefinition;
use zeta_protocol::ToolName;
use zeta_protocol::TurnId;
use zeta_protocol::UserInput;

struct Fixture {
    sessions: Arc<SessionCoordinator>,
    coordinator: MultiAgentCoordinator,
    session_id: SessionId,
    parent_thread_id: ThreadId,
    parent_turn_id: TurnId,
}

#[test]
fn spawn_creates_seeded_child_thread_and_initial_turn_idempotently() {
    let fixture = fixture();
    let first = fixture.coordinator.spawn(spawn_request(&fixture)).unwrap();
    let replayed = fixture.coordinator.spawn(spawn_request(&fixture)).unwrap();

    assert_eq!(first.child_thread_id, replayed.child_thread_id);
    assert_eq!(first.child_turn_id, replayed.child_turn_id);
    assert_eq!(first.disposition, CommandDisposition::Committed);
    assert_eq!(replayed.disposition, CommandDisposition::Replayed);
    assert_eq!(first.context_seed.parent_sequence, 4);

    let session = fixture.sessions.read_session(&fixture.session_id).unwrap();
    let membership = session
        .threads
        .iter()
        .find(|thread| thread.membership.thread_id == first.child_thread_id)
        .unwrap();
    assert_eq!(
        membership.membership.origin,
        ThreadOrigin::AgentSpawn {
            parent_thread_id: fixture.parent_thread_id.clone(),
            parent_sequence: 4,
            delegation_id: DelegationId::new("delegation-review").unwrap(),
        }
    );

    let child = fixture
        .sessions
        .threads()
        .read_thread(&first.child_thread_id)
        .unwrap();
    assert_eq!(child.agent_context_seed, Some(first.context_seed));
    assert_eq!(child.turns.len(), 1);
    assert_eq!(child.turns[0].activated_skills, vec![test_activation()]);
    let ModelInvocationPreparation::Ready(invocation) = fixture
        .sessions
        .threads()
        .prepare_model_invocation(
            &first.child_thread_id,
            PrepareModelInvocationRequest {
                turn_id: &first.child_turn_id,
                instructions: &HarnessInstructions::default(),
                extension_fragments: Vec::new(),
                evidence: Vec::new(),
                tools: vec![tool_definition("allowed"), tool_definition("blocked")],
                budget: ContextBudget::provider_managed(),
            },
        )
        .unwrap()
    else {
        panic!("provider-managed context should be ready")
    };
    assert_eq!(
        invocation
            .context()
            .tools()
            .iter()
            .map(|definition| definition.name.as_str())
            .collect::<Vec<_>>(),
        vec!["allowed"]
    );
    assert!(child.items.iter().any(|item| {
        matches!(item, ThreadItem::UserMessage { text, .. } if text == "Review the change")
    }));
}

#[test]
fn selected_and_forked_context_are_materialized_into_the_immutable_child_seed() {
    let fixture = fixture();
    let parent = fixture
        .sessions
        .threads()
        .read_thread(&fixture.parent_thread_id)
        .unwrap();
    let item_id = parent.items[0].item_id().clone();
    let mut selected = spawn_request_with_id(&fixture, "delegation-selected");
    selected.inheritance = AgentContextMode::Selected {
        sources: vec![AgentContextSource::Item {
            source_thread_id: fixture.parent_thread_id.clone(),
            source_sequence: parent.sequence,
            item_id,
        }],
    };
    let selected = fixture.coordinator.spawn(selected).unwrap();
    assert_eq!(selected.context_seed.materialized_context.len(), 1);

    let mut forked = spawn_request_with_id(&fixture, "delegation-forked");
    forked.inheritance = AgentContextMode::ForkedPrefix {
        selection: ForkedAgentContext::LastTurns { count: 1 },
    };
    let forked = fixture.coordinator.spawn(forked).unwrap();
    assert_eq!(forked.context_seed.materialized_context.len(), 1);
    let ModelInvocationPreparation::Ready(invocation) = fixture
        .sessions
        .threads()
        .prepare_model_invocation(
            &forked.child_thread_id,
            PrepareModelInvocationRequest {
                turn_id: &forked.child_turn_id,
                instructions: &HarnessInstructions::default(),
                extension_fragments: Vec::new(),
                evidence: Vec::new(),
                tools: vec![tool_definition("allowed")],
                budget: ContextBudget::provider_managed(),
            },
        )
        .unwrap()
    else {
        panic!("forked context should be ready")
    };
    assert!(
        invocation
            .context()
            .instructions()
            .iter()
            .any(|instruction| {
                instruction.body().contains("Delegate a review")
                    && instruction.body().contains("inherited-agent-context")
            })
    );
}

#[test]
fn joins_freeze_targets_and_satisfy_all_any_and_quorum_durably() {
    let fixture = fixture();
    let first = fixture
        .coordinator
        .spawn(spawn_request_with_id(&fixture, "delegation-first"))
        .unwrap();
    let second = fixture
        .coordinator
        .spawn(spawn_request_with_id(&fixture, "delegation-second"))
        .unwrap();
    for spawned in [&first, &second] {
        fixture
            .sessions
            .threads()
            .complete_turn(
                &spawned.child_thread_id,
                &spawned.child_turn_id,
                "done".into(),
            )
            .unwrap();
    }
    fixture
        .coordinator
        .complete_delegation(CompleteDelegationRequest {
            parent_thread_id: fixture.parent_thread_id.clone(),
            delegation_id: first.delegation_id.clone(),
            status: DelegationResultStatus::Completed,
            summary: "first".into(),
            artifacts: Vec::new(),
        })
        .unwrap();

    let any = fixture
        .coordinator
        .join(JoinAgentsRequest {
            join_id: AgentJoinId::new("join-any").unwrap(),
            parent_thread_id: fixture.parent_thread_id.clone(),
            policy: AgentJoinPolicy::Any,
            delegations: None,
        })
        .unwrap();
    assert_eq!(any.join.status, AgentJoinStatus::Satisfied);
    assert_eq!(any.results.len(), 1);
    let all = fixture
        .coordinator
        .join(JoinAgentsRequest {
            join_id: AgentJoinId::new("join-all").unwrap(),
            parent_thread_id: fixture.parent_thread_id.clone(),
            policy: AgentJoinPolicy::All,
            delegations: None,
        })
        .unwrap();
    assert_eq!(all.join.status, AgentJoinStatus::Waiting);
    let quorum = fixture
        .coordinator
        .join(JoinAgentsRequest {
            join_id: AgentJoinId::new("join-quorum").unwrap(),
            parent_thread_id: fixture.parent_thread_id.clone(),
            policy: AgentJoinPolicy::Quorum { count: 2 },
            delegations: None,
        })
        .unwrap();
    assert_eq!(quorum.join.status, AgentJoinStatus::Waiting);

    fixture
        .coordinator
        .complete_delegation(CompleteDelegationRequest {
            parent_thread_id: fixture.parent_thread_id.clone(),
            delegation_id: second.delegation_id,
            status: DelegationResultStatus::Completed,
            summary: "second".into(),
            artifacts: Vec::new(),
        })
        .unwrap();
    let parent = fixture
        .sessions
        .threads()
        .read_thread(&fixture.parent_thread_id)
        .unwrap();
    assert_eq!(
        parent
            .agent_joins
            .get(&AgentJoinId::new("join-all").unwrap())
            .unwrap()
            .status,
        AgentJoinStatus::Satisfied
    );
    assert_eq!(
        parent
            .agent_joins
            .get(&AgentJoinId::new("join-quorum").unwrap())
            .unwrap()
            .status,
        AgentJoinStatus::Satisfied
    );
}

#[test]
fn cancelling_a_parent_delegation_interrupts_every_live_descendant() {
    let fixture = fixture();
    let child = fixture
        .coordinator
        .spawn(spawn_request_with_id(&fixture, "delegation-child"))
        .unwrap();
    let grandchild = fixture
        .coordinator
        .spawn(SpawnAgentRequest {
            delegation_id: DelegationId::new("delegation-grandchild").unwrap(),
            session_id: fixture.session_id.clone(),
            parent_thread_id: child.child_thread_id.clone(),
            parent_turn_id: child.child_turn_id.clone(),
            task: DelegatedTask {
                title: "nested".into(),
                instructions: "Check one nested detail".into(),
            },
            role: AgentRoleSnapshot {
                name: "nested".into(),
                instructions: "Return the nested detail.".into(),
                model: None,
            },
            inheritance: AgentContextMode::Fresh,
            policy_ceiling: DelegatedPolicyCeiling {
                policy_revision: "policy-v1".into(),
            },
            capability_scope: DelegatedCapabilityScope {
                tools: Vec::new(),
                skills: Vec::new(),
            },
        })
        .unwrap();

    let result = fixture
        .coordinator
        .cancel_delegation(&fixture.parent_thread_id, &child.delegation_id)
        .unwrap();
    assert_eq!(result.status, DelegationResultStatus::Cancelled);
    let child_snapshot = fixture
        .sessions
        .threads()
        .read_thread(&child.child_thread_id)
        .unwrap();
    assert!(
        child_snapshot
            .agent_cancellations_received
            .contains(&child.delegation_id)
    );
    assert_eq!(
        child_snapshot.turns[0].status,
        zeta_protocol::TurnStatus::Interrupted
    );
    assert_eq!(
        child_snapshot
            .received_delegation_results
            .get(&grandchild.delegation_id)
            .unwrap()
            .status,
        DelegationResultStatus::Cancelled
    );
    let grandchild_snapshot = fixture
        .sessions
        .threads()
        .read_thread(&grandchild.child_thread_id)
        .unwrap();
    assert_eq!(
        grandchild_snapshot.turns[0].status,
        zeta_protocol::TurnStatus::Interrupted
    );
}

#[test]
fn recovery_finishes_waiting_joins_and_committed_tree_cancellation() {
    let fixture = fixture();
    let completed = fixture
        .coordinator
        .spawn(spawn_request_with_id(&fixture, "delegation-completed"))
        .unwrap();
    fixture
        .sessions
        .threads()
        .complete_turn(
            &completed.child_thread_id,
            &completed.child_turn_id,
            "done".into(),
        )
        .unwrap();
    fixture
        .coordinator
        .complete_delegation(CompleteDelegationRequest {
            parent_thread_id: fixture.parent_thread_id.clone(),
            delegation_id: completed.delegation_id.clone(),
            status: DelegationResultStatus::Completed,
            summary: "done".into(),
            artifacts: Vec::new(),
        })
        .unwrap();
    let join_id = AgentJoinId::new("join-recovery").unwrap();
    fixture
        .sessions
        .threads()
        .record_agent_join_requested(
            &fixture.parent_thread_id,
            AgentJoin {
                join_id: join_id.clone(),
                parent_thread_id: fixture.parent_thread_id.clone(),
                policy: AgentJoinPolicy::All,
                delegations: vec![completed.delegation_id],
                status: AgentJoinStatus::Waiting,
                satisfied_by: Vec::new(),
            },
        )
        .unwrap();
    let cancelled = fixture
        .coordinator
        .spawn(spawn_request_with_id(
            &fixture,
            "delegation-cancel-recovery",
        ))
        .unwrap();
    fixture
        .sessions
        .threads()
        .record_delegation_cancellation_requested(
            &fixture.parent_thread_id,
            cancelled.delegation_id.clone(),
        )
        .unwrap();

    fixture
        .coordinator
        .recover_session(&fixture.session_id)
        .unwrap();

    let parent = fixture
        .sessions
        .threads()
        .read_thread(&fixture.parent_thread_id)
        .unwrap();
    assert_eq!(
        parent.agent_joins.get(&join_id).unwrap().status,
        AgentJoinStatus::Satisfied
    );
    assert_eq!(
        parent
            .received_delegation_results
            .get(&cancelled.delegation_id)
            .unwrap()
            .status,
        DelegationResultStatus::Cancelled
    );
}

#[test]
fn recovery_finishes_a_spawn_after_only_the_parent_request_committed() {
    let fixture = fixture();
    let request = spawn_request(&fixture);
    let parent = fixture
        .sessions
        .threads()
        .read_thread(&fixture.parent_thread_id)
        .unwrap();
    let seed = build_context_seed(request, parent.sequence).unwrap();
    fixture
        .sessions
        .threads()
        .record_delegation_requested(&fixture.parent_thread_id, seed)
        .unwrap();

    let recovered = fixture
        .coordinator
        .recover_session(&fixture.session_id)
        .unwrap();

    assert_eq!(recovered.len(), 1);
    let parent = fixture
        .sessions
        .threads()
        .read_thread(&fixture.parent_thread_id)
        .unwrap();
    let delegation = parent
        .delegations
        .get(&DelegationId::new("delegation-review").unwrap())
        .unwrap();
    assert_eq!(
        delegation.child_thread_id,
        Some(recovered[0].child_thread_id.clone())
    );
}

#[test]
fn messages_and_results_apply_exactly_once_across_threads() {
    let fixture = fixture();
    let spawned = fixture.coordinator.spawn(spawn_request(&fixture)).unwrap();
    let delivered = fixture
        .coordinator
        .send_message(message_request(&fixture, &spawned.child_thread_id))
        .unwrap();
    let replayed = fixture
        .coordinator
        .send_message(message_request(&fixture, &spawned.child_thread_id))
        .unwrap();
    assert_eq!(delivered.message, replayed.message);

    let child = fixture
        .sessions
        .threads()
        .read_thread(&spawned.child_thread_id)
        .unwrap();
    assert_eq!(child.received_agent_messages.len(), 1);
    let ModelInvocationPreparation::Ready(invocation) = fixture
        .sessions
        .threads()
        .prepare_model_invocation(
            &spawned.child_thread_id,
            PrepareModelInvocationRequest {
                turn_id: &spawned.child_turn_id,
                instructions: &HarnessInstructions::default(),
                extension_fragments: Vec::new(),
                evidence: Vec::new(),
                tools: vec![tool_definition("allowed")],
                budget: ContextBudget::provider_managed(),
            },
        )
        .unwrap()
    else {
        panic!("provider-managed context should be ready")
    };
    assert!(
        invocation
            .context()
            .instructions()
            .iter()
            .any(|instruction| {
                instruction
                    .body()
                    .contains("Also inspect cancellation handling")
            })
    );

    fixture
        .sessions
        .threads()
        .complete_turn(
            &spawned.child_thread_id,
            &spawned.child_turn_id,
            "No regressions found".into(),
        )
        .unwrap();
    let first = fixture
        .coordinator
        .complete_delegation(CompleteDelegationRequest {
            parent_thread_id: fixture.parent_thread_id.clone(),
            delegation_id: spawned.delegation_id.clone(),
            status: DelegationResultStatus::Completed,
            summary: "No regressions found".into(),
            artifacts: Vec::new(),
        })
        .unwrap();
    let replayed = fixture
        .coordinator
        .complete_delegation(CompleteDelegationRequest {
            parent_thread_id: fixture.parent_thread_id.clone(),
            delegation_id: spawned.delegation_id,
            status: DelegationResultStatus::Completed,
            summary: "No regressions found".into(),
            artifacts: Vec::new(),
        })
        .unwrap();
    assert_eq!(first, replayed);

    let parent = fixture
        .sessions
        .threads()
        .read_thread(&fixture.parent_thread_id)
        .unwrap();
    assert_eq!(parent.received_delegation_results.len(), 1);
    assert_eq!(parent.received_agent_messages.len(), 1);
}

#[test]
fn reducers_reject_corrupted_seed_and_result_digests() {
    let fixture = fixture();
    let parent = fixture
        .sessions
        .threads()
        .read_thread(&fixture.parent_thread_id)
        .unwrap();
    let mut seed = build_context_seed(spawn_request(&fixture), parent.sequence).unwrap();
    seed.digest =
        zeta_protocol::ContextSeedDigest::new(format!("sha256:{}", "0".repeat(64))).unwrap();
    assert!(
        fixture
            .sessions
            .threads()
            .record_delegation_requested(&fixture.parent_thread_id, seed)
            .is_err()
    );

    let spawned = fixture.coordinator.spawn(spawn_request(&fixture)).unwrap();
    fixture
        .sessions
        .threads()
        .complete_turn(
            &spawned.child_thread_id,
            &spawned.child_turn_id,
            "done".into(),
        )
        .unwrap();
    let child = fixture
        .sessions
        .threads()
        .read_thread(&spawned.child_thread_id)
        .unwrap();
    let corrupted = DelegationResult {
        delegation_id: spawned.delegation_id,
        child_thread_id: spawned.child_thread_id.clone(),
        status: DelegationResultStatus::Completed,
        summary: "done".into(),
        artifacts: Vec::new(),
        source_range: ThreadSequenceRange {
            start_sequence: 1,
            end_sequence: child.sequence,
        },
        digest: DelegationResultDigest::new(format!("sha256:{}", "0".repeat(64))).unwrap(),
    };
    assert!(
        fixture
            .sessions
            .threads()
            .record_delegation_result_produced(&spawned.child_thread_id, corrupted)
            .is_err()
    );
}

fn fixture() -> Fixture {
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    let sessions = Arc::new(SessionCoordinator::with_store(
        Arc::new(InMemorySessionStore::default()),
        threads,
    ));
    let session = sessions
        .create_session(CreateSessionRequest {
            command_id: CommandId::new("create-session").unwrap(),
            title: "multi agent".into(),
            model: None,
        })
        .unwrap();
    let parent = sessions
        .create_thread(CreateSessionThreadRequest {
            command_id: CommandId::new("create-parent").unwrap(),
            session_id: session.session_id.clone(),
            expected_sequence: SequenceExpectation::Exact(1),
            title: "parent".into(),
        })
        .unwrap();
    let turn = sessions
        .start_turn(
            &session.session_id,
            &parent.thread_id,
            StartTurnRequest {
                command_id: CommandId::new("start-parent").unwrap(),
                expected_sequence: SequenceExpectation::Exact(1),
                model: None,
                policy_revision: "policy-v1".into(),
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "Delegate a review".into(),
                }],
            },
        )
        .unwrap();
    Fixture {
        coordinator: MultiAgentCoordinator::new(sessions.clone(), AgentTreeLimits::default()),
        sessions,
        session_id: session.session_id,
        parent_thread_id: parent.thread_id,
        parent_turn_id: turn.turn_id,
    }
}

fn spawn_request(fixture: &Fixture) -> SpawnAgentRequest {
    spawn_request_with_id(fixture, "delegation-review")
}

fn spawn_request_with_id(fixture: &Fixture, delegation_id: &str) -> SpawnAgentRequest {
    SpawnAgentRequest {
        delegation_id: DelegationId::new(delegation_id).unwrap(),
        session_id: fixture.session_id.clone(),
        parent_thread_id: fixture.parent_thread_id.clone(),
        parent_turn_id: fixture.parent_turn_id.clone(),
        task: DelegatedTask {
            title: "reviewer".into(),
            instructions: "Review the change".into(),
        },
        role: AgentRoleSnapshot {
            name: "reviewer".into(),
            instructions: "Review code and report concrete findings.".into(),
            model: None,
        },
        inheritance: AgentContextMode::Fresh,
        policy_ceiling: DelegatedPolicyCeiling {
            policy_revision: "policy-v1".into(),
        },
        capability_scope: DelegatedCapabilityScope {
            tools: vec![ToolName::new("allowed").unwrap()],
            skills: vec![test_activation()],
        },
    }
}

fn tool_definition(name: &str) -> ToolDefinition {
    ToolDefinition {
        name: ToolName::new(name).unwrap(),
        description: name.into(),
        parameters: serde_json::json!({"type": "object"}),
        strict: true,
    }
}

fn test_activation() -> FrozenSkillActivation {
    FrozenSkillActivation {
        id: SkillId::new(
            SkillSourceId::new("user:skill-source:test").unwrap(),
            SkillName::new("review").unwrap(),
        ),
        content_digest: ContentDigest::sha256(b"review body"),
        catalog_generation: 7,
        reason: SkillActivationReason::Automatic,
    }
}

fn message_request(fixture: &Fixture, child_thread_id: &ThreadId) -> SendAgentMessageRequest {
    SendAgentMessageRequest {
        message_id: AgentMessageId::new("steer-review").unwrap(),
        delegation_id: Some(DelegationId::new("delegation-review").unwrap()),
        sender_thread_id: fixture.parent_thread_id.clone(),
        receiver_thread_id: child_thread_id.clone(),
        text: "Also inspect cancellation handling".into(),
        provenance: AgentMessageProvenance::Agent,
    }
}
