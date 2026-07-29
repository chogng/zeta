use super::*;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread;
use std::time::Duration;
use zeta_protocol::{
    ActionApprovalCapability, ActionApprovalCapabilityKind, ActionApprovalDecision,
    ActionApprovalRequest, ActionApprovalResponse, AgentRequest, AgentResponse, CommandId,
    InteractionDeadline, RequestId, RequestUserInput, RequestUserInputResponse, SessionId,
    StableTurnError, StableTurnErrorCode, ThreadEvent, ThreadId, ThreadItem, ToolName, TurnId,
    UserInput, UserInputQuestion,
};
use zeta_thread_store::StoredEvent;

static NEXT_THREAD: AtomicU64 = AtomicU64::new(1);

fn start_request(key: &str) -> StartTurnRequest {
    StartTurnRequest {
        command_id: CommandId::new(key).expect("test ID is non-empty"),
        expected_sequence: SequenceExpectation::Any,
        model: None,
        input: vec![UserInput::Text {
            text: "hello".into(),
        }],
    }
}

fn create_thread(controller: &ThreadController, title: &str) -> ThreadId {
    let thread_id = ThreadId::new(format!(
        "thread_{}",
        NEXT_THREAD.fetch_add(1, Ordering::Relaxed)
    ))
    .expect("test ID is non-empty");
    controller
        .create_thread(CreateThreadRequest {
            session_id: SessionId::new("session_1").expect("test ID is non-empty"),
            thread_id: thread_id.clone(),
            title: title.into(),
        })
        .unwrap();
    thread_id
}

fn start_turn(threads: &ThreadController, thread: &ThreadId, key: &str) -> TurnId {
    threads
        .start_turn(thread, start_request(key))
        .unwrap()
        .turn_id
}

fn user_input_interaction() -> AgentRequest {
    AgentRequest::UserInput {
        request: RequestUserInput {
            questions: vec![UserInputQuestion {
                id: "answer".into(),
                header: "Answer".into(),
                question: "Continue?".into(),
                options: Vec::new(),
                allow_free_form: true,
            }],
        },
    }
}

fn user_input_response() -> AgentResponse {
    AgentResponse::UserInput {
        response: RequestUserInputResponse {
            answers: BTreeMap::new(),
        },
    }
}

fn approval_interaction() -> AgentRequest {
    AgentRequest::Approval {
        request: ActionApprovalRequest {
            action_digest: "a".repeat(64),
            policy_revision: "policy-1".into(),
            capabilities: vec![ActionApprovalCapability {
                kind: ActionApprovalCapabilityKind::Network,
                scope: "api.example.com".into(),
            }],
            reason: "network requires unsandboxed execution".into(),
            sandbox_denial: None,
        },
    }
}

fn approval_response(decision: ActionApprovalDecision) -> AgentResponse {
    AgentResponse::Approval {
        response: ActionApprovalResponse { decision },
    }
}

#[test]
fn completion_is_durable_before_snapshot_exposes_it() {
    let journal = Arc::new(InMemoryThreadStore::default());
    let threads = ThreadController::with_store(journal.clone());
    let thread = create_thread(&threads, "test");
    let turn = start_turn(&threads, &thread, "completion");
    threads
        .complete_turn(&thread, &turn, "answer".into())
        .unwrap();
    let snapshot = threads.read_thread(&thread).unwrap();
    assert_eq!(snapshot.turns[0].turn_id, turn);
    assert_eq!(snapshot.turns[0].status, TurnStatus::Completed);
    assert_eq!(
        journal.events().last().unwrap().event.kind(),
        "turn.completed"
    );
}

#[test]
fn failure_is_durable_before_snapshot_exposes_it() {
    let journal = Arc::new(InMemoryThreadStore::default());
    let threads = ThreadController::with_store(journal.clone());
    let thread = create_thread(&threads, "test");
    let turn = start_turn(&threads, &thread, "failure");
    threads
        .fail_turn(&thread, &turn, StableTurnError::model_invocation_failed())
        .unwrap();
    let snapshot = threads.read_thread(&thread).unwrap();
    assert_eq!(snapshot.turns[0].status, TurnStatus::Failed);
    assert_eq!(
        snapshot.turns[0].failure.as_ref().unwrap().code,
        StableTurnErrorCode::ModelInvocationFailed
    );
}

