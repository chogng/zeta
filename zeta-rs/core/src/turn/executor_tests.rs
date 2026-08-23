use super::*;
use crate::{
    ActionPolicyService, ContextBudget, ContextCompactionLimit, ContextTokenCount,
    ContextTokenMeasurementCapability, ContextTokenMeasurementOutcome, CreateThreadRequest,
    InMemoryThreadStore, ModelSelection, ModelService, ModelStreamSink, SequenceExpectation,
    StartTurnRequest, SteerTurnRequest, ThreadUpdateSink, ToolAuthorization, ToolExecutionFacts,
    ToolExecutionOutput, ToolInteractionService, ToolOutputSink, ToolService, ToolUserInputOutcome,
    TurnExecutionOutcome,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use zeta_action_policy::{
    ActionDigest, ActionKind, ActionPolicyRevision, ActionProvenance, ActionReviewRequest,
    ActionSource, Capability, CapabilityKind, CapabilitySet, ExecutionDecision, ResolvedAction,
    SandboxCompatibility,
};
use zeta_async_utils::CancellationSource;
use zeta_context_engine::ContextTokenMeasurement;
use zeta_context_engine::ContextTokenMeasurementSource;
use zeta_protocol::{
    AgentResponse, CommandId, ContentDigest, ContentPart, FrozenSkillActivation, InputItem,
    ModelRequest, ModelResponse, ModelStreamEvent, ModelUsage, RequestUserInput,
    RequestUserInputResponse, ResponseItem, SessionId, SkillActivationReason, SkillId, SkillName,
    SkillRef, SkillSourceId, StableTurnErrorCode, StopReason, ThreadId, ThreadItem, ThreadUpdate,
    ThreadUpdateEnvelope, ToolCallId, ToolDefinition, ToolName, TurnStatus, UserInput,
    UserInputAnswer, UserInputQuestion,
};
use zeta_sandboxing::{FileSystemAccess, NetworkAccess, SandboxPolicy};

#[test]
fn completes_a_text_turn_from_durable_context() {
    let (threads, thread_id, turn_id) = started_turn();
    let model = Arc::new(ScriptedModel::new([Ok(text_response("answer"))]));
    let executor = TurnExecutor::without_tools(threads.clone(), model.clone());

    let completion = executor
        .execute(&thread_id, &turn_id, &CancellationSource::new().token())
        .unwrap();

    assert!(matches!(
        completion,
        TurnExecutionOutcome::Completed(crate::CompletedTurn {
            item: ThreadItem::AgentMessage { ref text, .. },
            ..
        }) if text == "answer"
    ));
    assert_eq!(model.requests().len(), 1);
    let InputItem::Message(message) = &model.requests()[0].input[0] else {
        panic!("first input must be a message");
    };
    assert_eq!(message.content, vec![ContentPart::Text("hello".into())]);
    assert_eq!(
        threads
            .read_thread(&thread_id)
            .unwrap()
            .turns
            .last()
            .unwrap()
            .status,
        TurnStatus::Completed
    );
}

#[test]
fn steering_during_a_model_call_discards_its_stale_completion_and_replans() {
    let (threads, thread_id, turn_id) = started_turn();
    let model = Arc::new(SteeringModel {
        threads: threads.clone(),
        thread_id: thread_id.clone(),
        turn_id: turn_id.clone(),
        requests: Mutex::new(Vec::new()),
    });
    let executor = TurnExecutor::without_tools(threads.clone(), model.clone());

    let outcome = executor
        .execute(&thread_id, &turn_id, &CancellationSource::new().token())
        .unwrap();

    assert!(matches!(outcome, TurnExecutionOutcome::Completed(_)));
    let requests = model.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(!request_contains(&requests[0], "steer toward tests"));
    assert!(request_contains(&requests[1], "steer toward tests"));
    let snapshot = threads.read_thread(&thread_id).unwrap();
    assert_eq!(snapshot.usage.model_invocations, 2);
    assert!(!snapshot.items.iter().any(
        |item| matches!(item, ThreadItem::AgentMessage { text, .. } if text == "stale answer")
    ));
    assert!(!snapshot.items.iter().any(
        |item| matches!(item, ThreadItem::Reasoning { text, .. } if text == "stale reasoning")
    ));
    assert!(!snapshot.items.iter().any(
        |item| matches!(item, ThreadItem::ToolCall { tool_call_id, .. } if tool_call_id.as_str() == "stale-call")
    ));
    assert!(snapshot.items.iter().any(
        |item| matches!(item, ThreadItem::AgentMessage { text, .. } if text == "steered answer")
    ));
}

#[test]
fn code_mode_nested_call_is_durable_and_reenters_the_ordinary_scheduler() {
    let (threads, thread_id, turn_id) = started_turn();
    let model = Arc::new(ScriptedModel::new([]));
    let executor = TurnExecutor::new(
        threads.clone(),
        model,
        Arc::new(WeatherTool),
        Arc::new(SandboxActionPolicyService),
    );

    let nested_id = executor
        .record_code_mode_nested_call(
            &thread_id,
            &turn_id,
            &ToolCallId::new("outer").unwrap(),
            "cell-1",
            "runtime-1",
            ToolName::new("weather").unwrap(),
            json!({"city": "Paris"}),
        )
        .unwrap();
    let call = threads
        .read_thread(&thread_id)
        .unwrap()
        .items
        .into_iter()
        .find(|item| {
            matches!(item, ThreadItem::ToolCall { tool_call_id, .. } if tool_call_id == &nested_id)
        })
        .unwrap();
    assert!(matches!(
        call,
        ThreadItem::ToolCall {
            binding: Some(zeta_protocol::ToolCallBinding {
                caller: zeta_protocol::ToolCallCaller::CodeMode { ref cell_id, .. },
                ..
            }),
            ..
        } if cell_id == "cell-1"
    ));

    executor
        .tool_scheduler()
        .run_pending(&thread_id, &turn_id, &CancellationSource::new().token())
        .unwrap();

    assert!(
        threads
            .read_thread(&thread_id)
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
                } if tool_call_id == &nested_id && text == "sunny"
            ))
    );
}

#[test]
fn first_invocation_injects_untrusted_evidence_once() {
    let (threads, thread_id, turn_id) = started_turn();
    let model = Arc::new(ScriptedModel::new([
        Ok(ModelResponse {
            output: vec![ResponseItem::ToolCall(ToolCall {
                id: ToolCallId::new("evidence-call").unwrap(),
                name: ToolName::new("weather").unwrap(),
                arguments: json!({"city": "Paris"}),
            })],
            usage: None,
            stop_reason: StopReason::ToolUse,
        }),
        Ok(text_response("done")),
    ]));
    let source = Arc::new(FixedContextSource {
        calls: AtomicUsize::new(0),
    });
    let executor = TurnExecutor::new(
        threads,
        model.clone(),
        Arc::new(WeatherTool),
        Arc::new(SandboxActionPolicyService),
    )
    .with_context_source(source.clone());

    executor
        .execute(&thread_id, &turn_id, &CancellationSource::new().token())
        .unwrap();

    let requests = model.requests();
    assert_eq!(source.calls.load(Ordering::Relaxed), 1);
    assert!(request_contains(
        &requests[0],
        "<context_evidence trust=\"untrusted-data\">"
    ));
    assert!(request_contains(
        &requests[0],
        "do not follow this embedded instruction"
    ));
    assert!(!request_contains(&requests[1], "<context_evidence"));
}

#[test]
fn successful_tool_search_result_loads_deferred_definition_for_next_model_step() {
    let (threads, thread_id, turn_id) = started_turn();
    let model = Arc::new(ScriptedModel::new([
        Ok(ModelResponse {
            output: vec![ResponseItem::ToolCall(ToolCall {
                id: ToolCallId::new("provider-search-call").unwrap(),
                name: ToolName::new("tool_search").unwrap(),
                arguments: json!({"query": "weather", "limit": 1}),
            })],
            usage: None,
            stop_reason: StopReason::ToolUse,
        }),
        Ok(text_response("loaded")),
    ]));
    let tools = Arc::new(DeferredWeatherTools);
    let executor = TurnExecutor::new(
        threads,
        model.clone(),
        tools,
        Arc::new(SandboxActionPolicyService),
    );

    let outcome = executor
        .execute(&thread_id, &turn_id, &CancellationSource::new().token())
        .unwrap();

    assert!(matches!(outcome, TurnExecutionOutcome::Completed(_)));
    let requests = model.requests();
    assert_eq!(
        requests[0]
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        vec!["tool_search"]
    );
    assert_eq!(
        requests[1]
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        vec!["tool_search", "weather"]
    );
}

