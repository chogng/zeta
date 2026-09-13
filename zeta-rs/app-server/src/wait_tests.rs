use super::compose_extension_tools;
use crate::tool_composition::combine_tool_ports;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;
use zeta_async_utils::CancellationSource;
use zeta_async_utils::CancellationToken;
use zeta_core::CoreError;
use zeta_core::InMemoryThreadStore;
use zeta_core::ModelSelection;
use zeta_core::ModelService;
use zeta_core::SequenceExpectation;
use zeta_core::StartThreadRequest;
use zeta_core::StartTurnRequest;
use zeta_core::ThreadController;
use zeta_core::TurnExecutor;
use zeta_extension_api::ExtensionItemStore;
use zeta_extension_api::ExtensionRegistryBuilder;
use zeta_extension_api::ExtensionState;
use zeta_protocol::CommandId;
use zeta_protocol::ModelRequest;
use zeta_protocol::ModelResponse;
use zeta_protocol::ResponseItem;
use zeta_protocol::StopReason;
use zeta_protocol::ToolCall;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolName;
use zeta_protocol::TurnStatus;

#[test]
fn runtime_wait_resumes_model_once_and_cancelled_wait_never_resumes_it() {
    for duration_ms in [0, 60_000] {
        let threads = Arc::new(ThreadController::with_store(Arc::new(
            InMemoryThreadStore::default(),
        )));
        let items = Arc::new(ExtensionItemStore::new(Arc::new(ExtensionState::default())));
        let mut builder = ExtensionRegistryBuilder::new();
        sleep::install(&mut builder, items);
        let registry = Arc::new(builder.build());
        let port = compose_extension_tools(&registry).unwrap().unwrap();
        let port = combine_tool_ports(vec![port]).unwrap().unwrap();
        threads.install_extensions(registry).unwrap();
        let parent = threads
            .start_thread(
                &zeta_core::NoThreadWorktreeBinder,
                StartThreadRequest {
                    agent_id: None,
                    agent: None,
                    command_id: CommandId::new("start").unwrap(),
                    title: "wait".into(),
                },
            )
            .unwrap();
        let turn = threads
            .start_turn(
                &parent.thread_id,
                StartTurnRequest {
                    command_id: CommandId::new("turn").unwrap(),
                    expected_sequence: SequenceExpectation::Any,
                    model: None,
                    kind: zeta_protocol::TurnKind::Coding,
                    instructions: zeta_prompts::AGENT_INSTRUCTIONS.freeze(),
                    policy_revision: port.policy.revision(),
                    approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                    tool_mode: zeta_protocol::ToolMode::Direct,
                    tool_profile: None,
                    activated_skills: Vec::new(),
                    input: vec![zeta_protocol::UserInput::Text {
                        text: "wait then continue".into(),
                    }],
                },
            )
            .unwrap();
        let model = Arc::new(WaitModel {
            duration_ms,
            calls: AtomicUsize::new(0),
        });
        let executor = TurnExecutor::new(threads.clone(), model.clone(), port.tools, port.policy);
        executor.start(&parent.thread_id, &turn.turn_id).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let token = CancellationSource::new().token();
        loop {
            let changed = threads.thread_changed(&parent.thread_id).unwrap();
            let snapshot = threads.read_thread(&parent.thread_id).unwrap();
            assert_ne!(
                snapshot.turns[0].status,
                TurnStatus::Failed,
                "wait failed: {:?}; items: {:?}",
                snapshot.turns,
                snapshot.items
            );
            if duration_ms == 0 && snapshot.turns[0].status == TurnStatus::Completed {
                assert_eq!(model.calls.load(Ordering::SeqCst), 2);
                assert!(snapshot.items.iter().any(|item| matches!(
                    item,
                    zeta_protocol::ThreadItem::ToolResult {
                        is_error: false,
                        ..
                    }
                )));
                break;
            }
            if duration_ms != 0 && !snapshot.started_tool_calls.is_empty() {
                assert_eq!(model.calls.load(Ordering::SeqCst), 1);
                threads
                    .interrupt_turn(
                        &parent.thread_id,
                        zeta_core::InterruptTurnRequest {
                            command_id: CommandId::new("cancel").unwrap(),
                            expected_sequence: SequenceExpectation::Any,
                            turn_id: turn.turn_id.clone(),
                        },
                    )
                    .unwrap();
                assert_eq!(model.calls.load(Ordering::SeqCst), 1);
                assert_eq!(
                    threads.read_thread(&parent.thread_id).unwrap().turns[0].status,
                    TurnStatus::Interrupted
                );
                break;
            }
            assert!(
                Instant::now() < deadline,
                "wait Turn did not reach expected state: {:?}",
                snapshot.turns[0].status
            );
            pollster::block_on(zeta_async_utils::wait_until(changed, deadline, &token)).unwrap();
        }
    }
}

struct WaitModel {
    duration_ms: u64,
    calls: AtomicUsize,
}

impl ModelService for WaitModel {
    fn invoke(
        &self,
        _: ModelSelection<'_>,
        _: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        let output = if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            vec![ResponseItem::ToolCall(ToolCall {
                id: ToolCallId::new("wait-call").unwrap(),
                name: ToolName::new("sleep").unwrap(),
                arguments: json!({"duration_ms": self.duration_ms}),
            })]
        } else {
            vec![ResponseItem::Text("done".into())]
        };
        Ok(ModelResponse {
            output,
            usage: None,
            billing: None,
            stop_reason: StopReason::Completed,
        })
    }
}