#[test]
fn terminal_turn_cannot_restart() {
    assert!(
        crate::state::transition_turn_status(TurnStatus::Completed, TurnStatus::Running).is_err()
    );
}

#[test]
fn accepted_turn_can_be_cancelled_before_it_starts_running() {
    assert_eq!(
        crate::state::transition_turn_status(TurnStatus::Created, TurnStatus::Cancelling).unwrap(),
        TurnStatus::Cancelling
    );
}

#[test]
fn item_and_tool_terminal_states_are_distinct() {
    assert_eq!(
        ItemStatus::Created
            .transition(ItemStatus::InProgress)
            .unwrap(),
        ItemStatus::InProgress
    );
    assert_eq!(
        ToolCallStatus::AwaitingApproval
            .transition(ToolCallStatus::Declined)
            .unwrap(),
        ToolCallStatus::Declined
    );
    assert!(
        ToolCallStatus::Declined
            .transition(ToolCallStatus::Running)
            .is_err()
    );
}

struct ToggleStore {
    reject_writes: AtomicBool,
}

impl ThreadStore for ToggleStore {
    fn list_thread_ids(&self) -> Result<Vec<ThreadId>, ThreadStoreError> {
        Ok(Vec::new())
    }

    fn load(&self, _: &ThreadId) -> Result<Vec<StoredEvent>, ThreadStoreError> {
        Ok(Vec::new())
    }

    fn append_batch(
        &self,
        batch: &ThreadEventBatch,
    ) -> Result<AppendBatchResult, ThreadStoreError> {
        if self.reject_writes.load(Ordering::Relaxed) {
            Err(ThreadStoreError::Storage("simulated write failure".into()))
        } else {
            validate_append_batch(batch, batch.expected_sequence)
        }
    }
}

#[test]
fn failed_writes_do_not_expose_uncommitted_projection_changes() {
    let store = Arc::new(ToggleStore {
        reject_writes: AtomicBool::new(false),
    });
    let threads = ThreadController::with_store(store.clone());
    let thread = create_thread(&threads, "test");
    store.reject_writes.store(true, Ordering::Relaxed);

    assert!(
        threads
            .start_turn(&thread, start_request("failed-start"))
            .is_err()
    );
    assert!(threads.read_thread(&thread).unwrap().turns.is_empty());
}

#[test]
fn failed_thread_creation_does_not_register_a_projection() {
    let store = Arc::new(ToggleStore {
        reject_writes: AtomicBool::new(true),
    });
    let threads = ThreadController::with_store(store);
    let thread_id = ThreadId::new("failed").expect("test ID is non-empty");

    assert!(
        threads
            .create_thread(CreateThreadRequest {
                session_id: SessionId::new("session_1").expect("test ID is non-empty"),
                thread_id,
                title: "test".into(),
            })
            .is_err()
    );
    assert!(threads.list_threads().unwrap().is_empty());
}

#[test]
fn one_thread_durable_commit_does_not_block_another_thread() {
    let store = Arc::new(PerThreadBlockingStore::default());
    let threads = Arc::new(ThreadController::with_store(store.clone()));
    let blocked_thread = create_thread(&threads, "blocked");
    let independent_thread = create_thread(&threads, "independent");
    store.block_next_append_for(blocked_thread.clone());
    let blocked_threads = threads.clone();
    let blocked_thread_for_task = blocked_thread.clone();
    let blocked = thread::spawn(move || {
        blocked_threads
            .start_turn(&blocked_thread_for_task, start_request("blocked-start"))
            .unwrap();
    });
    store.wait_until_blocked();
    let independent_threads = threads.clone();
    let (completed_tx, completed_rx) = mpsc::channel();

    thread::spawn(move || {
        let result =
            independent_threads.start_turn(&independent_thread, start_request("independent-start"));
        completed_tx.send(result).unwrap();
    });

    let independent = completed_rx.recv_timeout(Duration::from_secs(1));
    store.release();
    blocked.join().unwrap();
    independent
        .expect("independent Thread commit was blocked")
        .unwrap();
}