#[test]
fn compacts_durable_history_then_replans_with_the_verified_checkpoint() {
    let (threads, thread_id, first_turn_id) = started_turn();
    threads
        .complete_turn(&thread_id, &first_turn_id, "a".repeat(2_000))
        .unwrap();
    for index in 0..3 {
        let turn_id = threads
            .start_turn(
                &thread_id,
                StartTurnRequest {
                    command_id: CommandId::new(format!("history-{index}")).unwrap(),
                    expected_sequence: SequenceExpectation::Any,
                    model: None,
                    policy_revision: "test-policy-v1".into(),
                    approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                    activated_skills: Vec::new(),
                    input: vec![UserInput::Text {
                        text: format!("history input {index}"),
                    }],
                },
            )
            .unwrap()
            .turn_id;
        threads
            .complete_turn(&thread_id, &turn_id, "history".repeat(286))
            .unwrap();
    }
    let current_turn_id = threads
        .start_turn(
            &thread_id,
            StartTurnRequest {
                command_id: CommandId::new("start-after-history").unwrap(),
                expected_sequence: SequenceExpectation::Any,
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "current input".into(),
                }],
            },
        )
        .unwrap()
        .turn_id;
    let model = Arc::new(CompactingModel::default());
    let executor = TurnExecutor::without_tools(threads.clone(), model.clone());

    let outcome = executor
        .execute(
            &thread_id,
            &current_turn_id,
            &CancellationSource::new().token(),
        )
        .unwrap();

    assert!(matches!(outcome, TurnExecutionOutcome::Completed(_)));
    let snapshot = threads.read_thread(&thread_id).unwrap();
    assert_eq!(snapshot.context_checkpoints.len(), 1);
    assert_eq!(
        snapshot.context_checkpoints[0].summary,
        "bounded checkpoint"
    );
    assert_eq!(snapshot.usage.model_invocations, 2);
    assert_eq!(snapshot.usage.input_tokens.reported, 150);
    assert!(snapshot.usage.input_tokens.complete);
    assert_eq!(snapshot.usage.output_tokens.reported, 15);
    assert!(snapshot.usage.output_tokens.complete);
    let requests = model.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(
        requests[0]
            .instructions
            .as_deref()
            .is_some_and(|body| body.contains("durable context checkpoint"))
    );
    assert!(request_contains(&requests[1], "bounded checkpoint"));
    assert!(!request_contains(&requests[1], &"a".repeat(600)));
    assert_eq!(requests[1].max_output_tokens, Some(200));
}

#[test]
fn provider_preflight_tightens_the_budget_and_rechecks_after_compaction() {
    let (threads, thread_id, first_turn_id) = started_turn();
    threads
        .complete_turn(&thread_id, &first_turn_id, "a".repeat(32_000))
        .unwrap();
    let current_turn_id = threads
        .start_turn(
            &thread_id,
            StartTurnRequest {
                command_id: CommandId::new("start-measured-turn").unwrap(),
                expected_sequence: SequenceExpectation::Any,
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "current input".into(),
                }],
            },
        )
        .unwrap()
        .turn_id;
    let model = Arc::new(PreflightCompactingModel::default());
    let executor = TurnExecutor::without_tools(threads.clone(), model.clone());

    let outcome = executor
        .execute(
            &thread_id,
            &current_turn_id,
            &CancellationSource::new().token(),
        )
        .unwrap();

    assert!(matches!(outcome, TurnExecutionOutcome::Completed(_)));
    assert_eq!(model.measurements.load(Ordering::Relaxed), 2);
    assert_eq!(
        threads
            .read_thread(&thread_id)
            .unwrap()
            .context_checkpoints
            .len(),
        1
    );
    let requests = model.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(
        requests[0]
            .instructions
            .as_deref()
            .is_some_and(|body| body.contains("durable context checkpoint"))
    );
    assert!(request_contains(&requests[1], "measured checkpoint"));
}

#[test]
fn explicit_skill_selection_uses_frozen_digest_and_layered_body() {
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    let thread_id = ThreadId::new("skill-thread").unwrap();
    threads
        .create_thread(CreateThreadRequest {
            session_id: SessionId::new("session").unwrap(),
            thread_id: thread_id.clone(),
            title: "skill".into(),
        })
        .unwrap();
    let skill_id = SkillId::new(
        SkillSourceId::new("user:skill-source:test").unwrap(),
        SkillName::new("review").unwrap(),
    );
    let digest = ContentDigest::sha256(b"skill body");
    let activation = FrozenSkillActivation {
        id: skill_id.clone(),
        content_digest: digest.clone(),
        catalog_generation: 7,
        reason: SkillActivationReason::Explicit,
    };
    let turn_id = threads
        .start_turn(
            &thread_id,
            StartTurnRequest {
                command_id: CommandId::new("skill-start").unwrap(),
                expected_sequence: SequenceExpectation::Any,
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                activated_skills: vec![activation.clone()],
                input: vec![
                    UserInput::Skill {
                        skill: SkillRef::pinned(skill_id, digest.clone()),
                    },
                    UserInput::Text {
                        text: "review this".into(),
                    },
                ],
            },
        )
        .unwrap()
        .turn_id;
    let model = Arc::new(ScriptedModel::new([Ok(text_response("done"))]));
    let mut extensions = zeta_extension_api::ExtensionRegistryBuilder::new();
    extensions.turn_input_contributor(Arc::new(FixedSkillContributor {
        expected: activation.clone(),
        body: "# Review workflow\nInspect correctness first.".into(),
    }));
    let executor = TurnExecutor::without_tools(threads.clone(), model.clone())
        .with_extensions(Arc::new(extensions.build()));

    executor
        .execute(&thread_id, &turn_id, &CancellationSource::new().token())
        .unwrap();

    let request = &model.requests()[0];
    assert!(request_contains(request, "Inspect correctness first."));
    assert!(request_contains(request, digest.as_str()));
    assert!(request_contains(request, "<skill-instructions"));
    assert!(!request_contains(request, "reason=\"explicit\""));
    assert!(!request_contains(request, "catalog-generation"));
    assert_eq!(
        threads.read_thread(&thread_id).unwrap().turns[0].activated_skills,
        vec![activation]
    );
}

#[test]
fn executes_a_durable_tool_loop_before_the_next_model_invocation() {
    let (threads, thread_id, turn_id) = started_turn();
    let call = ToolCall {
        id: ToolCallId::new("call_1").unwrap(),
        name: ToolName::new("weather").unwrap(),
        arguments: json!({"city": "Paris"}),
    };
    let model = Arc::new(ScriptedModel::new([
        Ok(ModelResponse {
            output: vec![ResponseItem::ToolCall(call.clone())],
            usage: None,
            stop_reason: StopReason::ToolUse,
        }),
        Ok(text_response("sunny")),
    ]));
    let tools = Arc::new(WeatherTool);
    let executor = TurnExecutor::new(
        threads.clone(),
        model.clone(),
        tools,
        Arc::new(SandboxActionPolicyService),
    );

    executor
        .execute(&thread_id, &turn_id, &CancellationSource::new().token())
        .unwrap();

    let snapshot = threads.read_thread(&thread_id).unwrap();
    assert!(snapshot.items.iter().any(
        |item| matches!(item, ThreadItem::ToolCall { tool_call_id, .. } if tool_call_id == &call.id)
    ));
    assert!(snapshot.items.iter().any(
        |item| matches!(item, ThreadItem::ToolResult { tool_call_id, text, .. } if tool_call_id == &call.id && text == "sunny")
    ));
    let requests = model.requests();
    assert_eq!(requests.len(), 2);
    assert!(matches!(
        requests[1].input.last(),
        Some(InputItem::ToolResult(result)) if result.call_id == call.id
    ));
}

#[test]
fn repeated_identical_tool_failures_stop_at_five_with_a_stable_turn_error() {
    let (threads, thread_id, turn_id) = started_turn();
    let mut responses = (1..=5)
        .map(|index| {
            Ok(ModelResponse {
                output: vec![ResponseItem::ToolCall(ToolCall {
                    id: ToolCallId::new(format!("repeat-{index}")).unwrap(),
                    name: ToolName::new("weather").unwrap(),
                    arguments: if index % 2 == 0 {
                        json!({"unit": "c", "city": "Paris"})
                    } else {
                        json!({"city": "Paris", "unit": "c"})
                    },
                })],
                usage: None,
                stop_reason: StopReason::ToolUse,
            })
        })
        .collect::<Vec<_>>();
    responses.push(Ok(text_response("must not be invoked")));
    let model = Arc::new(ScriptedModel::new(responses));
    let executor = TurnExecutor::new(
        threads.clone(),
        model.clone(),
        Arc::new(FailingWeatherTool),
        Arc::new(SandboxActionPolicyService),
    );

    let error = match executor.execute(&thread_id, &turn_id, &CancellationSource::new().token()) {
        Ok(_) => panic!("the fifth repeated failure must stop the Turn"),
        Err(error) => error,
    };

    assert!(matches!(error, CoreError::ToolRepetition(_)));
    let requests = model.requests();
    assert_eq!(requests.len(), 5);
    assert!(requests[3].input.iter().any(|input| matches!(
        input,
        InputItem::ToolResult(result)
            if result.content.iter().any(|content| matches!(
                content,
                ContentPart::Text(text)
                    if text.contains(crate::tool_repetition::TOOL_REPETITION_REMINDER)
            ))
    )));
    let snapshot = threads.read_thread(&thread_id).unwrap();
    assert_eq!(snapshot.turns[0].status, TurnStatus::Failed);
    assert_eq!(
        snapshot.turns[0].failure.as_ref().map(|error| error.code),
        Some(StableTurnErrorCode::ToolRepetition)
    );
    assert_eq!(
        snapshot
            .items
            .iter()
            .filter(|item| matches!(item, ThreadItem::ToolResult { is_error: true, .. }))
            .count(),
        5
    );
}

