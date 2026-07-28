use super::{CompletedTurn, RecordToolExecutionStart, RecordedToolCall, ThreadController};
use crate::CoreError;
use zeta_async_utils::CancellationToken;
use zeta_protocol::{
    ItemId, RequestId, StreamInstanceId, ThreadEvent, ThreadId, ThreadItem, ToolCall, TurnId,
};

impl ThreadController {
    pub(crate) fn record_model_tool_call(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        call: &ToolCall,
    ) -> Result<RecordedToolCall, CoreError> {
        let item = ThreadItem::ToolCall {
            item_id: ItemId::new(self.next_identifier("item"))
                .expect("generated Item ID is non-empty"),
            turn_id: turn_id.clone(),
            tool_call_id: call.id.clone(),
            name: call.name.clone(),
            arguments_json: serde_json::to_string(&call.arguments)
                .map_err(|error| CoreError::Context(error.to_string()))?,
        };
        let sequence = self.record_item(thread_id, turn_id, item.clone())?;
        Ok(RecordedToolCall {
            item,
            tool_call_id: call.id.clone(),
            sequence,
        })
    }

    pub(crate) fn record_agent_message(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        text: String,
    ) -> Result<u64, CoreError> {
        self.record_agent_message_with_id(
            thread_id,
            turn_id,
            ItemId::new(self.next_identifier("item")).expect("generated Item ID is non-empty"),
            text,
        )
    }

    pub(crate) fn record_agent_message_with_id(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        item_id: ItemId,
        text: String,
    ) -> Result<u64, CoreError> {
        self.record_item(
            thread_id,
            turn_id,
            ThreadItem::AgentMessage {
                item_id,
                turn_id: turn_id.clone(),
                text,
            },
        )
    }

    pub(crate) fn record_reasoning(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        text: String,
    ) -> Result<u64, CoreError> {
        self.record_reasoning_with_id(
            thread_id,
            turn_id,
            ItemId::new(self.next_identifier("item")).expect("generated Item ID is non-empty"),
            text,
        )
    }

    pub(crate) fn record_reasoning_with_id(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        item_id: ItemId,
        text: String,
    ) -> Result<u64, CoreError> {
        self.record_item(
            thread_id,
            turn_id,
            ThreadItem::Reasoning {
                item_id,
                turn_id: turn_id.clone(),
                text,
            },
        )
    }

    pub(crate) fn complete_turn_with_agent_message(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        item_id: ItemId,
        output: String,
    ) -> Result<CompletedTurn, CoreError> {
        let _lease = self.acquire_writer_lease(thread_id)?;
        let mut threads = self
            .threads
            .lock()
            .map_err(|_| CoreError::Journal("thread state lock poisoned".into()))?;
        let snapshot = threads
            .get_mut(thread_id)
            .ok_or_else(|| CoreError::NotFound(thread_id.to_string()))?;
        let item = ThreadItem::AgentMessage {
            item_id,
            turn_id: turn_id.clone(),
            text: output,
        };
        self.record_batch(
            snapshot,
            vec![
                ThreadEvent::ItemCompleted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    item: item.clone(),
                },
                ThreadEvent::TurnCompleted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                },
            ],
        )?;
        Ok(CompletedTurn {
            item,
            sequence: snapshot.sequence,
        })
    }

    pub(crate) fn next_stream_item_id(&self) -> ItemId {
        ItemId::new(self.next_identifier("item")).expect("generated Item ID is non-empty")
    }

    pub(crate) fn next_stream_instance_id(&self) -> StreamInstanceId {
        StreamInstanceId::new(self.next_identifier("stream"))
            .expect("generated stream instance ID is non-empty")
    }

    pub(crate) fn next_interaction_request_id(&self) -> RequestId {
        RequestId::new(self.next_identifier("request"))
            .expect("generated interaction request ID is non-empty")
    }

    pub(crate) fn record_tool_execution_started(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        start: RecordToolExecutionStart,
    ) -> Result<(), CoreError> {
        self.transition_turn(
            thread_id,
            vec![ThreadEvent::ToolExecutionStarted {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                tool_call_id: start.tool_call_id,
                action_digest: start.action_digest,
                policy_revision: start.policy_revision,
                authority: start.authority,
            }],
        )
    }

    pub(crate) fn enqueue_turn_execution(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        task: impl FnOnce(CancellationToken) + Send + 'static,
    ) -> Result<(), CoreError> {
        self.execution_mailboxes.enqueue(thread_id, turn_id, task)
    }

    pub(crate) fn interrupt_execution(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
    ) -> Result<(), CoreError> {
        let _lease = self.acquire_writer_lease(thread_id)?;
        let mut threads = self
            .threads
            .lock()
            .map_err(|_| CoreError::Journal("thread state lock poisoned".into()))?;
        let snapshot = threads
            .get_mut(thread_id)
            .ok_or_else(|| CoreError::NotFound(thread_id.to_string()))?;
        let status = snapshot
            .turns
            .iter()
            .find(|turn| &turn.turn_id == turn_id)
            .map(|turn| turn.status)
            .ok_or_else(|| CoreError::NotFound(turn_id.to_string()))?;
        let events = match status {
            crate::TurnStatus::Created
            | crate::TurnStatus::Running
            | crate::TurnStatus::WaitingForApproval
            | crate::TurnStatus::WaitingForUserInput
            | crate::TurnStatus::WaitingForCapability => vec![
                ThreadEvent::TurnCancelling {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                },
                ThreadEvent::TurnInterrupted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                },
            ],
            crate::TurnStatus::Cancelling => vec![ThreadEvent::TurnInterrupted {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
            }],
            crate::TurnStatus::Completed
            | crate::TurnStatus::Failed
            | crate::TurnStatus::Interrupted => return Ok(()),
        };
        self.record_batch(snapshot, events)
    }

    pub(crate) fn cancel_turn_execution(&self, thread_id: &ThreadId, turn_id: &TurnId) {
        self.execution_mailboxes.cancel(thread_id, turn_id);
    }
}