#[test]
fn recovery_interrupts_non_terminal_turns() {
    let journal = Arc::new(InMemoryThreadStore::default());
    let original = ThreadController::with_store(journal.clone());
    let thread = create_thread(&original, "recover me");
    let turn = start_turn(&original, &thread, "recovery");
    let recovered = ThreadController::with_store(journal.clone());
    let snapshot = recovered.recover_thread(&thread).unwrap();
    assert_eq!(snapshot.title, "recover me");
    assert_eq!(snapshot.turns[0].turn_id, turn);
    assert_eq!(snapshot.turns[0].status, TurnStatus::Interrupted);
}

#[derive(Default)]
struct PerThreadBlockingStore {
    inner: InMemoryThreadStore,
    state: Mutex<PerThreadBlockingState>,
    changed: Condvar,
}

#[derive(Default)]
struct PerThreadBlockingState {
    blocked_thread: Option<ThreadId>,
    entered: bool,
    released: bool,
}

impl PerThreadBlockingStore {
    fn block_next_append_for(&self, thread_id: ThreadId) {
        let mut state = self.state.lock().unwrap();
        state.blocked_thread = Some(thread_id);
        state.entered = false;
        state.released = false;
    }

    fn wait_until_blocked(&self) {
        let mut state = self.state.lock().unwrap();
        while !state.entered {
            state = self.changed.wait(state).unwrap();
        }
    }

    fn release(&self) {
        let mut state = self.state.lock().unwrap();
        state.released = true;
        self.changed.notify_all();
    }
}

impl ThreadStore for PerThreadBlockingStore {
    fn list_thread_ids(&self) -> Result<Vec<ThreadId>, ThreadStoreError> {
        self.inner.list_thread_ids()
    }

    fn load(&self, thread_id: &ThreadId) -> Result<Vec<StoredEvent>, ThreadStoreError> {
        self.inner.load(thread_id)
    }

    fn append_batch(
        &self,
        batch: &ThreadEventBatch,
    ) -> Result<AppendBatchResult, ThreadStoreError> {
        let should_block = self
            .state
            .lock()
            .unwrap()
            .blocked_thread
            .as_ref()
            .is_some_and(|thread_id| thread_id == &batch.thread_id);
        if should_block {
            let mut state = self.state.lock().unwrap();
            state.blocked_thread = None;
            state.entered = true;
            self.changed.notify_all();
            while !state.released {
                state = self.changed.wait(state).unwrap();
            }
        }
        self.inner.append_batch(batch)
    }
}

#[test]
fn waiting_interaction_survives_recovery_and_resolves_idempotently() {
    let store = Arc::new(InMemoryThreadStore::default());
    let original = ThreadController::with_store(store.clone());
    let thread = create_thread(&original, "interaction");
    let turn = start_turn(&original, &thread, "interaction-start");
    let requested = original
        .request_turn_interaction(
            &thread,
            &turn,
            RequestTurnInteraction {
                request_id: RequestId::new("request_1").expect("test ID is non-empty"),
                item_id: None,
                request: user_input_interaction(),
                deadline: Some(InteractionDeadline {
                    expires_at_unix_ms: 12_345,
                }),
            },
        )
        .unwrap();
    assert_eq!(requested.sequence, 5);
    assert_eq!(
        original.read_thread(&thread).unwrap().turns[0].status,
        TurnStatus::WaitingForUserInput
    );

    let recovered = ThreadController::with_store(store.clone());
    let snapshot = recovered.recover_thread(&thread).unwrap();
    assert_eq!(snapshot.turns[0].status, TurnStatus::WaitingForUserInput);
    assert_eq!(
        snapshot.turns[0]
            .pending_interaction
            .as_ref()
            .unwrap()
            .deadline,
        Some(InteractionDeadline {
            expires_at_unix_ms: 12_345,
        })
    );

    let resolve = || ResolveTurnInteractionRequest {
        command_id: CommandId::new("resolve_1").expect("test ID is non-empty"),
        expected_sequence: SequenceExpectation::Exact(requested.sequence),
        turn_id: turn.clone(),
        request_id: RequestId::new("request_1").expect("test ID is non-empty"),
        response: user_input_response(),
    };
    let resolved = recovered
        .resolve_turn_interaction(&thread, resolve())
        .unwrap();
    let replayed = recovered
        .resolve_turn_interaction(&thread, resolve())
        .unwrap();

    assert_eq!(
        resolved.disposition,
        ResolveTurnInteractionDisposition::Resolved
    );
    assert_eq!(
        replayed.disposition,
        ResolveTurnInteractionDisposition::Replayed
    );
    assert_eq!(
        recovered.read_thread(&thread).unwrap().turns[0].status,
        TurnStatus::Running
    );
    assert!(
        recovered.read_thread(&thread).unwrap().turns[0]
            .pending_interaction
            .is_none()
    );
    assert!(matches!(
        store.events().last().unwrap().event,
        ThreadEvent::InteractionResolved { .. }
    ));
}