#[test]
fn rejects_a_model_tool_call_outside_the_invocation_capability_scope() {
    let (threads, thread_id, turn_id) = started_turn();
    let model = Arc::new(ScriptedModel::new([Ok(ModelResponse {
        output: vec![ResponseItem::ToolCall(ToolCall {
            id: ToolCallId::new("invented-call").unwrap(),
            name: ToolName::new("invented_tool").unwrap(),
            arguments: json!({}),
        })],
        usage: None,
        stop_reason: StopReason::ToolUse,
    })]));
    let executor = TurnExecutor::without_tools(threads.clone(), model);

    assert!(
        executor
            .execute(&thread_id, &turn_id, &CancellationSource::new().token())
            .is_err()
    );

    let snapshot = threads.read_thread(&thread_id).unwrap();
    assert_eq!(snapshot.turns[0].status, TurnStatus::Failed);
    assert!(
        snapshot
            .items
            .iter()
            .all(|item| !matches!(item, ThreadItem::ToolCall { .. }))
    );
}

#[test]
fn reloadable_instructions_change_only_at_model_invocation_boundaries() {
    let (threads, thread_id, turn_id) = started_turn();
    let instructions = Arc::new(MutableInstructions::new("first instructions"));
    let model = Arc::new(InstructionRefreshingModel {
        instructions: Arc::clone(&instructions),
        requests: Mutex::new(Vec::new()),
    });
    let executor = TurnExecutor::new(
        threads,
        model.clone(),
        Arc::new(WeatherTool),
        Arc::new(SandboxActionPolicyService),
    )
    .with_instructions_provider(instructions);

    executor
        .execute(&thread_id, &turn_id, &CancellationSource::new().token())
        .unwrap();

    let requests = model.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(request_contains(&requests[0], "first instructions"));
    assert!(!request_contains(&requests[0], "second instructions"));
    assert!(request_contains(&requests[1], "second instructions"));
}

#[test]
fn continues_beyond_two_hundred_model_invocations() {
    const TOOL_ROUNDS: usize = 200;

    let (threads, thread_id, turn_id) = started_turn();
    let model = Arc::new(LongRunningToolModel::new(TOOL_ROUNDS));
    let executor = TurnExecutor::new(
        threads.clone(),
        model.clone(),
        Arc::new(WeatherTool),
        Arc::new(SandboxActionPolicyService),
    );

    let outcome = executor
        .execute(&thread_id, &turn_id, &CancellationSource::new().token())
        .unwrap();

    assert!(matches!(outcome, TurnExecutionOutcome::Completed(_)));
    assert_eq!(model.invocations(), TOOL_ROUNDS + 1);
    assert_eq!(
        threads
            .read_thread(&thread_id)
            .unwrap()
            .turns
            .last()
            .unwrap()
            .status,
        TurnStatus::Completed
    );
}

#[test]
fn cancellation_interrupts_the_turn_before_invoking_the_model() {
    let (threads, thread_id, turn_id) = started_turn();
    let model = Arc::new(ScriptedModel::new([Ok(text_response("unused"))]));
    let executor = TurnExecutor::without_tools(threads.clone(), model.clone());
    let cancellation = CancellationSource::new();
    cancellation.cancel();

    assert!(matches!(
        executor.execute(&thread_id, &turn_id, &cancellation.token()),
        Err(CoreError::Cancelled(_))
    ));
    assert!(model.requests().is_empty());
    assert_eq!(
        threads
            .read_thread(&thread_id)
            .unwrap()
            .turns
            .last()
            .unwrap()
            .status,
        TurnStatus::Interrupted
    );
}

#[test]
fn cancellation_after_a_model_response_preserves_its_usage() {
    let (threads, thread_id, turn_id) = started_turn();
    let cancellation = CancellationSource::new();
    let model = Arc::new(CancellingResponseModel {
        cancellation: cancellation.clone(),
    });
    let executor = TurnExecutor::without_tools(threads.clone(), model);

    assert!(matches!(
        executor.execute(&thread_id, &turn_id, &cancellation.token()),
        Err(CoreError::Cancelled(_))
    ));
    let snapshot = threads.read_thread(&thread_id).unwrap();
    assert_eq!(snapshot.turns[0].status, TurnStatus::Interrupted);
    assert_eq!(snapshot.usage.model_invocations, 1);
    assert_eq!(snapshot.usage.input_tokens.reported, 9);
    assert!(snapshot.usage.input_tokens.complete);
    assert!(
        !snapshot
            .items
            .iter()
            .any(|item| matches!(item, ThreadItem::AgentMessage { .. }))
    );
}

#[test]
fn model_failure_durably_fails_the_turn() {
    let (threads, thread_id, turn_id) = started_turn();
    let model = Arc::new(ScriptedModel::new([Err(CoreError::Model(
        "offline".into(),
    ))]));
    let executor = TurnExecutor::without_tools(threads.clone(), model);

    assert!(matches!(
        executor.execute(
            &thread_id,
            &turn_id,
            &CancellationSource::new().token()
        ),
        Err(CoreError::Model(message)) if message == "offline"
    ));
    assert_eq!(
        threads
            .read_thread(&thread_id)
            .unwrap()
            .turns
            .last()
            .unwrap()
            .status,
        TurnStatus::Failed
    );
}

#[test]
fn retries_transient_model_failures_before_completing() {
    let (threads, thread_id, turn_id) = started_turn();
    let model = Arc::new(ScriptedModel::new([
        Err(CoreError::ModelTransient {
            retry_after_ms: None,
        }),
        Err(CoreError::ModelTransient {
            retry_after_ms: None,
        }),
        Ok(text_response("recovered")),
    ]));
    let executor = TurnExecutor::without_tools(threads.clone(), model.clone());

    let outcome = executor
        .execute(&thread_id, &turn_id, &CancellationSource::new().token())
        .unwrap();

    assert!(matches!(outcome, TurnExecutionOutcome::Completed(_)));
    assert_eq!(model.requests().len(), 3);
}

#[test]
fn retries_an_invalid_model_response_once_before_failing_stably() {
    let (threads, thread_id, turn_id) = started_turn();
    let model = Arc::new(ScriptedModel::new([
        Err(CoreError::ModelInvalidResponse),
        Err(CoreError::ModelInvalidResponse),
        Ok(text_response("must not be reached")),
    ]));
    let executor = TurnExecutor::without_tools(threads.clone(), model.clone());

    assert!(matches!(
        executor.execute(&thread_id, &turn_id, &CancellationSource::new().token()),
        Err(CoreError::ModelInvalidResponse)
    ));
    assert_eq!(model.requests().len(), 2);
    let snapshot = threads.read_thread(&thread_id).unwrap();
    let failure = snapshot.turns.last().unwrap().failure.as_ref().unwrap();
    assert_eq!(failure.code, StableTurnErrorCode::InvalidResponse);
    assert!(failure.retryable);
}

#[test]
fn semantic_model_failures_do_not_retry_and_keep_stable_turn_codes() {
    let cases = [
        (
            CoreError::ModelContextOverflow,
            StableTurnErrorCode::ContextOverflow,
            true,
        ),
        (
            CoreError::ModelAuthFailed,
            StableTurnErrorCode::ProviderAuth,
            false,
        ),
        (
            CoreError::ModelInvalidRequest,
            StableTurnErrorCode::InvalidRequest,
            false,
        ),
    ];

    for (error, code, retryable) in cases {
        let (threads, thread_id, turn_id) = started_turn();
        let model = Arc::new(ScriptedModel::new([
            Err(error.clone()),
            Ok(text_response("must not be reached")),
        ]));
        let executor = TurnExecutor::without_tools(threads.clone(), model.clone());

        assert_eq!(
            executor
                .execute(&thread_id, &turn_id, &CancellationSource::new().token())
                .err(),
            Some(error)
        );
        assert_eq!(model.requests().len(), 1);
        let snapshot = threads.read_thread(&thread_id).unwrap();
        let failure = snapshot.turns.last().unwrap().failure.as_ref().unwrap();
        assert_eq!(failure.code, code);
        assert_eq!(failure.retryable, retryable);
    }
}

#[test]
fn context_overflow_commits_one_checkpoint_before_retrying_with_the_new_snapshot() {
    let (threads, thread_id, turn_id) = started_turn_with_history();
    let model = Arc::new(CheckpointObservingModel::new(
        threads.clone(),
        thread_id.clone(),
        turn_id.clone(),
        Ok(text_response("recovered")),
    ));
    let compaction = Arc::new(FixedOverflowCompaction::default());
    let executor = TurnExecutor::without_tools(threads.clone(), model.clone())
        .with_context_compaction_service(compaction.clone());

    let outcome = executor
        .execute(&thread_id, &turn_id, &CancellationSource::new().token())
        .unwrap();

    assert!(matches!(outcome, TurnExecutionOutcome::Completed(_)));
    assert_eq!(model.requests().len(), 2);
    assert_eq!(compaction.calls.load(Ordering::Relaxed), 1);
    let snapshot = threads.read_thread(&thread_id).unwrap();
    assert_eq!(snapshot.context_checkpoints.len(), 1);
    assert_eq!(snapshot.context_overflow_recoveries.len(), 1);
    assert_eq!(
        snapshot.context_overflow_recoveries.get(&turn_id),
        Some(&snapshot.context_checkpoints[0].checkpoint_id)
    );
}

