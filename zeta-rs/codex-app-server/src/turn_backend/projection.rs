use super::publish_committed_after;
use crate::CodexTurnStatus;
use std::sync::Arc;
use zeta_core::CoreError;
use zeta_core::ThreadController;
use zeta_core::ThreadUpdateSink;
use zeta_protocol::ItemDelta;
use zeta_protocol::ItemId;
use zeta_protocol::SessionId;
use zeta_protocol::StreamCursor;
use zeta_protocol::StreamInstanceId;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadItem;
use zeta_protocol::ThreadUpdate;
use zeta_protocol::ThreadUpdateEnvelope;
use zeta_protocol::TurnId;

pub(super) struct TurnProjection {
    threads: Arc<ThreadController>,
    updates: Arc<dyn ThreadUpdateSink>,
    session_id: SessionId,
    pub(super) thread_id: ThreadId,
    pub(super) turn_id: TurnId,
    pub(super) durable_sequence: u64,
    stream_instance_id: StreamInstanceId,
    stream_sequence: u64,
    agent_item_id: Option<ItemId>,
    agent_text: String,
    reasoning_item_id: Option<ItemId>,
    reasoning_text: String,
}

impl TurnProjection {
    pub(super) fn new(
        threads: Arc<ThreadController>,
        updates: Arc<dyn ThreadUpdateSink>,
        session_id: SessionId,
        thread_id: ThreadId,
        turn_id: TurnId,
        durable_sequence: u64,
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
            stream_sequence: 0,
            agent_item_id: None,
            agent_text: String::new(),
            reasoning_item_id: None,
            reasoning_text: String::new(),
        }
    }

    pub(super) fn agent_delta(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        let item_id = self.agent_item_id();
        self.agent_text.push_str(&text);
        self.publish(ThreadUpdate::ItemDelta {
            turn_id: self.turn_id.clone(),
            item_id,
            delta: ItemDelta::AgentMessage { text },
        });
    }

    pub(super) fn reasoning_delta(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        let item_id = self.reasoning_item_id();
        self.reasoning_text.push_str(&text);
        self.publish(ThreadUpdate::ItemDelta {
            turn_id: self.turn_id.clone(),
            item_id,
            delta: ItemDelta::Reasoning { text },
        });
    }

    pub(super) fn finish(&mut self, status: CodexTurnStatus) -> Result<(), CoreError> {
        match status {
            CodexTurnStatus::Completed => {
                let before = self.durable_sequence;
                if !self.reasoning_text.trim().is_empty() {
                    let item_id = self
                        .reasoning_item_id
                        .clone()
                        .unwrap_or_else(|| self.threads.next_stream_item_id());
                    self.durable_sequence = self.threads.record_reasoning_with_id(
                        &self.thread_id,
                        &self.turn_id,
                        item_id,
                        self.reasoning_text.clone(),
                    )?;
                }
                if self.agent_text.trim().is_empty() {
                    self.durable_sequence = self
                        .threads
                        .complete_turn_without_agent_message(&self.thread_id, &self.turn_id)?;
                } else {
                    let item_id = self
                        .agent_item_id
                        .clone()
                        .unwrap_or_else(|| self.threads.next_stream_item_id());
                    self.durable_sequence = self
                        .threads
                        .complete_turn_with_agent_message(
                            &self.thread_id,
                            &self.turn_id,
                            item_id,
                            self.agent_text.clone(),
                        )?
                        .sequence;
                }
                self.publish_committed_after(before);
                Ok(())
            }
            CodexTurnStatus::Interrupted => Err(CoreError::Execution(
                "Codex App Server interrupted the Turn unexpectedly".into(),
            )),
            CodexTurnStatus::Failed => Err(CoreError::Execution(
                "Codex App Server failed the Turn".into(),
            )),
        }
    }

    pub(super) fn policy_revision(&self) -> Result<String, CoreError> {
        self.threads
            .read_thread(&self.thread_id)?
            .turns
            .iter()
            .find(|turn| turn.turn_id == self.turn_id)
            .map(|turn| turn.policy_revision.clone())
            .ok_or_else(|| CoreError::NotFound(self.turn_id.to_string()))
    }

    pub(super) fn publish_committed_after(&self, sequence: u64) {
        publish_committed_after(
            self.threads.as_ref(),
            self.updates.as_ref(),
            &self.thread_id,
            sequence,
        );
    }

    fn agent_item_id(&mut self) -> ItemId {
        if let Some(item_id) = &self.agent_item_id {
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
        self.agent_item_id = Some(item_id.clone());
        item_id
    }

    fn reasoning_item_id(&mut self) -> ItemId {
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

    fn publish(&mut self, update: ThreadUpdate) {
        self.stream_sequence = self.stream_sequence.saturating_add(1);
        self.updates.publish(ThreadUpdateEnvelope {
            session_id: self.session_id.clone(),
            thread_id: self.thread_id.clone(),
            durable_sequence: self.durable_sequence,
            stream_cursor: Some(StreamCursor {
                stream_instance_id: self.stream_instance_id.clone(),
                sequence: self.stream_sequence,
            }),
            update,
        });
    }
}
