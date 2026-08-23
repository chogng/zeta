use std::sync::Arc;
use zeta_core::CreateThreadRequest;
use zeta_core::InMemoryThreadStore;
use zeta_core::SequenceExpectation;
use zeta_core::StartTurnRequest;
use zeta_core::ThreadController;
use zeta_protocol::ApprovalMode;
use zeta_protocol::CommandId;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::TurnStatus;
use zeta_protocol::UserInput;

#[test]
fn recovery_preserves_external_attempt_and_does_not_leave_the_turn_runnable() {
    let store = Arc::new(InMemoryThreadStore::default());
    let original = ThreadController::with_store(store.clone());
    let thread_id = ThreadId::new("external-recovery-thread").unwrap();
    original
        .create_thread(CreateThreadRequest {
            session_id: SessionId::new("external-recovery-session").unwrap(),
            thread_id: thread_id.clone(),
            title: "External recovery".into(),
        })
        .unwrap();
    let turn_id = original
        .start_turn(
            &thread_id,
            StartTurnRequest {
                command_id: CommandId::new("start-external-turn").unwrap(),
                expected_sequence: SequenceExpectation::Any,
                model: None,
                policy_revision: "recovery-policy-v1".into(),
                approval_mode: ApprovalMode::AskPermissions,
                resource_budget: None,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "run remotely".into(),
                }],
            },
        )
        .unwrap()
        .turn_id;
    original
        .record_turn_execution_attempt(&thread_id, &turn_id, "external-backend".into())
        .unwrap();

    let recovered = ThreadController::with_store(store);
    let snapshot = recovered.recover_thread(&thread_id).unwrap();
    let turn = snapshot
        .turns
        .iter()
        .find(|turn| turn.turn_id == turn_id)
        .unwrap();

    assert_eq!(
        turn.execution_backend_attempt.as_deref(),
        Some("external-backend")
    );
    assert_eq!(turn.status, TurnStatus::Interrupted);
}