#[test]
fn a_second_context_overflow_fails_stably_without_another_compaction() {
    let (threads, thread_id, turn_id) = started_turn_with_history();
    let model = Arc::new(CheckpointObservingModel::new(
        threads.clone(),
        thread_id.clone(),
        turn_id.clone(),
        Err(CoreError::ModelContextOverflow),
    ));
    let compaction = Arc::new(FixedOverflowCompaction::default());
    let executor = TurnExecutor::without_tools(threads.clone(), model.clone())
        .with_context_compaction_service(compaction.clone());

    assert_eq!(
        executor
            .execute(&thread_id, &turn_id, &CancellationSource::new().token())
            .err(),
        Some(CoreError::ModelContextOverflow)
    );

    assert_eq!(model.requests().len(), 2);
    assert_eq!(compaction.calls.load(Ordering::Relaxed), 1);
    let snapshot = threads.read_thread(&thread_id).unwrap();
    assert_eq!(snapshot.context_checkpoints.len(), 1);
    let failure = snapshot.turns.last().unwrap().failure.as_ref().unwrap();
    assert_eq!(failure.code, StableTurnErrorCode::ContextOverflow);
}

#[test]
fn cancellation_during_overflow_compaction_prevents_the_checkpoint_and_retry() {
    let (threads, thread_id, turn_id) = started_turn_with_history();
    let model = Arc::new(CheckpointObservingModel::new(
        threads.clone(),
        thread_id.clone(),
        turn_id.clone(),
        Ok(text_response("must not be reached")),
    ));
    let compaction = Arc::new(BlockingOverflowCompaction::default());
    let executor = TurnExecutor::without_tools(threads.clone(), model.clone())
        .with_context_compaction_service(compaction.clone());

    executor.start(&thread_id, &turn_id).unwrap();
    compaction.wait_until_entered();
    threads
        .interrupt_turn(
            &thread_id,
            crate::InterruptTurnRequest {
                command_id: CommandId::new("cancel-overflow-compaction").unwrap(),
                expected_sequence: SequenceExpectation::Any,
                turn_id: turn_id.clone(),
            },
        )
        .unwrap();
    wait_for_turn_status(&threads, &thread_id, &turn_id, TurnStatus::Interrupted);
    wait_for_flag(
        &compaction.was_cancelled,
        "overflow compaction was not cancelled",
    );

    assert_eq!(model.requests().len(), 1);
    let snapshot = threads.read_thread(&thread_id).unwrap();
    assert!(snapshot.context_checkpoints.is_empty());
    assert!(snapshot.context_overflow_recoveries.is_empty());
}

#[test]
fn restart_after_overflow_checkpoint_commit_does_not_replay_the_model_call() {
    let store = Arc::new(InMemoryThreadStore::default());
    let original = Arc::new(ThreadController::with_store(store.clone()));
    let thread_id = ThreadId::new("overflow-restart-thread").unwrap();
    original
        .create_thread(CreateThreadRequest {
            session_id: SessionId::new("overflow-restart-session").unwrap(),
            thread_id: thread_id.clone(),
            title: "overflow restart".into(),
        })
        .unwrap();
    let history_turn = original
        .start_turn(
            &thread_id,
            StartTurnRequest {
                command_id: CommandId::new("overflow-restart-history").unwrap(),
                expected_sequence: SequenceExpectation::Any,
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "history".into(),
                }],
            },
        )
        .unwrap()
        .turn_id;
    original
        .complete_turn(&thread_id, &history_turn, "answer".repeat(100))
        .unwrap();
    let current_turn = original
        .start_turn(
            &thread_id,
            StartTurnRequest {
                command_id: CommandId::new("overflow-restart-current").unwrap(),
                expected_sequence: SequenceExpectation::Any,
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "continue".into(),
                }],
            },
        )
        .unwrap()
        .turn_id;
    let source = original.read_thread(&thread_id).unwrap();
    let covered_end_sequence = source
        .items
        .iter()
        .filter(|item| item.turn_id() == &history_turn)
        .filter_map(|item| source.item_sequences.get(item.item_id()))
        .copied()
        .max()
        .unwrap();
    original
        .commit_context_overflow_recovery(
            &thread_id,
            &current_turn,
            CommitContextCheckpointRequest {
                source_thread_sequence: source.sequence,
                covered: zeta_protocol::ContextSourceRange {
                    start_sequence: 1,
                    end_sequence: covered_end_sequence,
                },
                summary: "overflow checkpoint".into(),
                schema_revision: "context-checkpoint-v1".into(),
                prompt_revision: "compaction-test-v1".into(),
                context_policy_revision: "context-policy-v1".into(),
                generator_model: None,
            },
        )
        .unwrap();

    let recovered = Arc::new(ThreadController::with_store(store));
    let snapshot = recovered.recover_thread(&thread_id).unwrap();
    let model = Arc::new(ScriptedModel::new([Ok(text_response(
        "must not be invoked",
    ))]));
    let executor = TurnExecutor::without_tools(recovered, model.clone());

    assert_eq!(executor.resume_recovered_tool_continuations().unwrap(), 0);
    assert!(model.requests().is_empty());
    assert_eq!(
        snapshot
            .turns
            .iter()
            .find(|turn| turn.turn_id == current_turn)
            .unwrap()
            .status,
        TurnStatus::Interrupted
    );
    assert_eq!(snapshot.context_checkpoints.len(), 1);
    assert_eq!(snapshot.context_overflow_recoveries.len(), 1);
}

#[test]
fn retries_one_empty_response_and_completes_refusal_as_agent_message() {
    let (threads, thread_id, turn_id) = started_turn();
    let model = Arc::new(ScriptedModel::new([
        Ok(ModelResponse {
            output: Vec::new(),
            usage: Some(ModelUsage {
                input_tokens: Some(10),
                output_tokens: Some(1),
                cached_input_tokens: Some(2),
                reasoning_tokens: None,
            }),
            stop_reason: StopReason::Completed,
        }),
        Ok(ModelResponse {
            output: vec![ResponseItem::Refusal("cannot comply".into())],
            usage: Some(ModelUsage {
                input_tokens: Some(12),
                output_tokens: Some(3),
                cached_input_tokens: None,
                reasoning_tokens: Some(1),
            }),
            stop_reason: StopReason::Completed,
        }),
    ]));
    let executor = TurnExecutor::without_tools(threads.clone(), model.clone());

    let outcome = executor
        .execute(&thread_id, &turn_id, &CancellationSource::new().token())
        .unwrap();

    assert!(matches!(
        outcome,
        TurnExecutionOutcome::Completed(crate::CompletedTurn {
            item: ThreadItem::AgentMessage { ref text, .. },
            ..
        }) if text == "cannot comply"
    ));
    assert_eq!(model.requests().len(), 2);
    let snapshot = threads.read_thread(&thread_id).unwrap();
    assert_eq!(snapshot.usage.model_invocations, 2);
    assert_eq!(snapshot.usage.input_tokens.reported, 22);
    assert!(snapshot.usage.input_tokens.complete);
    assert_eq!(snapshot.usage.output_tokens.reported, 4);
    assert!(snapshot.usage.output_tokens.complete);
    assert_eq!(snapshot.usage.cached_input_tokens.reported, 2);
    assert!(!snapshot.usage.cached_input_tokens.complete);
    assert_eq!(snapshot.usage.reasoning_tokens.reported, 1);
    assert!(!snapshot.usage.reasoning_tokens.complete);
    assert_eq!(snapshot.turns[0].usage, snapshot.usage);
}

#[test]
fn model_usage_projection_is_identical_after_recovery() {
    let store = Arc::new(InMemoryThreadStore::default());
    let original = Arc::new(ThreadController::with_store(store.clone()));
    let thread_id = ThreadId::new("usage-recovery-thread").unwrap();
    original
        .create_thread(CreateThreadRequest {
            session_id: SessionId::new("usage-recovery-session").unwrap(),
            thread_id: thread_id.clone(),
            title: "usage recovery".into(),
        })
        .unwrap();
    let turn_id = original
        .start_turn(
            &thread_id,
            StartTurnRequest {
                command_id: CommandId::new("usage-recovery-start").unwrap(),
                expected_sequence: SequenceExpectation::Any,
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "hello".into(),
                }],
            },
        )
        .unwrap()
        .turn_id;
    let model = Arc::new(ScriptedModel::new([Ok(ModelResponse {
        output: vec![ResponseItem::Text("answer".into())],
        usage: Some(ModelUsage {
            input_tokens: Some(11),
            output_tokens: Some(2),
            cached_input_tokens: None,
            reasoning_tokens: None,
        }),
        stop_reason: StopReason::Completed,
    })]));
    TurnExecutor::without_tools(original.clone(), model)
        .execute(&thread_id, &turn_id, &CancellationSource::new().token())
        .unwrap();
    let before_restart = original.read_thread(&thread_id).unwrap().usage;

    let recovered = ThreadController::with_store(store);
    let after_restart = recovered.recover_thread(&thread_id).unwrap().usage;

    assert_eq!(after_restart, before_restart);
    assert_eq!(after_restart.model_invocations, 1);
    assert_eq!(after_restart.input_tokens.reported, 11);
    assert!(after_restart.input_tokens.complete);
    assert!(!after_restart.cached_input_tokens.complete);
}