#[test]
fn approval_interaction_is_durable_bound_and_recoverable() {
    let store = Arc::new(InMemoryThreadStore::default());
    let original = ThreadController::with_store(store.clone());
    let thread = create_thread(&original, "approval");
    let turn = start_turn(&original, &thread, "approval-start");
    let requested = original
        .request_turn_interaction(
            &thread,
            &turn,
            RequestTurnInteraction {
                request_id: RequestId::new("approval_1").unwrap(),
                item_id: None,
                request: approval_interaction(),
                deadline: None,
            },
        )
        .unwrap();
    assert_eq!(
        original.read_thread(&thread).unwrap().turns[0].status,
        TurnStatus::WaitingForApproval
    );

    let recovered = ThreadController::with_store(store.clone());
    let snapshot = recovered.recover_thread(&thread).unwrap();
    assert_eq!(snapshot.turns[0].status, TurnStatus::WaitingForApproval);
    assert_eq!(
        snapshot.turns[0]
            .pending_interaction
            .as_ref()
            .unwrap()
            .request,
        approval_interaction()
    );

    recovered
        .resolve_turn_interaction(
            &thread,
            ResolveTurnInteractionRequest {
                command_id: CommandId::new("approve_1").unwrap(),
                expected_sequence: SequenceExpectation::Exact(requested.sequence),
                turn_id: turn,
                request_id: RequestId::new("approval_1").unwrap(),
                response: approval_response(ActionApprovalDecision::ApproveOnce),
            },
        )
        .unwrap();
    assert_eq!(
        recovered.read_thread(&thread).unwrap().turns[0].status,
        TurnStatus::Running
    );
    assert!(matches!(
        &store.events().last().unwrap().event,
        ThreadEvent::InteractionResolved {
            response: AgentResponse::Approval {
                response: ActionApprovalResponse {
                    decision: ActionApprovalDecision::ApproveOnce
                }
            },
            ..
        }
    ));
}

#[test]
fn interaction_resolution_rejects_a_response_for_another_request() {
    let threads = ThreadController::with_store(Arc::new(InMemoryThreadStore::default()));
    let thread = create_thread(&threads, "interaction");
    let turn = start_turn(&threads, &thread, "interaction-start");
    let requested = threads
        .request_turn_interaction(
            &thread,
            &turn,
            RequestTurnInteraction {
                request_id: RequestId::new("request_1").expect("test ID is non-empty"),
                item_id: None,
                request: user_input_interaction(),
                deadline: None,
            },
        )
        .unwrap();

    assert!(
        threads
            .resolve_turn_interaction(
                &thread,
                ResolveTurnInteractionRequest {
                    command_id: CommandId::new("wrong-request").expect("test ID is non-empty"),
                    expected_sequence: SequenceExpectation::Exact(requested.sequence),
                    turn_id: turn,
                    request_id: RequestId::new("request_2").expect("test ID is non-empty"),
                    response: user_input_response(),
                },
            )
            .is_err()
    );
    assert_eq!(
        threads.read_thread(&thread).unwrap().turns[0].status,
        TurnStatus::WaitingForUserInput
    );
}

