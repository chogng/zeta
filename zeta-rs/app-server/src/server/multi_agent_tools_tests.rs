use super::*;
use zeta_async_utils::CancellationSource;
use zeta_core::AgentTreeLimits;
use zeta_core::CoreError;
use zeta_core::InMemoryThreadStore;
use zeta_core::ModelSelection;
use zeta_core::ModelService;
use zeta_core::SequenceExpectation;
use zeta_core::StartThreadRequest;
use zeta_core::StartTurnRequest;
use zeta_core::ThreadController;
use zeta_core::TurnExecutionBackend;
use zeta_protocol::AgentCapabilityScope;
use zeta_protocol::AgentRoleSnapshot;
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
fn spawn_agent_describes_the_built_in_issue_role() {
    let definition = service()
        .definitions()
        .into_iter()
        .find(|definition| definition.name.as_str() == SPAWN_AGENT_TOOL_NAME)
        .unwrap();
    let description = definition.parameters["properties"]["agent"]["description"]
        .as_str()
        .unwrap();

    assert!(description.contains("issue: Coordinates one or more GitHub issues"));
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
fn wait_timeout_returns_a_durable_waiting_join_without_losing_the_delegation() {
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    let coordinator = Arc::new(MultiAgentCoordinator::new(
        Arc::clone(&threads),
        AgentTreeLimits::default(),
    ));
    let parent = threads
        .start_thread(
            &zeta_core::NoThreadWorktreeBinder,
            StartThreadRequest {
                agent: None,
                command_id: CommandId::new("timeout-parent").unwrap(),
                title: "parent".into(),
            },
        )
        .unwrap();
    let parent_turn = threads
        .start_turn(
            &parent.thread_id,
            StartTurnRequest {
                kind: zeta_protocol::TurnKind::Coding,
                instructions: zeta_prompts::AGENT_INSTRUCTIONS.freeze(),
                command_id: CommandId::new("timeout-turn").unwrap(),
                expected_sequence: SequenceExpectation::Exact(1),
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                tool_mode: zeta_protocol::ToolMode::Direct,
                tool_profile: None,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "delegate".into(),
                }],
            },
        )
        .unwrap();
    let spawned = coordinator
        .spawn(SpawnAgentRequest {
            base_instructions: zeta_prompts::AGENT_INSTRUCTIONS.freeze(),
            delegation_id: DelegationId::new("timeout-child").unwrap(),
            session_id: parent.session_id.clone(),
            parent_thread_id: parent.thread_id.clone(),
            parent_turn_id: parent_turn.turn_id,
            task: DelegatedTask {
                title: "child".into(),
                instructions: "keep working".into(),
            },
            role: Some(AgentRoleSnapshot {
                name: "general".into(),
                instructions: "Return one answer.".into(),
                model: None,
                definition: None,
            }),
            inheritance: AgentContextMode::Fresh,
            policy_ceiling: DelegatedPolicyCeiling {
                policy_revision: "test-policy-v1".into(),
            },
            capability_scope: AgentCapabilityScope {
                tools: Vec::new(),
                delegation_tools: Vec::new(),
                skills: Vec::new(),
            },
        })
        .unwrap();
    let service = MultiAgentToolService::new(
        Arc::clone(&coordinator),
        Arc::clone(&threads),
        Arc::new(NoopTurnBackend),
    );

    let output = service
        .wait_for_join(
            &parent.thread_id,
            AgentJoinId::new("timeout-join").unwrap(),
            Some(vec![spawned.delegation_id]),
            AgentJoinPolicy::All,
            Duration::ZERO,
            &CancellationSource::new().token(),
        )
        .unwrap();

    let ToolExecutionOutput::Success(output) = output else {
        panic!("wait timeout must return a successful waiting projection")
    };
    let output: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(output["status"], "waiting");
    let parent = threads.read_thread(&parent.thread_id).unwrap();
    assert_eq!(parent.agent_joins.len(), 1);
    assert_eq!(
        parent.agent_joins.values().next().unwrap().status,
        AgentJoinStatus::Waiting
    );
}

