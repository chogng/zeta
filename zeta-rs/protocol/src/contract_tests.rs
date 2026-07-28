use super::*;
use serde_json::json;

#[test]
fn durable_thread_event_serializes_without_a_runtime_message_wrapper() {
    let event = ThreadEvent::TurnStarted {
        thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
        turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
    };

    assert_eq!(
        serde_json::to_value(event).unwrap(),
        json!({
            "type": "turnStarted",
            "threadId": "thread_1",
            "turnId": "turn_1"
        })
    );
}

#[test]
fn canonical_session_contains_thread_lineage_without_embedding_thread_history() {
    let session = Session {
        session_id: SessionId::new("session_1").expect("test ID is non-empty"),
        title: "task".into(),
        status: SessionStatus::Active,
        sequence: 2,
        threads: vec![
            SessionThread {
                thread_id: ThreadId::new("thread_root").expect("test ID is non-empty"),
                origin: ThreadOrigin::Root,
                status: SessionThreadStatus::Active,
            },
            SessionThread {
                thread_id: ThreadId::new("thread_child").expect("test ID is non-empty"),
                origin: ThreadOrigin::Fork {
                    parent_thread_id: ThreadId::new("thread_root").expect("test ID is non-empty"),
                    parent_sequence: 7,
                },
                status: SessionThreadStatus::Creating,
            },
        ],
    };

    assert_eq!(session.threads.len(), 2);
    assert_eq!(
        session.threads[1].origin,
        ThreadOrigin::Fork {
            parent_thread_id: ThreadId::new("thread_root").expect("test ID is non-empty"),
            parent_sequence: 7,
        }
    );
}

#[test]
fn live_update_uses_a_separate_stream_cursor_from_durable_sequence() {
    let update = ThreadUpdateEnvelope {
        session_id: SessionId::new("session_1").expect("test ID is non-empty"),
        thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
        durable_sequence: 4,
        stream_cursor: Some(StreamCursor {
            stream_instance_id: StreamInstanceId::new("stream_1").expect("test ID is non-empty"),
            sequence: 9,
        }),
        update: ThreadUpdate::ItemDelta {
            turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
            item_id: ItemId::new("item_1").expect("test ID is non-empty"),
            delta: ItemDelta::AgentMessage {
                text: "delta".into(),
            },
        },
    };

    assert_eq!(update.durable_sequence, 4);
    assert_eq!(update.stream_cursor.unwrap().sequence, 9);
}

#[test]
fn interaction_request_is_durable_without_connection_ownership() {
    let event = ThreadEvent::InteractionRequested {
        thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
        turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
        interaction: TurnInteraction {
            request_id: RequestId::new("request_1").expect("test ID is non-empty"),
            item_id: None,
            request: AgentRequest::UserInput {
                request: RequestUserInput {
                    questions: Vec::new(),
                },
            },
            deadline: Some(InteractionDeadline {
                expires_at_unix_ms: 1_234,
            }),
        },
    };

    assert_eq!(
        serde_json::to_value(event).unwrap(),
        json!({
            "type": "interactionRequested",
            "threadId": "thread_1",
            "turnId": "turn_1",
            "interaction": {
                "requestId": "request_1",
                "request": {"type": "userInput", "request": {"questions": []}},
                "deadline": {"expiresAtUnixMs": 1234}
            }
        })
    );
}

#[test]
fn readable_wait_state_excludes_the_owner_directed_request_payload() {
    let interaction = TurnInteraction {
        request_id: RequestId::new("request_1").expect("test ID is non-empty"),
        item_id: None,
        request: AgentRequest::UserInput {
            request: RequestUserInput {
                questions: vec![UserInputQuestion {
                    id: "secret-question".into(),
                    header: "Secret".into(),
                    question: "Do not broadcast this payload".into(),
                    options: Vec::new(),
                    allow_free_form: true,
                }],
            },
        },
        deadline: None,
    };

    assert_eq!(
        serde_json::to_value(interaction.pending_state()).unwrap(),
        json!({
            "requestId": "request_1",
            "kind": "userInput"
        })
    );
}

#[test]
fn approval_interaction_serializes_its_exact_policy_binding() {
    let interaction = TurnInteraction {
        request_id: RequestId::new("approval_1").unwrap(),
        item_id: None,
        request: AgentRequest::Approval {
            request: ActionApprovalRequest {
                action_digest: "a".repeat(64),
                policy_revision: "policy-7".into(),
                capabilities: vec![ActionApprovalCapability {
                    kind: ActionApprovalCapabilityKind::Network,
                    scope: "api.example.com".into(),
                }],
                reason: "network requires unsandboxed execution".into(),
            },
        },
        deadline: None,
    };

    assert_eq!(
        serde_json::to_value(&interaction).unwrap(),
        json!({
            "requestId": "approval_1",
            "request": {
                "type": "approval",
                "request": {
                    "actionDigest": "a".repeat(64),
                    "policyRevision": "policy-7",
                    "capabilities": [{
                        "kind": "network",
                        "scope": "api.example.com"
                    }],
                    "reason": "network requires unsandboxed execution"
                }
            }
        })
    );
    assert_eq!(
        serde_json::to_value(interaction.pending_state()).unwrap(),
        json!({
            "requestId": "approval_1",
            "kind": "approval"
        })
    );
    assert_eq!(
        serde_json::to_value(AgentResponse::Approval {
            response: ActionApprovalResponse {
                decision: ActionApprovalDecision::ApproveOnce,
            },
        })
        .unwrap(),
        json!({
            "type": "approval",
            "response": {"decision": "approveOnce"}
        })
    );
}

#[test]
fn user_input_supports_text_images_skills_and_mentions() {
    let input = [
        UserInput::Text {
            text: "hello".into(),
        },
        UserInput::Image {
            url: "https://example.test/image.png".into(),
        },
        UserInput::Skill {
            name: "review".into(),
            path: "/skills/review/SKILL.md".into(),
        },
        UserInput::Mention {
            name: "issues".into(),
            path: "app://issues".into(),
        },
    ];

    assert_eq!(input.len(), 4);
}

#[test]
fn tool_names_reject_ambiguous_or_provider_specific_syntax() {
    assert!(ToolName::new("request_user_input").is_ok());
    assert!(ToolName::new("namespace/tool").is_err());
    assert!(ToolName::new("").is_err());
}

#[test]
fn canonical_identifiers_reject_empty_construction_and_deserialization() {
    assert!(SessionId::new("").is_err());
    assert!(ThreadId::new("   ").is_err());
    assert!(CommandId::new("").is_err());
    assert!(ToolCallId::new("\n").is_err());
    assert!(serde_json::from_str::<SessionId>("\"\"").is_err());
    assert!(serde_json::from_str::<StreamInstanceId>("\"  \"").is_err());
}

#[test]
fn tool_contract_uses_validated_names_and_call_identifiers() {
    let name = ToolName::new("search").unwrap();
    let call_id = ToolCallId::new("tool_1").unwrap();
    let call = ToolCall {
        id: call_id,
        name,
        arguments: json!({"query": "zeta"}),
    };

    assert_eq!(serde_json::to_value(call).unwrap()["name"], "search");
}

#[test]
fn model_metadata_caps_auto_compaction_at_ninety_percent_of_context() {
    let mut model = ModelInfo::new(ModelId::new("zeta-large").unwrap(), "Zeta Large");
    model.context_window = ContextWindow::Known(100_000);
    model.auto_compact_token_limit = Some(95_000);

    assert_eq!(model.effective_auto_compact_token_limit(), Some(90_000));
}
