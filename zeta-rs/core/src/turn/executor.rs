use crate::{
    CompletedTurn, ContextAssembler, CoreError, ModelService, ModelStreamSink, NoThreadUpdates,
    NoTools, ThreadController, ThreadUpdateSink, ToolCallOutput, ToolExecutionOutput, ToolService,
};
use std::num::NonZeroUsize;
use std::sync::Arc;
use zeta_async_utils::{Cancellation, CancellationReason, CancellationToken};
use zeta_protocol::{
    ItemId, ModelResponse, ModelStreamEvent, ResponseItem, StableTurnError, StreamCursor,
    StreamInstanceId, ThreadId, ThreadItem, ThreadUpdate, ThreadUpdateEnvelope, ToolCall, TurnId,
    TurnStatus,
};

/// Bounds one Turn's model-tool loop.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TurnExecutionLimits {
    max_model_invocations: NonZeroUsize,
}

impl TurnExecutionLimits {
    pub fn new(max_model_invocations: NonZeroUsize) -> Self {
        Self {
            max_model_invocations,
        }
    }
}

impl Default for TurnExecutionLimits {
    fn default() -> Self {
        Self::new(NonZeroUsize::new(32).expect("default limit is non-zero"))
    }
}

/// Executes provider-independent model and tool steps for one already-started Turn.
///
/// The executor derives every invocation from the latest durable Thread snapshot. It delegates
/// all mutations to [`ThreadController`], so model and tool I/O never owns the Thread projection
/// lock or writer lease.
#[derive(Clone)]
pub struct TurnExecutor {
    threads: Arc<ThreadController>,
    model: Arc<dyn ModelService>,
    tools: Arc<dyn ToolService>,
    limits: TurnExecutionLimits,
    updates: Arc<dyn ThreadUpdateSink>,
}

impl TurnExecutor {
    pub fn new(
        threads: Arc<ThreadController>,
        model: Arc<dyn ModelService>,
        tools: Arc<dyn ToolService>,
        limits: TurnExecutionLimits,
    ) -> Self {
        Self {
            threads,
            model,
            tools,
            limits,
            updates: Arc::new(NoThreadUpdates),
        }
    }

    pub fn without_tools(threads: Arc<ThreadController>, model: Arc<dyn ModelService>) -> Self {
        Self::new(
            threads,
            model,
            Arc::new(NoTools),
            TurnExecutionLimits::default(),
        )
    }

    /// Adds the outer transport sink for durable and transient Thread updates.
    pub fn with_thread_updates(mut self, updates: Arc<dyn ThreadUpdateSink>) -> Self {
        self.updates = updates;
        self
    }

    /// Enqueues one Turn on its Thread-owned execution mailbox and returns after acceptance.
    ///
    /// The mailbox runs model and tool I/O away from the caller and from the Thread projection
    /// lock. Completion, failure, and cancellation remain durable Core transitions.
    pub fn start(&self, thread_id: &ThreadId, turn_id: &TurnId) -> Result<(), CoreError> {
        let executor = self.clone();
        let queued_thread_id = thread_id.clone();
        let queued_turn_id = turn_id.clone();
        self.threads
            .enqueue_turn_execution(thread_id, turn_id, move |cancellation| {
                let _ = executor.execute(&queued_thread_id, &queued_turn_id, &cancellation);
            })
    }

    pub fn execute(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        cancellation: &CancellationToken,
    ) -> Result<CompletedTurn, CoreError> {
        let sequence_before_execution = self
            .threads
            .read_thread(thread_id)
            .map(|snapshot| snapshot.sequence)
            .unwrap_or(0);
        let result = match self.execute_steps(thread_id, turn_id, cancellation) {
            Ok(completion) => Ok(completion),
            Err(ExecutionFailure::Cancelled(error)) => {
                self.threads.interrupt_execution(thread_id, turn_id)?;
                Err(error)
            }
            Err(ExecutionFailure::Failed { error, stable }) => {
                self.threads.fail_turn(thread_id, turn_id, stable)?;
                Err(error)
            }
        };
        self.publish_committed_after(thread_id, sequence_before_execution);
        result
    }