#[test]
fn recovered_spawn_starts_a_new_child_turn_once() {
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    let server = crate::AppServer::new(Arc::clone(&threads), Arc::new(TextModel));
    let parent = threads
        .start_thread(
            &zeta_core::NoThreadWorktreeBinder,
            StartThreadRequest {
                agent: None,
                command_id: CommandId::new("create-parent").unwrap(),
                title: "parent".into(),
            },
        )
        .unwrap();
    let parent_turn = threads
        .start_turn(
            &parent.thread_id,
            StartTurnRequest {
                kind: zeta_protocol::TurnKind::Coding,
                instructions: zeta_prompts::AGENT_INSTRUCTIONS.freeze(),
                command_id: CommandId::new("start-parent").unwrap(),
                expected_sequence: SequenceExpectation::Exact(1),
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                tool_mode: zeta_protocol::ToolMode::Direct,
                tool_profile: None,
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
            base_instructions: zeta_prompts::AGENT_INSTRUCTIONS.freeze(),
            delegation_id: DelegationId::new("recover-child").unwrap(),
            session_id: parent.session_id.clone(),
            parent_thread_id: parent.thread_id,
            parent_turn_id: parent_turn.turn_id,
            task: DelegatedTask {
                title: "child".into(),
                instructions: "finish independently".into(),
            },
            role: Some(AgentRoleSnapshot {
                name: "general".into(),
                instructions: "Return one concise answer.".into(),
                model: None,
                definition: None,
            }),
            inheritance: AgentContextMode::Fresh,
            policy_ceiling: DelegatedPolicyCeiling {
                policy_revision: "test-policy-v1".into(),
            },
            capability_scope: AgentCapabilityScope {
                tools: Vec::new(),
                delegation_tools: Vec::new(),
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
            billing: None,
            stop_reason: StopReason::Completed,
        })
    }
}