#[test]
fn streaming_delta_and_final_item_share_one_identity() {
    let (threads, thread_id, turn_id) = started_turn();
    let updates = Arc::new(RecordingUpdates::default());
    let executor = TurnExecutor::without_tools(threads.clone(), Arc::new(ChunkedModel))
        .with_thread_updates(updates.clone());

    executor.start(&thread_id, &turn_id).unwrap();
    wait_for_turn_status(&threads, &thread_id, &turn_id, TurnStatus::Completed);

    let updates = updates.updates();
    let started_item_id = updates.iter().find_map(|update| match &update.update {
        ThreadUpdate::ItemStarted {
            item: ThreadItem::AgentMessage { item_id, .. },
            ..
        } => Some(item_id.clone()),
        _ => None,
    });
    let delta_item_id = updates.iter().find_map(|update| match &update.update {
        ThreadUpdate::ItemDelta { item_id, .. } => Some(item_id.clone()),
        _ => None,
    });
    let snapshot = threads.read_thread(&thread_id).unwrap();
    let completed_item_id = snapshot.items.iter().find_map(|item| match item {
        ThreadItem::AgentMessage { item_id, .. } => Some(item_id.clone()),
        _ => None,
    });

    assert_eq!(started_item_id, delta_item_id);
    assert_eq!(delta_item_id, completed_item_id);
    let text = updates
        .iter()
        .filter_map(|update| match &update.update {
            ThreadUpdate::ItemDelta {
                delta: zeta_protocol::ItemDelta::AgentMessage { text },
                ..
            } => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();
    assert_eq!(text, "hello");
}

#[test]
fn per_thread_mailboxes_run_independently_and_interrupt_the_active_turn() {
    let (threads, slow_thread_id, slow_turn_id) = started_turn();
    let fast_thread_id = ThreadId::new("fast-thread").unwrap();
    threads
        .create_thread(CreateThreadRequest {
            session_id: SessionId::new("session").unwrap(),
            thread_id: fast_thread_id.clone(),
            title: "fast".into(),
        })
        .unwrap();
    let fast_turn_id = threads
        .start_turn(
            &fast_thread_id,
            StartTurnRequest {
                command_id: CommandId::new("fast-start").unwrap(),
                expected_sequence: SequenceExpectation::Any,
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "fast".into(),
                }],
            },
        )
        .unwrap()
        .turn_id;
    let model = Arc::new(BlockingFirstModel::default());
    let executor = TurnExecutor::without_tools(threads.clone(), model.clone());

    executor.start(&slow_thread_id, &slow_turn_id).unwrap();
    model.wait_until_slow_invocation_enters();
    executor.start(&fast_thread_id, &fast_turn_id).unwrap();
    wait_for_turn_status(
        &threads,
        &fast_thread_id,
        &fast_turn_id,
        TurnStatus::Completed,
    );

    threads
        .interrupt_turn(
            &slow_thread_id,
            crate::InterruptTurnRequest {
                command_id: CommandId::new("slow-interrupt").unwrap(),
                expected_sequence: SequenceExpectation::Any,
                turn_id: slow_turn_id.clone(),
            },
        )
        .unwrap();
    wait_for_turn_status(
        &threads,
        &slow_thread_id,
        &slow_turn_id,
        TurnStatus::Interrupted,
    );

    wait_for_flag(&model.slow_was_cancelled, "slow model was not cancelled");
}

#[test]
fn interrupt_propagates_to_the_active_tool_call() {
    let (threads, thread_id, turn_id) = started_turn();
    let call = ToolCall {
        id: ToolCallId::new("blocking_call").unwrap(),
        name: ToolName::new("blocking").unwrap(),
        arguments: json!({}),
    };
    let model = Arc::new(ScriptedModel::new([Ok(ModelResponse {
        output: vec![ResponseItem::ToolCall(call)],
        usage: None,
        stop_reason: StopReason::ToolUse,
    })]));
    let tool = Arc::new(BlockingTool::default());
    let executor = TurnExecutor::new(
        threads.clone(),
        model,
        tool.clone(),
        Arc::new(SandboxActionPolicyService),
    );

    executor.start(&thread_id, &turn_id).unwrap();
    tool.wait_until_entered();
    threads
        .interrupt_turn(
            &thread_id,
            crate::InterruptTurnRequest {
                command_id: CommandId::new("tool-interrupt").unwrap(),
                expected_sequence: SequenceExpectation::Any,
                turn_id: turn_id.clone(),
            },
        )
        .unwrap();

    wait_for_turn_status(&threads, &thread_id, &turn_id, TurnStatus::Interrupted);
    wait_for_flag(
        &tool.was_cancelled,
        "active tool did not observe cancellation",
    );
    let snapshot = threads.read_thread(&thread_id).unwrap();
    assert!(
        snapshot
            .started_tool_calls
            .contains(&ToolCallId::new("blocking_call").unwrap())
    );
    assert!(!snapshot.items.iter().any(
        |item| matches!(item, ThreadItem::ToolResult { tool_call_id, .. } if tool_call_id.as_str() == "blocking_call")
    ));
}

#[test]
fn running_tool_user_input_is_durable_and_resumes_the_same_execution() {
    let (threads, thread_id, turn_id) = started_turn();
    let call = ToolCall {
        id: ToolCallId::new("interactive_call").unwrap(),
        name: ToolName::new("interactive").unwrap(),
        arguments: json!({}),
    };
    let model = Arc::new(ScriptedModel::new([
        Ok(ModelResponse {
            output: vec![ResponseItem::ToolCall(call)],
            usage: None,
            stop_reason: StopReason::ToolUse,
        }),
        Ok(text_response("done")),
    ]));
    let executor = TurnExecutor::new(
        threads.clone(),
        model,
        Arc::new(InteractiveTool),
        Arc::new(SandboxActionPolicyService),
    );

    executor.start(&thread_id, &turn_id).unwrap();
    let deadline = Instant::now() + Duration::from_secs(1);
    let pending = loop {
        let snapshot = threads.read_thread(&thread_id).unwrap();
        if let Some(interaction) = snapshot.turns[0].pending_interaction.clone() {
            break (snapshot.sequence, interaction);
        }
        assert!(
            Instant::now() < deadline,
            "interactive tool did not request input"
        );
        thread::sleep(Duration::from_millis(2));
    };
    assert!(matches!(
        pending.1.request,
        zeta_protocol::AgentRequest::UserInput { .. }
    ));
    let resolved = threads
        .resolve_turn_interaction(
            &thread_id,
            crate::ResolveTurnInteractionRequest {
                command_id: CommandId::new("resolve-interactive").unwrap(),
                expected_sequence: SequenceExpectation::Exact(pending.0),
                turn_id: turn_id.clone(),
                request_id: pending.1.request_id,
                response: AgentResponse::UserInput {
                    response: RequestUserInputResponse {
                        answers: BTreeMap::from([(
                            "city".into(),
                            UserInputAnswer {
                                value: "Paris".into(),
                            },
                        )]),
                    },
                },
            },
        )
        .unwrap();
    assert!(resolved.live_execution_woken);

    wait_for_turn_status(&threads, &thread_id, &turn_id, TurnStatus::Completed);
    let snapshot = threads.read_thread(&thread_id).unwrap();
    assert!(snapshot.items.iter().any(|item| matches!(
        item,
        ThreadItem::ToolResult { text, is_error: false, .. } if text == "Paris"
    )));
}

struct ScriptedModel {
    responses: Mutex<VecDeque<Result<ModelResponse, CoreError>>>,
    requests: Mutex<Vec<ModelRequest>>,
}

struct CancellingResponseModel {
    cancellation: CancellationSource,
}

impl ModelService for CancellingResponseModel {
    fn invoke(
        &self,
        _: ModelSelection<'_>,
        _: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        self.cancellation.cancel();
        Ok(ModelResponse {
            output: vec![ResponseItem::Text("discarded after cancellation".into())],
            usage: Some(ModelUsage {
                input_tokens: Some(9),
                output_tokens: Some(2),
                cached_input_tokens: None,
                reasoning_tokens: None,
            }),
            stop_reason: StopReason::Completed,
        })
    }

    fn stream(
        &self,
        selection: ModelSelection<'_>,
        request: &ModelRequest,
        cancellation: &CancellationToken,
        _: &mut dyn ModelStreamSink,
    ) -> Result<ModelResponse, CoreError> {
        self.invoke(selection, request, cancellation)
    }
}

