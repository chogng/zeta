use super::*;
use crate::server::notification_queue::NotificationQueue;
use zeta_app_server_protocol::protocol::common::AgentInteractionCapability;
use zeta_protocol::ActionApprovalCapability;
use zeta_protocol::ActionApprovalCapabilityKind;
use zeta_protocol::ActionApprovalRequest;
use zeta_protocol::AgentInteractionKind;
use zeta_protocol::AgentRequest;
use zeta_protocol::AgentRequestEnvelope;
use zeta_protocol::RequestId;
use zeta_protocol::SessionEvent;
use zeta_protocol::SessionUpdate;
use zeta_protocol::ThreadEvent;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadUpdate;
use zeta_protocol::ThreadUpdateEnvelope;
use zeta_protocol::TurnId;

#[test]
fn broker_fans_out_and_advances_each_connection_cursor() {
    let broker = UpdateBroker::default();
    let first = NotificationQueue::default();
    let second = NotificationQueue::default();
    let session_id = SessionId::new("session_1").expect("test ID is non-empty");
    broker.register(1, &first);
    broker.register(2, &second);
    broker.subscribe_session(1, session_id.clone(), 0);
    broker.subscribe_session(2, session_id.clone(), 1);
    let updates = vec![update(&session_id, 1), update(&session_id, 2)];

    broker.publish_session(&session_id, &updates);
    broker.publish_session(&session_id, &updates);

    assert_eq!(first.len(), 2);
    assert_eq!(second.len(), 1);
}

#[test]
fn broker_fans_out_filesystem_invalidation_without_a_subscription() {
    let broker = UpdateBroker::default();
    let queue = NotificationQueue::default();
    broker.register(1, &queue);

    broker.publish_fs_changed(FsChanged::PathsChanged {
        paths: vec!["src/lib.rs".into()],
    });

    let notifications = queue.drain();
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0]["method"], "fs/changed");
    assert_eq!(notifications[0]["params"]["paths"][0], "src/lib.rs");
}

#[test]
fn session_owned_thread_subscription_follows_session_lifecycle() {
    let broker = UpdateBroker::default();
    let queue = NotificationQueue::default();
    let session_id = SessionId::new("session_1").expect("test ID is non-empty");
    let thread_id = ThreadId::new("thread_1").expect("test ID is non-empty");
    broker.register(1, &queue);
    broker.subscribe_session(1, session_id.clone(), 0);
    broker.subscribe_session_thread(1, session_id.clone(), thread_id.clone(), 0);

    broker.publish_thread(&thread_id, &[thread_update(&session_id, &thread_id, 1)]);
    assert_eq!(queue.len(), 1);
    let notifications = queue.drain();
    assert_eq!(notifications[0]["method"], "session/thread/update");

    broker.unsubscribe_session(1, &session_id);
    broker.publish_thread(&thread_id, &[thread_update(&session_id, &thread_id, 2)]);
    assert_eq!(queue.len(), 0);
}

#[test]
fn session_thread_subscription_can_be_removed_independently() {
    let broker = UpdateBroker::default();
    let queue = NotificationQueue::default();
    let session_id = SessionId::new("session_1").expect("test ID is non-empty");
    let thread_id = ThreadId::new("thread_1").expect("test ID is non-empty");
    broker.register(1, &queue);
    broker.subscribe_session_thread(1, session_id.clone(), thread_id.clone(), 0);
    broker.publish_thread(&thread_id, &[thread_update(&session_id, &thread_id, 1)]);
    assert_eq!(queue.len(), 1);
    queue.drain();

    broker.unsubscribe_session_thread(1, &session_id, &thread_id);
    broker.publish_thread(&thread_id, &[thread_update(&session_id, &thread_id, 2)]);
    assert_eq!(queue.len(), 0);
}