fn service() -> MultiAgentToolService {
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    MultiAgentToolService::new(
        Arc::new(MultiAgentCoordinator::new(
            Arc::clone(&threads),
            AgentTreeLimits::default(),
        )),
        threads,
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

struct GuidanceModel(std::sync::mpsc::Sender<ModelRequest>);

impl ModelService for GuidanceModel {
    fn invoke(
        &self,
        _: ModelSelection<'_>,
        request: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        self.0.send(request.clone()).unwrap();
        let worker = request.input.iter().any(|item| {
            matches!(item, InputItem::Message(message) if message.content.iter().any(|part| matches!(part, ContentPart::Text(text) if text == "worker task")))
        });
        let spawn_returned = request.input.iter().any(|item| matches!(item, InputItem::ToolResult(result) if result.name.as_str() == SPAWN_AGENT_TOOL_NAME));
        let output = if worker || spawn_returned {
            ResponseItem::Text("verified".into())
        } else {
            ResponseItem::ToolCall(ToolCall {
                id: ToolCallId::new("initial-guidance-spawn").unwrap(),
                name: ToolName::new(SPAWN_AGENT_TOOL_NAME).unwrap(),
                arguments: json!({"task":"worker task", "agent":{"type":"default"}}),
            })
        };
        Ok(ModelResponse {
            output: vec![output],
            usage: None,
            billing: None,
            stop_reason: StopReason::Completed,
        })
    }
}

struct SelectedModel(zeta_protocol::ModelRef);
impl crate::model_catalog::ModelCatalog for SelectedModel {
    fn list(
        &self,
    ) -> Result<Vec<zeta_app_server_protocol::protocol::model::ModelCatalogEntry>, CoreError> {
        Ok(Vec::new())
    }
    fn configured_default(&self) -> Result<Option<zeta_protocol::ModelRef>, CoreError> {
        Ok(Some(self.0.clone()))
    }
}

struct AllowCoordination;
impl zeta_core::ActionPolicyService for AllowCoordination {
    fn revision(&self) -> String {
        local_policy_revision().as_str().to_owned()
    }
    fn decide(
        &self,
        request: &ActionReviewRequest,
        _: &CancellationToken,
    ) -> Result<zeta_action_policy::ExecutionDecision, CoreError> {
        assert_eq!(request.provenance().source_id(), SPAWN_AGENT_TOOL_NAME);
        Ok(zeta_action_policy::ExecutionDecision::RunUnsandboxed {
            grant_id: zeta_action_policy::GrantId::new("test-coordination"),
        })
    }
}

#[test]
fn built_in_model_guidance_reaches_rpc_roots_and_default_workers_through_tool_execution() {
    for (provider, name) in [
        ("openai", "gpt-6-astra"),
        ("anthropic", "claude-sonnet-4-20250514"),
        ("google", "gemini-3.6-flash"),
        ("deepseek", "deepseek-v4-pro"),
    ] {
        let model = zeta_protocol::ModelRef::new(
            zeta_protocol::ProviderId::new(provider).unwrap(),
            zeta_protocol::ModelId::new(name).unwrap(),
        );
        let expected =
            zeta_models_manager::ModelInstructionCatalog::built_in().resolve(Some(&model));
        let zeta_protocol::ModelInstructionSelection::Specialized { instructions, .. } = &expected
        else {
            panic!("built-in guidance missing");
        };
        let threads = Arc::new(ThreadController::with_store(Arc::new(
            InMemoryThreadStore::default(),
        )));
        let (sender, receiver) = std::sync::mpsc::channel();
        let server = crate::AppServer::new(threads.clone(), Arc::new(GuidanceModel(sender)));
        // Defer only child scheduling so this test can inspect the durable spawn before running it.
        let service = MultiAgentToolService::new(
            server.multi_agent.clone(),
            threads.clone(),
            Arc::new(NoopTurnBackend),
        )
        .with_model_instructions(server.model_instructions.clone());
        let mut server = server.with_tool_service(Arc::new(service), Arc::new(AllowCoordination));
        server.model_catalog = Arc::new(SelectedModel(model.clone()));
        let mut connection = server.connection();
        let mut id = 0;
        let mut call = |method: &str, params: Value| -> Value {
            id += 1;
            let response: Value = serde_json::from_str(&server.handle_json(
                &mut connection,
                &json!({"jsonrpc":"2.0", "id":id, "method":method, "params":params}).to_string(),
            ))
            .unwrap();
            assert!(response.get("error").is_none(), "{response}");
            response["result"].clone()
        };
        call(
            "initialize",
            json!({"clientInfo":{"name":"test", "version":"1"}, "capabilities":{}}),
        );
        let created = call(
            "session/create",
            json!({"commandId":"initial-guidance-root", "title":"guided root", "agent":{"type":"default"}}),
        );
        let session = created["session"]["sessionId"].as_str().unwrap();
        call(
            "session/request",
            json!({"commandId":"initial-guidance-turn", "sessionId":session, "request":{"type":"startTurn", "threadId":session, "expectedSequence":1, "input":[{"type":"text", "text":"start a worker"}]}}),
        );
        let first = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        let continued = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        let parent_id = ThreadId::new(session).unwrap();
        let parent = threads.read_thread(&parent_id).unwrap();
        let delegation = parent.delegations.values().next().unwrap();
        let child_id = delegation.child_thread_id.as_ref().unwrap();
        let child = threads.read_thread(child_id).unwrap();
        assert!(child.agent_configuration().unwrap().role.is_none());
        assert_eq!(
            parent.turns[0]
                .instructions
                .as_ref()
                .unwrap()
                .model_guidance(),
            Some(&expected)
        );
        assert_eq!(
            child.turns[0]
                .instructions
                .as_ref()
                .unwrap()
                .model_guidance(),
            Some(&expected)
        );
        assert_eq!(child.turns[0].model, Some(model));
        server
            .turn_executor_snapshot()
            .start(child_id, &child.turns[0].turn_id)
            .unwrap();
        let worker = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        for request in [first, continued, worker] {
            let body = request.instructions.unwrap();
            assert_eq!(body.matches(instructions.body.trim()).count(), 1);
            assert_eq!(body.matches("## Shared working rules").count(), 1);
            assert_eq!(body.matches("## Tool permissions").count(), 1);
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while [parent_id.clone(), child_id.clone()]
            .iter()
            .any(|id| threads.read_thread(id).unwrap().turns[0].status != TurnStatus::Completed)
        {
            assert!(Instant::now() < deadline, "guided turns must complete");
            std::thread::yield_now();
        }
    }
}
