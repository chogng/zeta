use super::*;
use crate::{
    CreateThreadRequest, InMemoryThreadStore, ModelService, ModelStreamSink, SequenceExpectation,
    StartTurnRequest, ThreadUpdateSink, ToolExecutionOutput, ToolService,
};
use serde_json::json;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use zeta_async_utils::CancellationSource;
use zeta_protocol::{
    CommandId, ContentPart, InputItem, ModelRequest, ModelResponse, ModelStreamEvent, ResponseItem,
    SessionId, StopReason, ThreadId, ThreadItem, ThreadUpdate, ThreadUpdateEnvelope, ToolCallId,
    ToolDefinition, ToolName, TurnStatus, UserInput,
};

#[test]
fn completes_a_text_turn_from_durable_context() {
    let (threads, thread_id, turn_id) = started_turn();
    let model = Arc::new(ScriptedModel::new([Ok(text_response("answer"))]));
    let executor = TurnExecutor::without_tools(threads.clone(), model.clone());

    let completion = executor
        .execute(&thread_id, &turn_id, &CancellationSource::new().token())
        .unwrap();

    assert!(matches!(
        completion.item,
        ThreadItem::AgentMessage { ref text, .. } if text == "answer"
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
        TurnExecutionLimits::default(),
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

struct ScriptedModel {
    responses: Mutex<VecDeque<Result<ModelResponse, CoreError>>>,
    requests: Mutex<Vec<ModelRequest>>,
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
    fn invoke(&self, _: &ModelRequest, _: &CancellationToken) -> Result<ModelResponse, CoreError> {
        unreachable!("stream is overridden")
    }

    fn stream(
        &self,
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
    fn invoke(&self, _: &ModelRequest, _: &CancellationToken) -> Result<ModelResponse, CoreError> {
        unreachable!("stream is overridden")
    }

    fn stream(
        &self,
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

impl ToolService for WeatherTool {
    fn definitions(&self) -> Vec<ToolDefinition> {
        vec![ToolDefinition {
            name: ToolName::new("weather").unwrap(),
            description: "Get weather".into(),
            parameters: json!({"type": "object"}),
            strict: true,
        }]
    }

    fn execute(
        &self,
        call: &ToolCall,
        _: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        assert_eq!(call.arguments["city"], "Paris");
        Ok(ToolExecutionOutput::Success("sunny".into()))
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
                input: vec![UserInput::Text {
                    text: "hello".into(),
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
