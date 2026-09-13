use crate::local_tools::local_policy_revision;
use agent::MultiAgentToolService;
use agent::SPAWN_AGENT_TOOL_NAME;
use serde_json::Value;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use ash_action_policy::ActionReviewRequest;
use ash_async_utils::CancellationToken;
use ash_core::CoreError;
use ash_core::InMemoryThreadStore;
use ash_core::ModelSelection;
use ash_core::ModelService;
use ash_core::SequenceExpectation;
use ash_core::SpawnAgentRequest;
use ash_core::StartThreadRequest;
use ash_core::StartTurnRequest;
use ash_core::ThreadController;
use ash_core::TurnExecutionBackend;
use ash_protocol::AgentCapabilityScope;
use ash_protocol::AgentContextMode;
use ash_protocol::AgentRoleSnapshot;
use ash_protocol::CommandId;
use ash_protocol::ContentPart;
use ash_protocol::DelegatedPolicyCeiling;
use ash_protocol::DelegatedTask;
use ash_protocol::DelegationId;
use ash_protocol::InputItem;
use ash_protocol::ModelRequest;
use ash_protocol::ModelResponse;
use ash_protocol::ResponseItem;
use ash_protocol::StopReason;
use ash_protocol::ThreadId;
use ash_protocol::ToolCall;
use ash_protocol::ToolCallId;
use ash_protocol::ToolName;
use ash_protocol::TurnStatus;
use ash_protocol::UserInput;

#[test]
fn recovered_spawn_starts_a_new_child_turn_once() {
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    let server = crate::AppServer::new(Arc::clone(&threads), Arc::new(TextModel));
    let parent = threads
        .start_thread(
            &ash_core::NoThreadWorktreeBinder,
            StartThreadRequest {
                agent_id: None,
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
                kind: ash_protocol::TurnKind::Coding,
                instructions: ash_prompts::AGENT_INSTRUCTIONS.freeze(),
                command_id: CommandId::new("start-parent").unwrap(),
                expected_sequence: SequenceExpectation::Exact(1),
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: ash_protocol::ApprovalMode::AskPermissions,
                tool_mode: ash_protocol::ToolMode::Direct,
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
            base_instructions: ash_prompts::AGENT_INSTRUCTIONS.freeze(),
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

struct SelectedModel(ash_protocol::ModelRef);
impl crate::model_catalog::ModelCatalog for SelectedModel {
    fn list(
        &self,
    ) -> Result<Vec<ash_app_server_protocol::protocol::model::ModelCatalogEntry>, CoreError> {
        Ok(Vec::new())
    }
    fn configured_default(&self) -> Result<Option<ash_protocol::ModelRef>, CoreError> {
        Ok(Some(self.0.clone()))
    }
}

struct AllowCoordination;
impl ash_core::ActionPolicyService for AllowCoordination {
    fn revision(&self) -> String {
        local_policy_revision().as_str().to_owned()
    }
    fn decide(
        &self,
        request: &ActionReviewRequest,
        _: &CancellationToken,
    ) -> Result<ash_action_policy::ExecutionDecision, CoreError> {
        assert_eq!(request.provenance().source_id(), SPAWN_AGENT_TOOL_NAME);
        Ok(ash_action_policy::ExecutionDecision::RunUnsandboxed {
            grant_id: ash_action_policy::GrantId::new("test-coordination"),
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
        let model = ash_protocol::ModelRef::new(
            ash_protocol::ProviderId::new(provider).unwrap(),
            ash_protocol::ModelId::new(name).unwrap(),
        );
        let expected =
            ash_models_manager::ModelInstructionCatalog::built_in().resolve(Some(&model));
        let ash_protocol::ModelInstructionSelection::Specialized { instructions, .. } = &expected
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
            local_policy_revision(),
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

struct NoopTurnBackend;

impl TurnExecutionBackend for NoopTurnBackend {
    fn start(
        &self,
        _: &ash_protocol::ThreadId,
        _: &ash_protocol::TurnId,
    ) -> Result<(), CoreError> {
        Ok(())
    }

    fn resume(
        &self,
        _: &ash_protocol::ThreadId,
        _: &ash_protocol::TurnId,
    ) -> Result<(), CoreError> {
        Ok(())
    }
}