    fn execute_steps(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        cancellation: &CancellationToken,
    ) -> Result<CompletedTurn, ExecutionFailure> {
        check_cancellation(cancellation)?;
        self.require_running_turn(thread_id, turn_id)?;
        let tools = self.tools.definitions();

        for _ in 0..self.limits.max_model_invocations.get() {
            check_cancellation(cancellation)?;
            let snapshot = self
                .threads
                .read_thread(thread_id)
                .map_err(ExecutionFailure::model)?;
            let request = ContextAssembler::assemble(&snapshot, tools.clone())
                .map_err(ExecutionFailure::model)?;
            let mut stream = InvocationStream::new(
                self.threads.clone(),
                self.updates.clone(),
                snapshot.session_id.clone(),
                thread_id.clone(),
                turn_id.clone(),
                snapshot.sequence,
                cancellation.clone(),
            );
            let response = self
                .model
                .stream(&request, cancellation, &mut stream)
                .map_err(ExecutionFailure::service)?;
            check_cancellation(cancellation)?;

            let tool_calls = response.tool_calls().cloned().collect::<Vec<_>>();
            self.record_reasoning(thread_id, turn_id, &response, &mut stream)?;
            let text = final_text(&response, &stream);
            if tool_calls.is_empty() {
                if text.trim().is_empty() {
                    return Err(ExecutionFailure::model(CoreError::Execution(
                        response_failure_message(&response),
                    )));
                }
                let completion = match stream.text_item_id() {
                    Some(item_id) => self
                        .threads
                        .complete_turn_with_agent_message(thread_id, turn_id, item_id, text),
                    None => self.threads.complete_turn(thread_id, turn_id, text),
                };
                return completion.map_err(ExecutionFailure::persistence);
            }

            if !text.trim().is_empty() {
                match stream.text_item_id() {
                    Some(item_id) => self
                        .threads
                        .record_agent_message_with_id(thread_id, turn_id, item_id, text),
                    None => self.threads.record_agent_message(thread_id, turn_id, text),
                }
                .map_err(ExecutionFailure::persistence)?;
            }
            self.execute_tools(thread_id, turn_id, &tool_calls, cancellation)?;
        }

        Err(ExecutionFailure::model(CoreError::Execution(format!(
            "Turn exceeded its model invocation limit ({})",
            self.limits.max_model_invocations
        ))))
    }

    fn require_running_turn(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
    ) -> Result<(), ExecutionFailure> {
        let snapshot = self
            .threads
            .read_thread(thread_id)
            .map_err(ExecutionFailure::model)?;
        let status = snapshot
            .turns
            .iter()
            .find(|turn| &turn.turn_id == turn_id)
            .map(|turn| turn.status)
            .ok_or_else(|| ExecutionFailure::model(CoreError::NotFound(turn_id.to_string())))?;
        if status == TurnStatus::Running {
            Ok(())
        } else {
            Err(ExecutionFailure::model(CoreError::Execution(format!(
                "cannot execute a {status:?} Turn"
            ))))
        }
    }

    fn record_reasoning(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        response: &ModelResponse,
        stream: &mut InvocationStream,
    ) -> Result<(), ExecutionFailure> {
        if let Some((item_id, text)) = stream.take_reasoning() {
            if !text.trim().is_empty() {
                self.threads
                    .record_reasoning_with_id(thread_id, turn_id, item_id, text)
                    .map_err(ExecutionFailure::persistence)?;
            }
            return Ok(());
        }
        for item in &response.output {
            if let ResponseItem::Reasoning(text) = item
                && !text.trim().is_empty()
            {
                self.threads
                    .record_reasoning(thread_id, turn_id, text.clone())
                    .map_err(ExecutionFailure::persistence)?;
            }
        }
        Ok(())
    }

    fn publish_committed_after(&self, thread_id: &ThreadId, sequence: u64) {
        if let Ok(updates) = self.threads.thread_updates_after(thread_id, sequence) {
            for update in updates {
                self.updates.publish(update);
            }
        }
    }

    fn execute_tools(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        calls: &[ToolCall],
        cancellation: &CancellationToken,
    ) -> Result<(), ExecutionFailure> {
        for call in calls {
            self.threads
                .record_model_tool_call(thread_id, turn_id, call)
                .map_err(ExecutionFailure::persistence)?;
        }
        for call in calls {
            check_cancellation(cancellation)?;
            let output = self
                .tools
                .execute(call, cancellation)
                .map_err(ExecutionFailure::service)?;
            let output = match output {
                ToolExecutionOutput::Success(text) => ToolCallOutput::Success(text),
                ToolExecutionOutput::Failure(text) => ToolCallOutput::Failure(text),
            };
            self.threads
                .record_tool_result(
                    thread_id,
                    turn_id,
                    crate::RecordToolResultRequest {
                        tool_call_id: call.id.clone(),
                        output,
                    },
                )
                .map_err(ExecutionFailure::persistence)?;
        }
        Ok(())
    }
}

struct InvocationStream {
    threads: Arc<ThreadController>,
    updates: Arc<dyn ThreadUpdateSink>,
    session_id: zeta_protocol::SessionId,
    thread_id: ThreadId,
    turn_id: TurnId,
    durable_sequence: u64,
    stream_instance_id: StreamInstanceId,
    next_stream_sequence: u64,
    cancellation: CancellationToken,
    text_item_id: Option<ItemId>,
    reasoning_item_id: Option<ItemId>,
    text: String,
    reasoning: String,
}

impl InvocationStream {
    #[allow(clippy::too_many_arguments)]
    fn new(
        threads: Arc<ThreadController>,
        updates: Arc<dyn ThreadUpdateSink>,
        session_id: zeta_protocol::SessionId,
        thread_id: ThreadId,
        turn_id: TurnId,
        durable_sequence: u64,
        cancellation: CancellationToken,
    ) -> Self {
        let stream_instance_id = threads.next_stream_instance_id();
        Self {
            threads,
            updates,
            session_id,
            thread_id,
            turn_id,
            durable_sequence,
            stream_instance_id,
            next_stream_sequence: 0,
            cancellation,
            text_item_id: None,
            reasoning_item_id: None,
            text: String::new(),
            reasoning: String::new(),
        }
    }