#[test]
fn agent_request_is_delivered_to_exactly_one_capable_thread_subscriber() {
    let broker = UpdateBroker::default();
    let first = NotificationQueue::default();
    let second = NotificationQueue::default();
    let session_id = SessionId::new("session_1").expect("test ID is non-empty");
    let thread_id = ThreadId::new("thread_1").expect("test ID is non-empty");
    broker.register(1, &first);
    broker.register(2, &second);
    broker.set_agent_interaction_capability(1, Some(approval_capability()));
    broker.set_agent_interaction_capability(2, Some(approval_capability()));
    broker.subscribe_session_thread(1, session_id.clone(), thread_id.clone(), 0);
    broker.subscribe_session_thread(2, session_id.clone(), thread_id.clone(), 0);

    broker.offer_agent_request(approval_request(&session_id, &thread_id));

    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 0);
    assert!(broker.is_agent_interaction_owner(
        1,
        &RequestId::new("approval_1").expect("test ID is non-empty")
    ));
}

#[test]
fn agent_request_is_reassigned_when_its_connection_closes() {
    let broker = UpdateBroker::default();
    let first = NotificationQueue::default();
    let second = NotificationQueue::default();
    let session_id = SessionId::new("session_1").expect("test ID is non-empty");
    let thread_id = ThreadId::new("thread_1").expect("test ID is non-empty");
    broker.register(1, &first);
    broker.register(2, &second);
    broker.set_agent_interaction_capability(1, Some(approval_capability()));
    broker.set_agent_interaction_capability(2, Some(approval_capability()));
    broker.subscribe_session_thread(1, session_id.clone(), thread_id.clone(), 0);
    broker.subscribe_session_thread(2, session_id.clone(), thread_id.clone(), 0);
    broker.offer_agent_request(approval_request(&session_id, &thread_id));
    first.drain();

    broker.unregister(1);

    let notifications = second.drain();
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0]["method"], "agent/request");
    assert!(broker.is_agent_interaction_owner(
        2,
        &RequestId::new("approval_1").expect("test ID is non-empty")
    ));
}

#[test]
fn agent_request_waits_until_a_matching_capability_subscribes() {
    let broker = UpdateBroker::default();
    let queue = NotificationQueue::default();
    let session_id = SessionId::new("session_1").expect("test ID is non-empty");
    let thread_id = ThreadId::new("thread_1").expect("test ID is non-empty");
    broker.register(1, &queue);
    broker.subscribe_session_thread(1, session_id.clone(), thread_id.clone(), 0);
    broker.offer_agent_request(approval_request(&session_id, &thread_id));
    assert_eq!(queue.len(), 0);

    broker.set_agent_interaction_capability(1, Some(approval_capability()));

    assert_eq!(queue.len(), 1);
}

fn approval_capability() -> AgentInteractionCapability {
    AgentInteractionCapability {
        version: 1,
        kinds: vec![AgentInteractionKind::Approval],
    }
}

fn approval_request(session_id: &SessionId, thread_id: &ThreadId) -> AgentRequestEnvelope {
    AgentRequestEnvelope {
        session_id: session_id.clone(),
        thread_id: thread_id.clone(),
        turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
        interaction: zeta_protocol::TurnInteraction {
            request_id: RequestId::new("approval_1").expect("test ID is non-empty"),
            item_id: None,
            request: AgentRequest::Approval {
                request: ActionApprovalRequest {
                    action_digest: "digest".into(),
                    policy_revision: "policy-1".into(),
                    capabilities: vec![ActionApprovalCapability {
                        kind: ActionApprovalCapabilityKind::Network,
                        scope: "api.example.test".into(),
                    }],
                    reason: "connect to the test service".into(),
                    sandbox_denial: None,
                },
            },
            deadline: None,
        },
    }
}

fn update(session_id: &SessionId, sequence: u64) -> SessionUpdateEnvelope {
    SessionUpdateEnvelope {
        session_id: session_id.clone(),
        durable_sequence: sequence,
        update: SessionUpdate::Committed {
            event: SessionEvent::SessionCreated {
                session_id: session_id.clone(),
                title: "task".into(),
                model: None,
            },
        },
    }
}

fn thread_update(
    session_id: &SessionId,
    thread_id: &ThreadId,
    sequence: u64,
) -> ThreadUpdateEnvelope {
    ThreadUpdateEnvelope {
        session_id: session_id.clone(),
        thread_id: thread_id.clone(),
        durable_sequence: sequence,
        stream_cursor: None,
        update: ThreadUpdate::Committed {
            event: ThreadEvent::TurnCompleted {
                thread_id: thread_id.clone(),
                turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
            },
        },
    }
}
