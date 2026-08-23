use super::{
    CommitModelInvocationItemsResult, CompleteModelInvocationResult, CompletedTurn,
    RecordToolExecutionEscalation, RecordToolExecutionStart, ThreadController,
};
use crate::CoreError;
use crate::ThreadCommandResult;
use zeta_protocol::{
    ItemId, RequestId, StreamInstanceId, ThreadEvent, ThreadId, ThreadItem, TurnId,
};

#[cfg(test)]
use super::RecordedToolCall;
#[cfg(test)]
use zeta_protocol::{ToolCall, ToolCallBinding};

impl ThreadController {
    /// Durably records that a delegated backend is about to cross its external side-effect
    /// boundary.
    ///
    /// A backend must append this fact before its first remote request and must never replay a
    /// Turn that already carries an attempt after process recovery.
    pub fn record_turn_execution_attempt(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        backend: String,
    ) -> Result<u64, CoreError> {
        self.mutate_thread(thread_id, |snapshot| {
            self.record_batch(
                snapshot,
                vec![ThreadEvent::TurnExecutionAttempted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    backend,
                }],
            )?;
            Ok(snapshot.sequence)
        })
    }

    /// Completes a delegated Turn that intentionally produced no agent message.
    pub fn complete_turn_without_agent_message(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
    ) -> Result<u64, CoreError> {
        self.mutate_thread(thread_id, |snapshot| {
            self.record_batch(
                snapshot,
                vec![ThreadEvent::TurnCompleted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                }],
            )?;
            Ok(snapshot.sequence)
        })
    }

    #[cfg(test)]
    pub(crate) fn record_model_tool_call(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        call: &ToolCall,
        binding: ToolCallBinding,
    ) -> Result<RecordedToolCall, CoreError> {
        let item = ThreadItem::ToolCall {
            item_id: ItemId::new(self.next_identifier("item"))
                .expect("generated Item ID is non-empty"),
            turn_id: turn_id.clone(),
            tool_call_id: call.id.clone(),
            name: call.name.clone(),
            arguments_json: serde_json::to_string(&call.arguments)
                .map_err(|error| CoreError::Context(error.to_string()))?,
            binding: Some(binding),
        };
        let sequence = self.record_item(thread_id, turn_id, item.clone())?;
        Ok(RecordedToolCall {
            item,
            tool_call_id: call.id.clone(),
            sequence,
        })
    }

    /// Commits one complete agent message produced by a Turn execution backend.
    pub fn record_agent_message(
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

    /// Commits one complete agent message using the Item ID projected during streaming.
    pub fn record_agent_message_with_id(
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

    /// Commits one complete reasoning item produced by a Turn execution backend.
    pub fn record_reasoning(
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

    /// Commits one complete reasoning item using the Item ID projected during streaming.
    pub fn record_reasoning_with_id(
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

    /// Atomically commits the final agent message and terminal Turn completion.
    pub fn complete_turn_with_agent_message(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        item_id: ItemId,
        output: String,
    ) -> Result<CompletedTurn, CoreError> {
        self.mutate_thread(thread_id, |snapshot| {
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
        })
    }

    /// Commits one local model completion unless durable steering arrived after its input snapshot.
    pub(crate) fn complete_model_invocation_with_agent_message(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        source_thread_sequence: u64,
        preceding_items: Vec<ThreadItem>,
        item_id: ItemId,
        output: String,
    ) -> Result<CompleteModelInvocationResult, CoreError> {
        self.mutate_thread(thread_id, |snapshot| {
            if has_steer_after(snapshot, turn_id, source_thread_sequence) {
                return Ok(CompleteModelInvocationResult::SupersededBySteer);
            }
            let item = ThreadItem::AgentMessage {
                item_id,
                turn_id: turn_id.clone(),
                text: output,
            };
            let mut events = preceding_items
                .into_iter()
                .map(|item| ThreadEvent::ItemCompleted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    item,
                })
                .collect::<Vec<_>>();
            events.extend([
                ThreadEvent::ItemCompleted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    item: item.clone(),
                },
                ThreadEvent::TurnCompleted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                },
            ]);
            self.record_batch(snapshot, events)?;
            Ok(CompleteModelInvocationResult::Completed(CompletedTurn {
                item,
                sequence: snapshot.sequence,
            }))
        })
    }

    /// Atomically commits non-terminal model output unless newer steering superseded its input.
    pub(crate) fn commit_model_invocation_items(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        source_thread_sequence: u64,
        items: Vec<ThreadItem>,
    ) -> Result<CommitModelInvocationItemsResult, CoreError> {
        self.mutate_thread(thread_id, |snapshot| {
            if has_steer_after(snapshot, turn_id, source_thread_sequence) {
                return Ok(CommitModelInvocationItemsResult::SupersededBySteer);
            }
            let events = items
                .into_iter()
                .map(|item| ThreadEvent::ItemCompleted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    item,
                })
                .collect();
            self.record_batch(snapshot, events)?;
            Ok(CommitModelInvocationItemsResult::Committed)
        })
    }

    /// Returns whether a model response still precedes every accepted steering command.
    pub(crate) fn model_invocation_is_current(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        source_thread_sequence: u64,
    ) -> Result<bool, CoreError> {
        self.with_loaded_thread(thread_id, |loaded| {
            Ok(!has_steer_after(
                &loaded.snapshot,
                turn_id,
                source_thread_sequence,
            ))
        })
    }

    /// Allocates a process-unique Item ID for a backend's transient stream projection.
    pub fn next_stream_item_id(&self) -> ItemId {
        ItemId::new(self.next_identifier("item")).expect("generated Item ID is non-empty")
    }

    /// Allocates a process-unique stream identity for ordered transient updates.
    pub fn next_stream_instance_id(&self) -> StreamInstanceId {
        StreamInstanceId::new(self.next_identifier("stream"))
            .expect("generated stream instance ID is non-empty")
    }

    /// Allocates a process-unique ID for a backend-originated durable interaction.
    pub fn next_interaction_request_id(&self) -> RequestId {
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

    pub(crate) fn record_tool_execution_escalated(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        escalation: RecordToolExecutionEscalation,
    ) -> Result<(), CoreError> {
        self.transition_turn(
            thread_id,
            vec![ThreadEvent::ToolExecutionEscalated {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                tool_call_id: escalation.tool_call_id,
                action_digest: escalation.action_digest,
                policy_revision: escalation.policy_revision,
                denial: escalation.denial,
                authority: escalation.authority,
            }],
        )
    }

    /// Enqueues backend work on the Thread-owned bounded execution mailbox.
    ///
    /// The task must keep all external Turn work inside the supplied cancellation scope. Returning
    /// releases the active operation; a later interaction continuation may enqueue another task.
    pub fn enqueue_turn_execution(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        task: impl FnOnce(super::mailbox::ThreadExecutionContext) + Send + 'static,
    ) -> Result<(), CoreError> {
        self.execution_mailboxes.enqueue(thread_id, turn_id, task)
    }

    pub(crate) fn interrupt_execution(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
    ) -> Result<(), CoreError> {
        self.mutate_thread(thread_id, |snapshot| {
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
        })
    }

    pub(crate) fn cancel_turn_execution(&self, thread_id: &ThreadId, turn_id: &TurnId) {
        self.execution_mailboxes.cancel(thread_id, turn_id);
    }
}

fn has_steer_after(
    snapshot: &crate::ThreadSnapshot,
    turn_id: &TurnId,
    source_thread_sequence: u64,
) -> bool {
    snapshot.commands.iter().any(|command| {
        command.response_sequence > source_thread_sequence
            && matches!(
                &command.result,
                ThreadCommandResult::TurnSteered {
                    turn_id: command_turn_id,
                } if command_turn_id == turn_id
            )
    })
}
