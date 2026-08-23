use super::*;
use zeta_async_utils::CancellationSource;
use zeta_core::AgentTreeLimits;
use zeta_core::CoreError;
use zeta_core::CreateSessionRequest;
use zeta_core::CreateSessionThreadRequest;
use zeta_core::InMemorySessionStore;
use zeta_core::InMemoryThreadStore;
use zeta_core::ModelSelection;
use zeta_core::ModelService;
use zeta_core::SequenceExpectation;
use zeta_core::StartTurnRequest;
use zeta_core::ThreadController;
use zeta_core::TurnExecutionBackend;
use zeta_protocol::CommandId;
use zeta_protocol::ContentPart;
use zeta_protocol::InputItem;
use zeta_protocol::ModelRequest;
use zeta_protocol::ModelResponse;
use zeta_protocol::ResponseItem;
use zeta_protocol::StopReason;
use zeta_protocol::ToolCallId;
use zeta_protocol::TurnStatus;
use zeta_protocol::UserInput;

#[test]
fn exposes_only_the_three_agent_coordination_tools() {
    let service = service();

    let names = service
        .definitions()
        .into_iter()
        .map(|definition| definition.name.to_string())
        .collect::<Vec<_>>();

    assert_eq!(
        names,
        vec![
            SPAWN_AGENT_TOOL_NAME,
            SEND_AGENT_MESSAGE_TOOL_NAME,
            WAIT_AGENT_TOOL_NAME,
        ]
    );
}

#[test]
fn prepares_agent_coordination_as_a_builtin_system_operation() {
    let service = service();
    let call = ToolCall {
        id: ToolCallId::new("spawn-call").unwrap(),
        name: ToolName::new(SPAWN_AGENT_TOOL_NAME).unwrap(),
        arguments: json!({"task": "review the change", "name": null}),
    };

    let review = service.prepare(&call).unwrap();

    assert_eq!(review.action().kind(), &ActionKind::SystemOperation);
    assert_eq!(review.provenance().source(), &ActionSource::BuiltInTool);
    assert_eq!(review.provenance().source_id(), SPAWN_AGENT_TOOL_NAME);
    assert!(matches!(
        review.sandbox(),
        SandboxCompatibility::NotApplicable { .. }
    ));
}

#[test]
fn refuses_execution_without_durable_turn_identity() {
    let service = service();
    let call = ToolCall {
        id: ToolCallId::new("spawn-call").unwrap(),
        name: ToolName::new(SPAWN_AGENT_TOOL_NAME).unwrap(),
        arguments: json!({"task": "review the change", "name": null}),
    };

    let error = service
        .execute(
            &call,
            &ToolAuthorization::UnsandboxedGrant {
                grant_id: zeta_action_policy::GrantId::new("test"),
            },
            &CancellationSource::new().token(),
        )
        .unwrap_err();

    assert!(error.to_string().contains("durable execution facts"));
}

#[test]
fn spawn_context_arguments_cover_selected_and_forked_modes() {
    let selected = spawn_context(Some(SpawnContextArguments {
        mode: SpawnContextMode::Selected,
        count: None,
        sources: Some(vec![SpawnContextSourceArguments {
            kind: SpawnContextSourceKind::Item,
            source_thread_id: "parent".into(),
            source_sequence: 7,
            item_id: Some("item-1".into()),
            checkpoint_id: None,
        }]),
    }))
    .unwrap();
    assert!(matches!(
        selected,
        AgentContextMode::Selected { sources } if sources.len() == 1
    ));
    let forked = spawn_context(Some(SpawnContextArguments {
        mode: SpawnContextMode::LastTurns,
        count: Some(3),
        sources: None,
    }))
    .unwrap();
    assert!(matches!(
        forked,
        AgentContextMode::ForkedPrefix {
            selection: ForkedAgentContext::LastTurns { count: 3 }
        }
    ));
}