struct SteeringModel {
    threads: Arc<ThreadController>,
    thread_id: ThreadId,
    turn_id: zeta_protocol::TurnId,
    requests: Mutex<Vec<ModelRequest>>,
}

impl ModelService for SteeringModel {
    fn invoke(
        &self,
        _: ModelSelection<'_>,
        request: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        let invocation = {
            let mut requests = self.requests.lock().unwrap();
            requests.push(request.clone());
            requests.len()
        };
        if invocation == 1 {
            let sequence = self.threads.read_thread(&self.thread_id)?.sequence;
            self.threads.steer_turn(
                &self.thread_id,
                SteerTurnRequest {
                    command_id: CommandId::new("steer-during-model").unwrap(),
                    expected_sequence: SequenceExpectation::Exact(sequence),
                    turn_id: self.turn_id.clone(),
                    input: vec![UserInput::Text {
                        text: "steer toward tests".into(),
                    }],
                },
            )?;
            Ok(ModelResponse {
                output: vec![
                    ResponseItem::Reasoning("stale reasoning".into()),
                    ResponseItem::ToolCall(ToolCall {
                        id: ToolCallId::new("stale-call").unwrap(),
                        name: ToolName::new("stale-tool").unwrap(),
                        arguments: json!({}),
                    }),
                ],
                usage: None,
                stop_reason: StopReason::ToolUse,
            })
        } else {
            Ok(text_response("steered answer"))
        }
    }
}

struct CheckpointObservingModel {
    threads: Arc<ThreadController>,
    thread_id: ThreadId,
    turn_id: TurnId,
    second_response: Mutex<Option<Result<ModelResponse, CoreError>>>,
    requests: Mutex<Vec<ModelRequest>>,
}

impl CheckpointObservingModel {
    fn new(
        threads: Arc<ThreadController>,
        thread_id: ThreadId,
        turn_id: TurnId,
        second_response: Result<ModelResponse, CoreError>,
    ) -> Self {
        Self {
            threads,
            thread_id,
            turn_id,
            second_response: Mutex::new(Some(second_response)),
            requests: Mutex::new(Vec::new()),
        }
    }

    fn requests(&self) -> Vec<ModelRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl ModelService for CheckpointObservingModel {
    fn invoke(
        &self,
        _: ModelSelection<'_>,
        request: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        let invocation = {
            let mut requests = self.requests.lock().unwrap();
            let invocation = requests.len();
            requests.push(request.clone());
            invocation
        };
        if invocation == 0 {
            return Err(CoreError::ModelContextOverflow);
        }
        assert_eq!(invocation, 1, "overflow recovery may retry only once");
        let snapshot = self.threads.read_thread(&self.thread_id).unwrap();
        let checkpoint_id = snapshot
            .context_overflow_recoveries
            .get(&self.turn_id)
            .expect("recovery marker must be durable before the retry");
        assert_eq!(
            Some(checkpoint_id),
            snapshot
                .context_checkpoints
                .last()
                .map(|checkpoint| &checkpoint.checkpoint_id)
        );
        assert!(request_contains(request, "overflow checkpoint"));
        self.second_response
            .lock()
            .unwrap()
            .take()
            .expect("second response is configured")
    }
}

#[derive(Default)]
struct FixedOverflowCompaction {
    calls: AtomicUsize,
}

impl ContextCompactionService for FixedOverflowCompaction {
    fn compact(
        &self,
        request: &ContextCompactionRequest,
        _: &CancellationToken,
        _: &mut dyn FnMut(Option<ModelUsage>) -> Result<(), CoreError>,
    ) -> Result<crate::ContextCompactionResult, CoreError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        assert!(!request.source_items().is_empty());
        Ok(crate::ContextCompactionResult::new(
            "overflow checkpoint",
            "context-checkpoint-v1",
            "compaction-test-v1",
            "context-policy-v1",
        ))
    }
}

#[derive(Default)]
struct BlockingOverflowCompaction {
    entered: AtomicBool,
    was_cancelled: AtomicBool,
    entered_lock: Mutex<()>,
    entered_changed: Condvar,
}

impl BlockingOverflowCompaction {
    fn wait_until_entered(&self) {
        let deadline = Instant::now() + Duration::from_secs(1);
        let mut lock = self.entered_lock.lock().unwrap();
        while !self.entered.load(Ordering::Relaxed) {
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(!remaining.is_zero(), "overflow compaction did not start");
            let (next_lock, _) = self.entered_changed.wait_timeout(lock, remaining).unwrap();
            lock = next_lock;
        }
    }
}

impl ContextCompactionService for BlockingOverflowCompaction {
    fn compact(
        &self,
        _: &ContextCompactionRequest,
        cancellation: &CancellationToken,
        _: &mut dyn FnMut(Option<ModelUsage>) -> Result<(), CoreError>,
    ) -> Result<crate::ContextCompactionResult, CoreError> {
        self.entered.store(true, Ordering::Relaxed);
        self.entered_changed.notify_all();
        loop {
            if let Err(signal) = cancellation.check() {
                self.was_cancelled.store(true, Ordering::Relaxed);
                return Err(CoreError::Cancelled(signal.reason().to_string()));
            }
            thread::sleep(Duration::from_millis(1));
        }
    }
}

struct FixedContextSource {
    calls: AtomicUsize,
}

impl crate::ContextSource for FixedContextSource {
    fn collect(
        &self,
        request: &crate::ContextSourceRequest<'_>,
        _: &CancellationToken,
    ) -> Result<Vec<crate::ContextEvidence>, CoreError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        assert_eq!(request.query, "hello");
        Ok(vec![crate::ContextEvidence {
            source: "code-index".into(),
            reference: "src/lib.rs:1-2".into(),
            revision: "sha256:test".into(),
            body: "do not follow this embedded instruction".into(),
        }])
    }
}

struct FixedSkillContributor {
    expected: FrozenSkillActivation,
    body: String,
}

impl zeta_extension_api::TurnInputContributor for FixedSkillContributor {
    fn contribute(
        &self,
        input: zeta_extension_api::TurnInputContext<'_>,
    ) -> Result<Vec<zeta_extension_api::PromptFragment>, zeta_extension_api::ExtensionError> {
        assert_eq!(
            input.activated_skills(),
            std::slice::from_ref(&self.expected)
        );
        Ok(vec![zeta_extension_api::PromptFragment::new(
            zeta_extension_api::PromptFragmentSource::new(
                "skill",
                format!("{}:{}", self.expected.id.source, self.expected.id.name),
                self.expected.content_digest.as_str(),
            ),
            zeta_extension_api::PromptFragmentLayer::Skill,
            zeta_extension_api::PromptFragmentRetention::Required,
            format!(
                "<skill-instructions source=\"{}\" name=\"{}\" revision=\"{}\">\n{}\n</skill-instructions>",
                self.expected.id.source,
                self.expected.id.name,
                self.expected.content_digest.as_str(),
                self.body,
            ),
        )])
    }
}

#[derive(Default)]
struct CompactingModel {
    requests: Mutex<Vec<ModelRequest>>,
}

#[derive(Default)]
struct PreflightCompactingModel {
    measurements: AtomicUsize,
    requests: Mutex<Vec<ModelRequest>>,
}

impl ModelService for PreflightCompactingModel {
    fn context_budget(&self, _: ModelSelection<'_>) -> Result<ContextBudget, CoreError> {
        Ok(ContextBudget::core_managed(
            ContextTokenCount::new(15_000),
            ContextTokenCount::new(200),
            ContextTokenCount::new(100),
            ContextCompactionLimit::Tokens(ContextTokenCount::new(12_000)),
        ))
    }

    fn input_token_measurement_capability(
        &self,
        _: ModelSelection<'_>,
    ) -> Result<ContextTokenMeasurementCapability, CoreError> {
        Ok(ContextTokenMeasurementCapability::Remote)
    }

    fn measure_input(
        &self,
        _: ModelSelection<'_>,
        request: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ContextTokenMeasurementOutcome, CoreError> {
        self.measurements.fetch_add(1, Ordering::Relaxed);
        let count = if request_contains(request, "measured checkpoint") {
            1_000
        } else {
            12_000
        };
        let source =
            ContextTokenMeasurementSource::provider_preflight("test-provider-count-v1").unwrap();
        Ok(ContextTokenMeasurementOutcome::Measured(
            ContextTokenMeasurement::exact(ContextTokenCount::new(count), source),
        ))
    }

    fn invoke(
        &self,
        _: ModelSelection<'_>,
        request: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        self.requests.lock().unwrap().push(request.clone());
        if request
            .instructions
            .as_deref()
            .is_some_and(|body| body.contains("durable context checkpoint"))
        {
            Ok(text_response("measured checkpoint"))
        } else {
            Ok(text_response("done"))
        }
    }
}

impl ModelService for CompactingModel {
    fn context_budget(&self, _: ModelSelection<'_>) -> Result<ContextBudget, CoreError> {
        Ok(ContextBudget::core_managed(
            ContextTokenCount::new(2_100),
            ContextTokenCount::new(200),
            ContextTokenCount::new(100),
            ContextCompactionLimit::ContextWindow,
        ))
    }

