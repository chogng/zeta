use super::*;
use crate::server::notification_queue::NotificationQueue;
use zeta_app_server_protocol::protocol::common::AgentInteractionCapability;
use zeta_app_server_protocol::protocol::language::LanguageServerStateDto;
use zeta_app_server_protocol::protocol::language::LanguageServerStateNotification;
use zeta_protocol::ActionApprovalCapability;
use zeta_protocol::ActionApprovalCapabilityKind;
use zeta_protocol::ActionApprovalRequest;
use zeta_protocol::AgentInteractionKind;
use zeta_protocol::AgentRequest;
use zeta_protocol::AgentRequestEnvelope;
use zeta_protocol::DynamicToolCall;
use zeta_protocol::RequestId;
use zeta_protocol::SessionEvent;
use zeta_protocol::SessionUpdate;
use zeta_protocol::ThreadEvent;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadUpdate;
use zeta_protocol::ThreadUpdateEnvelope;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolName;
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
        workspace_folder_id: None,
        paths: vec!["src/lib.rs".into()],
    });

    let notifications = queue.drain();
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0]["method"], "fs/changed");
    assert_eq!(notifications[0]["params"]["paths"][0], "src/lib.rs");
}

#[test]
fn profile_broker_shares_sessions_but_isolates_workspace_notifications() {
    let first_scope = UpdateBroker::default();
    let second_scope = first_scope.fork_scope();
    let first = NotificationQueue::default();
    let second = NotificationQueue::default();
    let first_connection = first_scope.allocate_connection_id();
    let second_connection = second_scope.allocate_connection_id();
    let session_id = SessionId::new("session_1").expect("test ID is non-empty");
    first_scope.register(first_connection, &first);
    second_scope.register(second_connection, &second);
    first_scope.subscribe_session(first_connection, session_id.clone(), 0);
    second_scope.subscribe_session(second_connection, session_id.clone(), 0);

    first_scope.publish_session(&session_id, &[update(&session_id, 1)]);
    first_scope.publish_fs_changed(FsChanged::PathsChanged {
        workspace_folder_id: None,
        paths: vec!["first/src/lib.rs".into()],
    });

    assert_ne!(first_connection, second_connection);
    assert_eq!(first.len(), 2);
    assert_eq!(second.len(), 1);
    assert_eq!(second.drain()[0]["method"], "session/update");
}

#[test]
fn profile_broker_fans_out_marketplace_generations_across_workspace_scopes() {
    let first_scope = UpdateBroker::default();
    let second_scope = first_scope.fork_scope();
    let first = NotificationQueue::default();
    let second = NotificationQueue::default();
    first_scope.register(first_scope.allocate_connection_id(), &first);
    second_scope.register(second_scope.allocate_connection_id(), &second);

    let instance_id = first_scope.marketplace_instance_id().to_owned();
    assert_eq!(second_scope.marketplace_instance_id(), instance_id);
    assert_eq!(first_scope.marketplace_generation(), 1);
    assert_eq!(first_scope.publish_marketplace_changed(), 2);
    assert_eq!(second_scope.publish_marketplace_changed(), 3);
    assert_eq!(first_scope.marketplace_generation(), 3);
    assert_eq!(second_scope.marketplace_generation(), 3);

    for queue in [&first, &second] {
        let notifications = queue.drain();
        assert_eq!(notifications.len(), 2);
        assert_eq!(notifications[0]["method"], "marketplace/changed");
        assert_eq!(notifications[0]["params"]["instanceId"], instance_id);
        assert_eq!(notifications[0]["params"]["generation"], 2);
        assert_eq!(notifications[1]["params"]["generation"], 3);
    }
}

#[test]
fn profile_marketplace_broker_deduplicates_one_manager_commit_seen_by_multiple_scopes() {
    let first_scope = UpdateBroker::default();
    let second_scope = first_scope.fork_scope();
    let queue = NotificationQueue::default();
    first_scope.register(first_scope.allocate_connection_id(), &queue);

    assert_eq!(
        first_scope.publish_marketplace_manager_changed("manager-a", 2),
        2
    );
    assert_eq!(
        second_scope.publish_marketplace_manager_changed("manager-a", 2),
        2
    );
    assert_eq!(
        second_scope.publish_marketplace_manager_changed("manager-a", 3),
        3
    );
    assert_eq!(
        first_scope.publish_marketplace_manager_changed("manager-b", 2),
        4
    );

    let notifications = queue.drain();
    assert_eq!(notifications.len(), 3);
    assert_eq!(notifications[0]["params"]["generation"], 2);
    assert_eq!(notifications[1]["params"]["generation"], 3);
    assert_eq!(notifications[2]["params"]["generation"], 4);
}