#[test]
fn wait_arguments_map_to_durable_all_any_and_quorum_policies() {
    let all = wait_join_policy(&WaitArguments {
        delegation_id: Some("one".into()),
        delegation_ids: None,
        policy: Some(WaitPolicy::All),
        quorum: None,
        timeout_ms: None,
    })
    .unwrap();
    assert_eq!(all.0.unwrap().len(), 1);
    assert_eq!(all.1, AgentJoinPolicy::All);
    let any = wait_join_policy(&WaitArguments {
        delegation_id: None,
        delegation_ids: Some(vec!["one".into(), "two".into()]),
        policy: Some(WaitPolicy::Any),
        quorum: None,
        timeout_ms: Some(0),
    })
    .unwrap();
    assert_eq!(any.1, AgentJoinPolicy::Any);
    let quorum = wait_join_policy(&WaitArguments {
        delegation_id: None,
        delegation_ids: None,
        policy: Some(WaitPolicy::Quorum),
        quorum: Some(2),
        timeout_ms: None,
    })
    .unwrap();
    assert_eq!(quorum.1, AgentJoinPolicy::Quorum { count: 2 });
}

#[test]
fn recovered_spawn_starts_a_new_child_turn_once() {
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    let sessions = Arc::new(SessionCoordinator::with_store(
        Arc::new(InMemorySessionStore::default()),
        Arc::clone(&threads),
    ));
    let server = crate::AppServer::new(Arc::clone(&sessions), Arc::new(TextModel));
    let session = sessions
        .create_session(CreateSessionRequest {
            command_id: CommandId::new("create-session").unwrap(),
            title: "agent recovery".into(),
            model: None,
            workspace: None,
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
    let parent_turn = sessions
        .start_turn(
            &session.session_id,
            &parent.thread_id,
            StartTurnRequest {
                command_id: CommandId::new("start-parent").unwrap(),
                expected_sequence: SequenceExpectation::Exact(1),
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                resource_budget: None,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "delegate".into(),
                }],
            },
        )
        .unwrap();
    let spawned = server
        .multi_agent
        .spawn(SpawnAgentRequest {
            delegation_id: DelegationId::new("recover-child").unwrap(),
            session_id: session.session_id,
            parent_thread_id: parent.thread_id,
            parent_turn_id: parent_turn.turn_id,
            task: DelegatedTask {
                title: "child".into(),
                instructions: "finish independently".into(),
            },
            role: AgentRoleSnapshot {
                name: "general".into(),
                instructions: "Return one concise answer.".into(),
                model: None,
            },
            inheritance: AgentContextMode::Fresh,
            policy_ceiling: DelegatedPolicyCeiling {
                policy_revision: "test-policy-v1".into(),
            },
            capability_scope: DelegatedCapabilityScope {
                tools: Vec::new(),
                skills: Vec::new(),
            },
        })
        .unwrap();

    assert_eq!(server.resume_recovered_agent_coordinations().unwrap(), 1);
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let child = threads.read_thread(&spawned.child_thread_id).unwrap();
        if child.turns[0].status == TurnStatus::Completed {
            break;
        }
        assert!(Instant::now() < deadline, "child Turn did not complete");
        std::thread::yield_now();
    }
    assert_eq!(server.resume_recovered_agent_coordinations().unwrap(), 0);
}

struct TextModel;

impl ModelService for TextModel {
    fn invoke(
        &self,
        _: ModelSelection<'_>,
        request: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        let prompt = request
            .input
            .iter()
            .find_map(|input| match input {
                InputItem::Message(message) => message.content.iter().find_map(|content| {
                    let ContentPart::Text(text) = content else {
                        return None;
                    };
                    Some(text.clone())
                }),
                InputItem::ToolResult(_) => None,
            })
            .unwrap_or_else(|| "done".into());
        Ok(ModelResponse {
            output: vec![ResponseItem::Text(prompt)],
            usage: None,
            stop_reason: StopReason::Completed,
        })
    }
}

fn service() -> MultiAgentToolService {
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    let sessions = Arc::new(SessionCoordinator::with_store(
        Arc::new(InMemorySessionStore::default()),
        threads,
    ));
    MultiAgentToolService::new(
        Arc::new(MultiAgentCoordinator::new(
            Arc::clone(&sessions),
            AgentTreeLimits::default(),
        )),
        sessions,
        Arc::new(NoopTurnBackend),
    )
}

struct NoopTurnBackend;

impl TurnExecutionBackend for NoopTurnBackend {
    fn start(
        &self,
        _: &zeta_protocol::ThreadId,
        _: &zeta_protocol::TurnId,
    ) -> Result<(), CoreError> {
        Ok(())
    }

    fn resume(
        &self,
        _: &zeta_protocol::ThreadId,
        _: &zeta_protocol::TurnId,
    ) -> Result<(), CoreError> {
        Ok(())
    }
}