    fn invoke(
        &self,
        _: ModelSelection<'_>,
        request: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        self.requests.lock().unwrap().push(request.clone());
        let is_compaction = request
            .instructions
            .as_deref()
            .is_some_and(|body| body.contains("durable context checkpoint"));
        let mut response = if is_compaction {
            text_response("bounded checkpoint")
        } else {
            text_response("done")
        };
        response.usage = Some(ModelUsage {
            input_tokens: Some(if is_compaction { 100 } else { 50 }),
            output_tokens: Some(if is_compaction { 10 } else { 5 }),
            cached_input_tokens: None,
            reasoning_tokens: None,
        });
        Ok(response)
    }
}

struct LongRunningToolModel {
    tool_rounds: usize,
    invocations: AtomicUsize,
}

struct MutableInstructions {
    current: Mutex<Arc<HarnessInstructions>>,
}

impl MutableInstructions {
    fn new(content: &str) -> Self {
        Self {
            current: Mutex::new(Arc::new(HarnessInstructions::new(
                "system",
                "environment",
                Some(content.into()),
            ))),
        }
    }

    fn replace(&self, content: &str) {
        *self.current.lock().unwrap() = Arc::new(HarnessInstructions::new(
            "system",
            "environment",
            Some(content.into()),
        ));
    }
}

impl HarnessInstructionsProvider for MutableInstructions {
    fn snapshot(&self) -> Arc<HarnessInstructions> {
        Arc::clone(&self.current.lock().unwrap())
    }
}

struct InstructionRefreshingModel {
    instructions: Arc<MutableInstructions>,
    requests: Mutex<Vec<ModelRequest>>,
}

impl ModelService for InstructionRefreshingModel {
    fn invoke(
        &self,
        _: ModelSelection<'_>,
        request: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        let mut requests = self.requests.lock().unwrap();
        requests.push(request.clone());
        if requests.len() == 1 {
            self.instructions.replace("second instructions");
            return Ok(ModelResponse {
                output: vec![ResponseItem::ToolCall(ToolCall {
                    id: ToolCallId::new("refresh_instructions").unwrap(),
                    name: ToolName::new("weather").unwrap(),
                    arguments: json!({"city": "Paris"}),
                })],
                usage: None,
                stop_reason: StopReason::ToolUse,
            });
        }
        Ok(text_response("done"))
    }
}

impl LongRunningToolModel {
    fn new(tool_rounds: usize) -> Self {
        Self {
            tool_rounds,
            invocations: AtomicUsize::new(0),
        }
    }

    fn invocations(&self) -> usize {
        self.invocations.load(Ordering::Relaxed)
    }
}

impl ModelService for LongRunningToolModel {
    fn invoke(
        &self,
        _: ModelSelection<'_>,
        _: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        let invocation = self.invocations.fetch_add(1, Ordering::Relaxed);
        if invocation == self.tool_rounds {
            return Ok(text_response("done"));
        }
        Ok(ModelResponse {
            output: vec![ResponseItem::ToolCall(ToolCall {
                id: ToolCallId::new(format!("call_{invocation}")).unwrap(),
                name: ToolName::new("weather").unwrap(),
                arguments: json!({"city": "Paris"}),
            })],
            usage: None,
            stop_reason: StopReason::ToolUse,
        })
    }
}

#[derive(Default)]
struct RecordingUpdates(Mutex<Vec<ThreadUpdateEnvelope>>);

impl RecordingUpdates {
    fn updates(&self) -> Vec<ThreadUpdateEnvelope> {
        self.0.lock().unwrap().clone()
    }
}

impl ThreadUpdateSink for RecordingUpdates {
    fn publish(&self, update: ThreadUpdateEnvelope) {
        self.0.lock().unwrap().push(update);
    }
}

struct ChunkedModel;

impl ModelService for ChunkedModel {
    fn invoke(
        &self,
        _: ModelSelection<'_>,
        _: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        unreachable!("stream is overridden")
    }

    fn stream(
        &self,
        _: ModelSelection<'_>,
        _: &ModelRequest,
        _: &CancellationToken,
        sink: &mut dyn ModelStreamSink,
    ) -> Result<ModelResponse, CoreError> {
        sink.emit(ModelStreamEvent::TextDelta("hel".into()))?;
        sink.emit(ModelStreamEvent::TextDelta("lo".into()))?;
        Ok(text_response("hello"))
    }
}

#[derive(Default)]
struct BlockingFirstModel {
    slow_has_entered: AtomicBool,
    slow_was_cancelled: AtomicBool,
    entered_lock: Mutex<()>,
    entered: Condvar,
}

impl BlockingFirstModel {
    fn wait_until_slow_invocation_enters(&self) {
        let deadline = Instant::now() + Duration::from_secs(1);
        let mut lock = self.entered_lock.lock().unwrap();
        while !self.slow_has_entered.load(Ordering::Relaxed) {
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(!remaining.is_zero(), "slow model invocation did not start");
            let (next_lock, _) = self.entered.wait_timeout(lock, remaining).unwrap();
            lock = next_lock;
        }
    }
}

impl ModelService for BlockingFirstModel {
    fn invoke(
        &self,
        _: ModelSelection<'_>,
        _: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        unreachable!("stream is overridden")
    }

    fn stream(
        &self,
        _: ModelSelection<'_>,
        request: &ModelRequest,
        cancellation: &CancellationToken,
        sink: &mut dyn ModelStreamSink,
    ) -> Result<ModelResponse, CoreError> {
        let prompt = request
            .input
            .iter()
            .find_map(|input| match input {
                InputItem::Message(message) => {
                    message.content.iter().find_map(|content| match content {
                        ContentPart::Text(text) => Some(text.as_str()),
                        ContentPart::ImageAttachment { .. } => None,
                        ContentPart::ImageUrl { .. } => None,
                    })
                }
                InputItem::ToolResult(_) => None,
            })
            .unwrap_or_default();
        if prompt == "fast" {
            sink.emit(ModelStreamEvent::TextDelta("fast result".into()))?;
            return Ok(text_response("fast result"));
        }
        sink.emit(ModelStreamEvent::TextDelta("partial".into()))?;
        self.slow_has_entered.store(true, Ordering::Relaxed);
        self.entered.notify_all();
        while !cancellation.is_cancelled() {
            thread::sleep(Duration::from_millis(1));
        }
        self.slow_was_cancelled.store(true, Ordering::Relaxed);
        Err(CoreError::Cancelled("cancelled during stream".into()))
    }
}

impl ScriptedModel {
    fn new(responses: impl IntoIterator<Item = Result<ModelResponse, CoreError>>) -> Self {
        Self {
            responses: Mutex::new(responses.into_iter().collect()),
            requests: Mutex::new(Vec::new()),
        }
    }

    fn requests(&self) -> Vec<ModelRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl ModelService for ScriptedModel {
    fn invoke(
        &self,
        _: ModelSelection<'_>,
        request: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        self.requests.lock().unwrap().push(request.clone());
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("script contains a response")
    }
}

struct WeatherTool;

struct FailingWeatherTool;

struct DeferredWeatherTools;

impl ToolService for DeferredWeatherTools {
    fn definitions(&self) -> Vec<ToolDefinition> {
        vec![tool_definition("tool_search"), tool_definition("weather")]
    }

    fn model_definitions(
        &self,
        activated: &std::collections::BTreeSet<ToolName>,
    ) -> Result<Vec<ToolDefinition>, CoreError> {
        let mut definitions = vec![tool_definition("tool_search")];
        if activated.contains(&ToolName::new("weather").unwrap()) {
            definitions.push(tool_definition("weather"));
        }
        Ok(definitions)
    }

    fn activated_tool_names(
        &self,
        call: &ToolCall,
        result: &str,
    ) -> Result<Vec<ToolName>, CoreError> {
        if call.name.as_str() == "tool_search" && result == "weather loaded" {
            Ok(vec![ToolName::new("weather").unwrap()])
        } else {
            Ok(Vec::new())
        }
    }

    fn prepare(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        Ok(ActionReviewRequest::new(
            ResolvedAction::new(
                ActionDigest::from_canonical_bytes(serde_json::to_vec(call).unwrap()),
                ActionKind::LocalProcess(zeta_action_policy::ProcessInvocationKind::Direct),
                "search tools",
                CapabilitySet::new([]),
            ),
            ActionProvenance::new(ActionSource::BuiltInTool, "tool-search"),
            SandboxCompatibility::Supported(SandboxPolicy::new(
                FileSystemAccess::ReadOnly,
                NetworkAccess::Denied,
            )),
            ActionPolicyRevision::new("test-policy"),
        ))
    }

