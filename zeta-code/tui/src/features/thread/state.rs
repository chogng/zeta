use super::ThreadPresentationEvent;
use super::projection::TransientProjection;
use super::projection::apply_transient;
use super::projection::project_messages;
use crate::components::transcript::CommandStatus;
use crate::components::transcript::Message;
use crate::components::transcript::MessageRole;
use zeta_protocol::Thread;

/// Owns the canonical active Thread snapshot and its current transcript projection.
///
/// Local optimistic and diagnostic messages live here as presentation overlays until the next
/// canonical snapshot replaces the projection.
#[derive(Debug, Default)]
pub(crate) struct ThreadFeatureState {
    snapshot: Option<Thread>,
    messages: Vec<Message>,
    transient: TransientProjection,
}

impl ThreadFeatureState {
    pub(crate) fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub(crate) fn update(&mut self, event: ThreadPresentationEvent) {
        match event {
            ThreadPresentationEvent::SnapshotReceived(snapshot) => {
                self.messages = project_messages(&snapshot);
                self.transient.clear();
                self.snapshot = Some(snapshot);
            }
            ThreadPresentationEvent::HistoryPageReceived(page) => {
                self.merge_history_page(page);
            }
            ThreadPresentationEvent::TransientStreamReset => {
                self.transient.remove_from(&mut self.messages);
            }
            ThreadPresentationEvent::TransientUpdateReceived(update) => {
                apply_transient(&mut self.messages, &mut self.transient, &update);
            }
            ThreadPresentationEvent::UserSubmitted(text) => {
                self.push_message(MessageRole::User, text);
            }
            ThreadPresentationEvent::CommandStarted(command) => {
                self.messages
                    .push(Message::command(command, CommandStatus::Running, None));
            }
            ThreadPresentationEvent::CommandCompleted { command, result } => {
                if let Some(message) = self.messages.iter_mut().rev().find(|message| {
                    message.role == MessageRole::Command
                        && message.text == command
                        && message.command_status == Some(CommandStatus::Running)
                }) {
                    message.command_status = Some(CommandStatus::Succeeded);
                    message.detail = Some(result);
                } else {
                    self.messages.push(Message::command(
                        command,
                        CommandStatus::Succeeded,
                        Some(result),
                    ));
                }
            }
            ThreadPresentationEvent::NoticeReceived(text) => {
                self.push_message(MessageRole::Notice, text);
            }
            ThreadPresentationEvent::FailureReported(text) => {
                self.push_message(MessageRole::Error, text);
            }
            ThreadPresentationEvent::Interrupted => {
                self.push_message(MessageRole::Notice, "turn interrupted".into());
            }
            ThreadPresentationEvent::Cleared => {
                self.snapshot = None;
                self.messages.clear();
                self.transient.clear();
            }
        }
    }

    fn push_message(&mut self, role: MessageRole, text: String) {
        self.messages.push(Message::plain(role, text));
    }

    fn merge_history_page(&mut self, page: Thread) {
        let Some(current) = self.snapshot.as_ref() else {
            self.messages = project_messages(&page);
            self.snapshot = Some(page);
            self.transient.clear();
            return;
        };
        if current.session_id != page.session_id || current.thread_id != page.thread_id {
            return;
        }
        let mut older_projection = page;
        let older_turns = std::mem::take(&mut older_projection.turns)
            .into_iter()
            .filter(|turn| {
                current
                    .turns
                    .iter()
                    .all(|existing| existing.turn_id != turn.turn_id)
            })
            .collect::<Vec<_>>();
        older_projection.turns = older_turns.clone();
        let mut messages = project_messages(&older_projection);
        messages.append(&mut self.messages);

        let mut merged = current.clone();
        let mut turns = older_turns;
        turns.extend(current.turns.iter().cloned());
        merged.turns = turns;
        self.messages = messages;
        self.snapshot = Some(merged);
    }

    #[cfg(test)]
    pub(super) fn snapshot(&self) -> Option<&Thread> {
        self.snapshot.as_ref()
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