#[test]
fn interrupt_closes_an_outstanding_interaction_before_the_turn() {
    let store = Arc::new(InMemoryThreadStore::default());
    let threads = ThreadController::with_store(store.clone());
    let thread = create_thread(&threads, "interaction");
    let turn = start_turn(&threads, &thread, "interaction-start");
    threads
        .request_turn_interaction(
            &thread,
            &turn,
            RequestTurnInteraction {
                request_id: RequestId::new("request_1").expect("test ID is non-empty"),
                item_id: None,
                request: user_input_interaction(),
                deadline: None,
            },
        )
        .unwrap();

    threads
        .interrupt_turn(
            &thread,
            InterruptTurnRequest {
                command_id: CommandId::new("interrupt-wait").expect("test ID is non-empty"),
                expected_sequence: SequenceExpectation::Any,
                turn_id: turn.clone(),
            },
        )
        .unwrap();

    let events = store.events();
    assert!(matches!(
        events[events.len() - 3].event,
        ThreadEvent::InteractionCancelled { .. }
    ));
    assert_eq!(
        threads.read_thread(&thread).unwrap().turns[0].status,
        TurnStatus::Interrupted
    );
}

#[test]
fn start_turn_replays_typed_command_without_creating_another_turn() {
    let store = Arc::new(InMemoryThreadStore::default());
    let threads = ThreadController::with_store(store);
    let thread = create_thread(&threads, "test");

    let created = threads
        .start_turn(&thread, start_request("replay"))
        .unwrap();
    let replayed = threads
        .start_turn(&thread, start_request("replay"))
        .unwrap();

    assert_eq!(created.disposition, StartTurnDisposition::Created);
    assert_eq!(replayed.disposition, StartTurnDisposition::Replayed);
    assert_eq!(replayed.turn_id, created.turn_id);
    assert_eq!(threads.read_thread(&thread).unwrap().turns.len(), 1);
}

#[test]
fn typed_command_rejects_reusing_an_id_with_different_input() {
    let threads = ThreadController::with_store(Arc::new(InMemoryThreadStore::default()));
    let thread = create_thread(&threads, "test");
    threads
        .start_turn(&thread, start_request("conflict"))
        .unwrap();
    let conflicting = StartTurnRequest {
        command_id: CommandId::new("conflict").expect("test ID is non-empty"),
        expected_sequence: SequenceExpectation::Any,
        model: None,
        input: vec![UserInput::Text {
            text: "different".into(),
        }],
    };

    assert!(matches!(
        threads.start_turn(&thread, conflicting),
        Err(CoreError::CommandConflict)
    ));
}

#[test]
fn start_turn_snapshots_the_selected_model() {
    let threads = ThreadController::with_store(Arc::new(InMemoryThreadStore::default()));
    let thread = create_thread(&threads, "model");
    let model = zeta_protocol::ModelRef::new(
        zeta_protocol::ProviderId::new("openai").unwrap(),
        zeta_protocol::ModelId::new("gpt-5.6").unwrap(),
    );
    let mut request = start_request("model-turn");
    request.model = Some(model.clone());

    let started = threads.start_turn(&thread, request).unwrap();
    let snapshot = threads.read_thread(&thread).unwrap();

    assert_eq!(
        snapshot
            .turns
            .iter()
            .find(|turn| turn.turn_id == started.turn_id)
            .and_then(|turn| turn.model.clone()),
        Some(model)
    );
}

#[test]
fn recovered_command_replays_without_creating_another_turn() {
    let store = Arc::new(InMemoryThreadStore::default());
    let original = ThreadController::with_store(store.clone());
    let thread = create_thread(&original, "test");
    let created = original
        .start_turn(&thread, start_request("recovered-replay"))
        .unwrap();
    original
        .complete_turn(&thread, &created.turn_id, "answer".into())
        .unwrap();

    let recovered = ThreadController::with_store(store);
    recovered.recover_thread(&thread).unwrap();
    let replayed = recovered
        .start_turn(&thread, start_request("recovered-replay"))
        .unwrap();

    assert_eq!(replayed.disposition, StartTurnDisposition::Replayed);
    assert_eq!(replayed.turn_id, created.turn_id);
}