#[test]
fn broker_fans_out_language_server_lifecycle_without_a_subscription() {
    let broker = UpdateBroker::default();
    let queue = NotificationQueue::default();
    broker.register(1, &queue);

    broker.publish_language_server_state(LanguageServerStateNotification {
        workspace_folder_id: None,
        server: "rust-analyzer".into(),
        state: LanguageServerStateDto::BackingOff {
            attempt: 2,
            retry_after_millis: 1_500,
        },
    });

    let notifications = queue.drain();
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0]["method"], "language/serverState");
    assert_eq!(notifications[0]["params"]["server"], "rust-analyzer");
    assert_eq!(notifications[0]["params"]["state"]["type"], "backingOff");
    assert_eq!(notifications[0]["params"]["state"]["attempt"], 2);
    assert_eq!(
        notifications[0]["params"]["state"]["retryAfterMillis"],
        1_500
    );
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
    broker.bind_session_scope(session_id.clone());
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
    broker.bind_session_scope(session_id.clone());
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
fn delivered_dynamic_tool_is_not_reassigned_when_its_owner_closes() {
    let broker = UpdateBroker::default();
    let first = NotificationQueue::default();
    let second = NotificationQueue::default();
    let session_id = SessionId::new("session_1").unwrap();
    let thread_id = ThreadId::new("thread_1").unwrap();
    broker.bind_session_scope(session_id.clone());
    broker.register(1, &first);
    broker.register(2, &second);
    broker.set_agent_interaction_capability(1, Some(dynamic_tool_capability()));
    broker.set_agent_interaction_capability(2, Some(dynamic_tool_capability()));
    broker.subscribe_session_thread(1, session_id.clone(), thread_id.clone(), 0);
    broker.subscribe_session_thread(2, session_id.clone(), thread_id.clone(), 0);
    broker.offer_agent_request(dynamic_tool_request(&session_id, &thread_id));
    first.drain();

    let lost = broker.unregister(1);

    assert_eq!(lost.len(), 1);
    assert_eq!(lost[0].interaction.request_id.as_str(), "dynamic_1");
    assert!(second.drain().is_empty());
}

#[test]
fn dynamic_tool_is_delivered_only_to_a_connection_hosting_that_tool_name() {
    let broker = UpdateBroker::default();
    let other = NotificationQueue::default();
    let owner = NotificationQueue::default();
    let session_id = SessionId::new("session_1").unwrap();
    let thread_id = ThreadId::new("thread_1").unwrap();
    broker.bind_session_scope(session_id.clone());
    broker.register(1, &other);
    broker.register(2, &owner);
    broker.set_agent_interaction_capability(1, Some(dynamic_tool_capability_for("other_tool")));
    broker.set_agent_interaction_capability(2, Some(dynamic_tool_capability()));
    broker.subscribe_session_thread(1, session_id.clone(), thread_id.clone(), 0);
    broker.subscribe_session_thread(2, session_id.clone(), thread_id.clone(), 0);

    broker.offer_agent_request(dynamic_tool_request(&session_id, &thread_id));

    assert!(other.drain().is_empty());
    assert_eq!(owner.drain().len(), 1);
}

#[test]
fn agent_request_waits_until_a_matching_capability_subscribes() {
    let broker = UpdateBroker::default();
    let queue = NotificationQueue::default();
    let session_id = SessionId::new("session_1").expect("test ID is non-empty");
    let thread_id = ThreadId::new("thread_1").expect("test ID is non-empty");
    broker.bind_session_scope(session_id.clone());
    broker.register(1, &queue);
    broker.subscribe_session_thread(1, session_id.clone(), thread_id.clone(), 0);
    broker.offer_agent_request(approval_request(&session_id, &thread_id));
    assert_eq!(queue.len(), 0);

    broker.set_agent_interaction_capability(1, Some(approval_capability()));

    assert_eq!(queue.len(), 1);
}

#[test]
fn agent_request_is_not_assigned_to_a_foreign_workspace_scope() {
    let owning_scope = UpdateBroker::default();
    let foreign_scope = owning_scope.fork_scope();
    let queue = NotificationQueue::default();
    let session_id = SessionId::new("session_1").expect("test ID is non-empty");
    let thread_id = ThreadId::new("thread_1").expect("test ID is non-empty");
    let connection_id = foreign_scope.allocate_connection_id();
    owning_scope.bind_session_scope(session_id.clone());
    foreign_scope.register(connection_id, &queue);
    foreign_scope.set_agent_interaction_capability(connection_id, Some(approval_capability()));
    foreign_scope.subscribe_session_thread(connection_id, session_id.clone(), thread_id.clone(), 0);

    owning_scope.offer_agent_request(approval_request(&session_id, &thread_id));

    assert!(queue.drain().is_empty());
}

fn approval_capability() -> AgentInteractionCapability {
    AgentInteractionCapability {
        version: 1,
        kinds: vec![AgentInteractionKind::Approval],
        dynamic_tools: None,
    }
}

fn dynamic_tool_capability() -> AgentInteractionCapability {
    dynamic_tool_capability_for("client_lookup")
}

fn dynamic_tool_capability_for(name: &str) -> AgentInteractionCapability {
    AgentInteractionCapability {
        version: 1,
        kinds: vec![AgentInteractionKind::DynamicTool],
        dynamic_tools: Some(vec![ToolName::new(name).unwrap()]),
    }
}

fn dynamic_tool_request(session_id: &SessionId, thread_id: &ThreadId) -> AgentRequestEnvelope {
    AgentRequestEnvelope {
        session_id: session_id.clone(),
        thread_id: thread_id.clone(),
        turn_id: TurnId::new("turn_1").unwrap(),
        interaction: zeta_protocol::TurnInteraction {
            request_id: RequestId::new("dynamic_1").unwrap(),
            item_id: None,
            request: AgentRequest::DynamicTool {
                call: DynamicToolCall {
                    call_id: ToolCallId::new("call_1").unwrap(),
                    name: ToolName::new("client_lookup").unwrap(),
                    definition_digest: "a".repeat(64),
                    arguments: serde_json::json!({"query": "zeta"}),
                },
            },
            deadline: None,
        },
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
                workspace: None,
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