    fn execute(
        &self,
        call: &ToolCall,
        _: &ToolAuthorization,
        _: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        assert_eq!(call.name.as_str(), "tool_search");
        Ok(ToolExecutionOutput::Success("weather loaded".into()))
    }
}

fn tool_definition(name: &str) -> ToolDefinition {
    ToolDefinition {
        name: ToolName::new(name).unwrap(),
        description: name.into(),
        parameters: json!({"type": "object"}),
        strict: true,
    }
}

#[derive(Default)]
struct BlockingTool {
    entered: AtomicBool,
    was_cancelled: AtomicBool,
    entered_lock: Mutex<()>,
    entered_changed: Condvar,
}

struct InteractiveTool;

impl ToolService for InteractiveTool {
    fn definitions(&self) -> Vec<ToolDefinition> {
        vec![tool_definition("interactive")]
    }

    fn prepare(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        Ok(ActionReviewRequest::new(
            ResolvedAction::new(
                ActionDigest::from_canonical_bytes(serde_json::to_vec(call).unwrap()),
                ActionKind::SystemOperation,
                "request user input",
                CapabilitySet::new([]),
            ),
            ActionProvenance::new(ActionSource::BuiltInTool, "interactive"),
            SandboxCompatibility::Supported(SandboxPolicy::new(
                FileSystemAccess::ReadOnly,
                NetworkAccess::Denied,
            )),
            ActionPolicyRevision::new("test-policy"),
        ))
    }

    fn execute(
        &self,
        _: &ToolCall,
        _: &ToolAuthorization,
        _: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        Err(CoreError::Execution(
            "interactive execution context is required".into(),
        ))
    }

    fn execute_streaming_with_facts_and_interactions(
        &self,
        _: &ToolCall,
        _: &ToolAuthorization,
        _: &CancellationToken,
        _: &ToolExecutionFacts,
        interactions: Arc<dyn ToolInteractionService>,
        _: &mut dyn ToolOutputSink,
    ) -> Result<ToolExecutionOutput, CoreError> {
        let outcome = interactions.request_user_input(RequestUserInput {
            questions: vec![UserInputQuestion {
                id: "city".into(),
                header: "City".into(),
                question: "Which city?".into(),
                options: Vec::new(),
                allow_free_form: true,
            }],
        })?;
        match outcome {
            ToolUserInputOutcome::Answered(response) => Ok(ToolExecutionOutput::Success(
                response.answers["city"].value.clone(),
            )),
            ToolUserInputOutcome::Cancelled(reason) => Ok(ToolExecutionOutput::Failure(format!(
                "interaction cancelled: {reason:?}"
            ))),
        }
    }
}

impl BlockingTool {
    fn wait_until_entered(&self) {
        let deadline = Instant::now() + Duration::from_secs(1);
        let mut lock = self.entered_lock.lock().unwrap();
        while !self.entered.load(Ordering::Relaxed) {
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(!remaining.is_zero(), "blocking tool did not start");
            let (next_lock, _) = self.entered_changed.wait_timeout(lock, remaining).unwrap();
            lock = next_lock;
        }
    }
}

impl ToolService for BlockingTool {
    fn definitions(&self) -> Vec<ToolDefinition> {
        vec![ToolDefinition {
            name: ToolName::new("blocking").unwrap(),
            description: "Wait for cancellation".into(),
            parameters: json!({"type": "object"}),
            strict: true,
        }]
    }

    fn prepare(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        Ok(ActionReviewRequest::new(
            ResolvedAction::new(
                ActionDigest::from_canonical_bytes(serde_json::to_vec(call).unwrap()),
                ActionKind::LocalProcess(zeta_action_policy::ProcessInvocationKind::Direct),
                "wait for cancellation",
                CapabilitySet::new([]),
            ),
            ActionProvenance::new(ActionSource::BuiltInTool, "blocking"),
            SandboxCompatibility::Supported(SandboxPolicy::new(
                FileSystemAccess::ReadOnly,
                NetworkAccess::Denied,
            )),
            ActionPolicyRevision::new("test-policy"),
        ))
    }

    fn execute(
        &self,
        _: &ToolCall,
        _: &ToolAuthorization,
        cancellation: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        self.entered.store(true, Ordering::Relaxed);
        self.entered_changed.notify_all();
        while !cancellation.is_cancelled() {
            thread::sleep(Duration::from_millis(1));
        }
        self.was_cancelled.store(true, Ordering::Relaxed);
        Err(CoreError::Cancelled(
            "cancelled during tool execution".into(),
        ))
    }
}

impl ToolService for WeatherTool {
    fn definitions(&self) -> Vec<ToolDefinition> {
        vec![ToolDefinition {
            name: ToolName::new("weather").unwrap(),
            description: "Get weather".into(),
            parameters: json!({"type": "object"}),
            strict: true,
        }]
    }

    fn prepare(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        let capabilities =
            CapabilitySet::new([Capability::new(CapabilityKind::Network, "weather.example")]);
        let sandbox = SandboxPolicy::new(FileSystemAccess::ReadOnly, NetworkAccess::Allowed);
        Ok(ActionReviewRequest::new(
            ResolvedAction::new(
                ActionDigest::from_canonical_bytes(serde_json::to_vec(call).unwrap()),
                ActionKind::NetworkRequest,
                "read weather",
                capabilities,
            ),
            ActionProvenance::new(ActionSource::BuiltInTool, "weather"),
            SandboxCompatibility::Supported(sandbox),
            ActionPolicyRevision::new("test-policy"),
        ))
    }

    fn execute(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        _: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        assert!(matches!(authorization, ToolAuthorization::Sandboxed(_)));
        assert_eq!(call.arguments["city"], "Paris");
        Ok(ToolExecutionOutput::Success("sunny".into()))
    }
}

impl ToolService for FailingWeatherTool {
    fn definitions(&self) -> Vec<ToolDefinition> {
        WeatherTool.definitions()
    }

    fn prepare(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        WeatherTool.prepare(call)
    }

    fn execute(
        &self,
        _: &ToolCall,
        authorization: &ToolAuthorization,
        _: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        assert!(matches!(authorization, ToolAuthorization::Sandboxed(_)));
        Ok(ToolExecutionOutput::Failure("weather unavailable".into()))
    }
}

struct SandboxActionPolicyService;

impl ActionPolicyService for SandboxActionPolicyService {
    fn revision(&self) -> String {
        "test-policy-v1".into()
    }

    fn decide(
        &self,
        request: &ActionReviewRequest,
        _: &zeta_async_utils::CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        let SandboxCompatibility::Supported(policy) = request.sandbox() else {
            return Err(CoreError::Policy("test action has no sandbox".into()));
        };
        Ok(ExecutionDecision::RunSandboxed(*policy))
    }
}

fn started_turn() -> (Arc<ThreadController>, ThreadId, TurnId) {
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    let thread_id = ThreadId::new("thread").unwrap();
    threads
        .create_thread(CreateThreadRequest {
            session_id: SessionId::new("session").unwrap(),
            thread_id: thread_id.clone(),
            title: "test".into(),
        })
        .unwrap();
    let turn_id = threads
        .start_turn(
            &thread_id,
            StartTurnRequest {
                command_id: CommandId::new("start").unwrap(),
                expected_sequence: SequenceExpectation::Any,
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "hello".into(),
                }],
            },
        )
        .unwrap()
        .turn_id;
    (threads, thread_id, turn_id)
}

fn started_turn_with_history() -> (Arc<ThreadController>, ThreadId, TurnId) {
    let (threads, thread_id, history_turn_id) = started_turn();
    threads
        .complete_turn(&thread_id, &history_turn_id, "durable history ".repeat(400))
        .unwrap();
    let turn_id = threads
        .start_turn(
            &thread_id,
            StartTurnRequest {
                command_id: CommandId::new("start-overflow-turn").unwrap(),
                expected_sequence: SequenceExpectation::Any,
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "continue after durable history".into(),
                }],
            },
        )
        .unwrap()
        .turn_id;
    (threads, thread_id, turn_id)
}

fn text_response(text: &str) -> ModelResponse {
    ModelResponse {
        output: vec![ResponseItem::Text(text.into())],
        usage: None,
        stop_reason: StopReason::Completed,
    }
}

fn request_contains(request: &ModelRequest, expected: &str) -> bool {
    request.input.iter().any(|input| match input {
        InputItem::Message(message) => message
            .content
            .iter()
            .any(|content| matches!(content, ContentPart::Text(text) if text.contains(expected))),
        InputItem::ToolResult(_) => false,
    })
}

fn wait_for_turn_status(
    threads: &ThreadController,
    thread_id: &ThreadId,
    turn_id: &zeta_protocol::TurnId,
    expected: TurnStatus,
) {
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let snapshot = threads.read_thread(thread_id).unwrap();
        if snapshot
            .turns
            .iter()
            .find(|turn| &turn.turn_id == turn_id)
            .is_some_and(|turn| turn.status == expected)
        {
            return;
        }
        assert!(Instant::now() < deadline, "Turn did not reach {expected:?}");
        thread::sleep(Duration::from_millis(1));
    }
}

fn wait_for_flag(flag: &AtomicBool, message: &str) {
    let deadline = Instant::now() + Duration::from_secs(1);
    while !flag.load(Ordering::Relaxed) {
        assert!(Instant::now() < deadline, "{message}");
        thread::sleep(Duration::from_millis(1));
    }
}