    fn text_item_id(&self) -> Option<ItemId> {
        self.text_item_id.clone()
    }

    fn take_reasoning(&mut self) -> Option<(ItemId, String)> {
        self.reasoning_item_id
            .take()
            .map(|item_id| (item_id, std::mem::take(&mut self.reasoning)))
    }

    fn publish(&mut self, update: ThreadUpdate) {
        self.next_stream_sequence = self.next_stream_sequence.saturating_add(1);
        self.updates.publish(ThreadUpdateEnvelope {
            session_id: self.session_id.clone(),
            thread_id: self.thread_id.clone(),
            durable_sequence: self.durable_sequence,
            stream_cursor: Some(StreamCursor {
                stream_instance_id: self.stream_instance_id.clone(),
                sequence: self.next_stream_sequence,
            }),
            update,
        });
    }

    fn start_text_item(&mut self) -> ItemId {
        if let Some(item_id) = &self.text_item_id {
            return item_id.clone();
        }
        let item_id = self.threads.next_stream_item_id();
        self.publish(ThreadUpdate::ItemStarted {
            turn_id: self.turn_id.clone(),
            item: ThreadItem::AgentMessage {
                item_id: item_id.clone(),
                turn_id: self.turn_id.clone(),
                text: String::new(),
            },
        });
        self.text_item_id = Some(item_id.clone());
        item_id
    }

    fn start_reasoning_item(&mut self) -> ItemId {
        if let Some(item_id) = &self.reasoning_item_id {
            return item_id.clone();
        }
        let item_id = self.threads.next_stream_item_id();
        self.publish(ThreadUpdate::ItemStarted {
            turn_id: self.turn_id.clone(),
            item: ThreadItem::Reasoning {
                item_id: item_id.clone(),
                turn_id: self.turn_id.clone(),
                text: String::new(),
            },
        });
        self.reasoning_item_id = Some(item_id.clone());
        item_id
    }
}

impl ModelStreamSink for InvocationStream {
    fn emit(&mut self, event: ModelStreamEvent) -> Result<(), CoreError> {
        self.cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        match event {
            ModelStreamEvent::TextDelta(text) if !text.is_empty() => {
                let item_id = self.start_text_item();
                self.text.push_str(&text);
                self.publish(ThreadUpdate::ItemDelta {
                    turn_id: self.turn_id.clone(),
                    item_id,
                    delta: zeta_protocol::ItemDelta::AgentMessage { text },
                });
            }
            ModelStreamEvent::ReasoningDelta(text) if !text.is_empty() => {
                let item_id = self.start_reasoning_item();
                self.reasoning.push_str(&text);
                self.publish(ThreadUpdate::ItemDelta {
                    turn_id: self.turn_id.clone(),
                    item_id,
                    delta: zeta_protocol::ItemDelta::Reasoning { text },
                });
            }
            ModelStreamEvent::TextDelta(_) | ModelStreamEvent::ReasoningDelta(_) => {}
        }
        Ok(())
    }
}

fn final_text(response: &ModelResponse, stream: &InvocationStream) -> String {
    let text = response.text();
    if text.is_empty() {
        stream.text.clone()
    } else {
        text
    }
}

enum ExecutionFailure {
    Cancelled(CoreError),
    Failed {
        error: CoreError,
        stable: StableTurnError,
    },
}

impl ExecutionFailure {
    fn model(error: CoreError) -> Self {
        Self::Failed {
            error,
            stable: StableTurnError::model_invocation_failed(),
        }
    }

    fn service(error: CoreError) -> Self {
        match error {
            CoreError::Cancelled(_) => Self::Cancelled(error),
            _ => Self::model(error),
        }
    }

    fn persistence(error: CoreError) -> Self {
        Self::Failed {
            error,
            stable: StableTurnError::completion_persistence_failed(),
        }
    }
}

fn check_cancellation(cancellation: &CancellationToken) -> Result<(), ExecutionFailure> {
    cancellation
        .check()
        .map_err(|signal| ExecutionFailure::Cancelled(cancelled_error(&signal)))
}

fn cancelled_error(signal: &Cancellation<CancellationReason>) -> CoreError {
    CoreError::Cancelled(signal.reason().to_string())
}

fn response_failure_message(response: &zeta_protocol::ModelResponse) -> String {
    response
        .output
        .iter()
        .find_map(|item| match item {
            ResponseItem::Refusal(message) => Some(format!("model refused the request: {message}")),
            _ => None,
        })
        .unwrap_or_else(|| "model returned no final text or Tool Call".into())
}

#[cfg(test)]
#[path = "executor_tests.rs"]
mod tests;