#[test]
fn interrupt_turn_is_a_retry_safe_typed_command() {
    let threads = ThreadController::with_store(Arc::new(InMemoryThreadStore::default()));
    let thread = create_thread(&threads, "test");
    let turn = start_turn(&threads, &thread, "turn");
    let request = || InterruptTurnRequest {
        command_id: CommandId::new("interrupt").expect("test ID is non-empty"),
        expected_sequence: SequenceExpectation::Any,
        turn_id: turn.clone(),
    };

    let interrupted = threads.interrupt_turn(&thread, request()).unwrap();
    let replayed = threads.interrupt_turn(&thread, request()).unwrap();

    assert_eq!(
        interrupted.disposition,
        InterruptTurnDisposition::Interrupted
    );
    assert_eq!(replayed.disposition, InterruptTurnDisposition::Replayed);
    assert_eq!(
        threads.read_thread(&thread).unwrap().turns[0].status,
        TurnStatus::Interrupted
    );
}

#[test]
fn durable_projection_contains_messages_tools_and_session_identity() {
    let store = Arc::new(InMemoryThreadStore::default());
    let original = ThreadController::with_store(store.clone());
    let thread = create_thread(&original, "test");
    let turn = start_turn(&original, &thread, "items");
    let call = original
        .record_tool_call(
            &thread,
            &turn,
            RecordToolCallRequest {
                name: ToolName::new("search").expect("test tool name is valid"),
                arguments_json: r#"{"query":"zeta"}"#.into(),
            },
        )
        .unwrap();
    original
        .record_tool_result(
            &thread,
            &turn,
            RecordToolResultRequest {
                tool_call_id: call.tool_call_id,
                output: ToolCallOutput::Success("result".into()),
            },
        )
        .unwrap();
    original
        .complete_turn(&thread, &turn, "answer".into())
        .unwrap();

    let snapshot = ThreadController::with_store(store)
        .recover_thread(&thread)
        .unwrap();
    assert_eq!(
        snapshot.public_thread().session_id,
        SessionId::new("session_1").expect("test ID is non-empty")
    );
    assert!(matches!(
        &snapshot.items[0],
        ThreadItem::UserMessage { text, .. } if text == "hello"
    ));
    assert!(matches!(
        &snapshot.items[1],
        ThreadItem::ToolCall { name, .. } if name.as_str() == "search"
    ));
    assert!(matches!(
        &snapshot.items[2],
        ThreadItem::ToolResult {
            text,
            is_error: false,
            ..
        } if text == "result"
    ));
}

#[test]
fn start_turn_persists_ordered_text_and_image_items() {
    let threads = ThreadController::with_store(Arc::new(InMemoryThreadStore::default()));
    let thread = create_thread(&threads, "image");
    let image_url = "data:image/png;base64,iVBORw0KGgpwYXlsb2Fk";

    let result = threads
        .start_turn(
            &thread,
            StartTurnRequest {
                command_id: CommandId::new("image-turn").expect("test ID is non-empty"),
                expected_sequence: SequenceExpectation::Any,
                model: None,
                input: vec![
                    UserInput::Text {
                        text: "describe".into(),
                    },
                    UserInput::Image {
                        url: image_url.into(),
                    },
                ],
            },
        )
        .unwrap();
    let snapshot = threads.read_thread(&thread).unwrap();

    assert!(matches!(
        &snapshot.items[0],
        ThreadItem::UserMessage { turn_id, text, .. }
            if turn_id == &result.turn_id && text == "describe"
    ));
    assert!(matches!(
        &snapshot.items[1],
        ThreadItem::UserImage { turn_id, url, .. }
            if turn_id == &result.turn_id && url == image_url
    ));
}
