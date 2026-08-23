use super::ContextManager;
use crate::ContextBudget;
use crate::ThreadCommandSnapshot;
use crate::ThreadSnapshot;
use crate::TurnSnapshot;
use crate::context::ContextInput;
use crate::context::ContextPreparation;
use crate::context::ContextPreparationError;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use zeta_protocol::ItemId;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadItem;
use zeta_protocol::TurnId;
use zeta_protocol::TurnStatus;

#[test]
fn manager_can_be_discarded_and_rebuilt_from_the_same_durable_facts() {
    let turn_id = id::<TurnId>("turn");
    let snapshot = snapshot(7, turn_id.clone());
    let input = ContextInput::new(
        &snapshot,
        turn_id,
        Vec::new(),
        Vec::new(),
        ContextBudget::provider_managed(),
    );

    let first = ContextManager::default().prepare(&input).unwrap();
    let second = ContextManager::default().prepare(&input).unwrap();

    assert_eq!(format!("{first:?}"), format!("{second:?}"));
    assert!(matches!(first, ContextPreparation::Ready(_)));
}

#[test]
fn manager_rejects_a_snapshot_older_than_its_observed_sequence() {
    let turn_id = id::<TurnId>("turn");
    let older = snapshot(7, turn_id.clone());
    let newer = snapshot(8, turn_id.clone());
    let older_input = ContextInput::new(
        &older,
        turn_id.clone(),
        Vec::new(),
        Vec::new(),
        ContextBudget::provider_managed(),
    );
    let newer_input = ContextInput::new(
        &newer,
        turn_id,
        Vec::new(),
        Vec::new(),
        ContextBudget::provider_managed(),
    );
    let mut manager = ContextManager::default();

    manager.prepare(&newer_input).unwrap();

    assert!(matches!(
        manager.prepare(&older_input),
        Err(ContextPreparationError::UnsupportedContextShape(_))
    ));
}

fn snapshot(sequence: u64, turn_id: TurnId) -> ThreadSnapshot {
    ThreadSnapshot {
        session_id: id::<SessionId>("session"),
        thread_id: id::<ThreadId>("thread"),
        title: "test".into(),
        turn_execution_binding: None,
        sequence,
        usage: zeta_protocol::ModelUsageSummary::default(),
        context_calibrations: Vec::new(),
        turns: vec![TurnSnapshot {
            turn_id: turn_id.clone(),
            status: TurnStatus::Running,
            model: None,
            policy_revision: "test-policy-v1".into(),
            approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
            activated_skills: Vec::new(),
            failure: None,
            pending_interaction: None,
            execution_backend_attempt: None,
            resource_budget: None,
            tool_profile: None,
            plan: None,
            usage: zeta_protocol::ModelUsageSummary::default(),
        }],
        items: vec![ThreadItem::UserMessage {
            item_id: id::<ItemId>("item"),
            turn_id,
            text: "hello".into(),
        }],
        context_checkpoints: Vec::new(),
        context_overflow_recoveries: BTreeMap::new(),
        item_sequences: BTreeMap::new(),
        event_digests: BTreeMap::new(),
        commands: Vec::<ThreadCommandSnapshot>::new(),
        steer_deliveries: BTreeMap::new(),
        seen_interaction_ids: BTreeSet::new(),
        resolved_interactions: Vec::new(),
        started_tool_calls: BTreeSet::new(),
        tool_execution_starts: BTreeMap::new(),
        escalated_tool_calls: BTreeSet::new(),
        agent_context_seed: None,
        delegations: BTreeMap::new(),
        agent_cancellations_received: BTreeSet::new(),
        agent_joins: BTreeMap::new(),
        produced_delegation_results: BTreeMap::new(),
        received_delegation_results: BTreeMap::new(),
        sent_agent_messages: BTreeMap::new(),
        received_agent_messages: BTreeMap::new(),
    }
}

trait TestId: Sized {
    fn from_test(value: &str) -> Self;
}

macro_rules! impl_test_id {
    ($($type:ty),+ $(,)?) => {
        $(
            impl TestId for $type {
                fn from_test(value: &str) -> Self {
                    Self::new(value).expect("test ID is non-empty")
                }
            }
        )+
    };
}

impl_test_id!(ItemId, SessionId, ThreadId, TurnId);

fn id<T: TestId>(value: &str) -> T {
    T::from_test(value)
}
