use super::ThreadPresentationEvent;
use crate::components::transcript::CommandStatus;
use crate::components::transcript::Message;
use crate::components::transcript::MessageRole;
use zeta_protocol::Thread;
use zeta_protocol::ThreadItem;

/// Owns the canonical active Thread snapshot and its current transcript projection.
///
/// Local optimistic and diagnostic messages live here as presentation overlays until the next
/// canonical snapshot replaces the projection.
#[derive(Debug, Default)]
pub(crate) struct ThreadFeatureState {
    snapshot: Option<Thread>,
    messages: Vec<Message>,
}

impl ThreadFeatureState {
    pub(crate) fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub(crate) fn update(&mut self, event: ThreadPresentationEvent) {
        match event {
            ThreadPresentationEvent::SnapshotReceived(snapshot) => {
                self.messages = project_messages(&snapshot);
                self.snapshot = Some(snapshot);
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
            }
        }
    }

    fn push_message(&mut self, role: MessageRole, text: String) {
        self.messages.push(Message::plain(role, text));
    }

    #[cfg(test)]
    pub(super) fn snapshot(&self) -> Option<&Thread> {
        self.snapshot.as_ref()
    }
}

fn project_messages(thread: &Thread) -> Vec<Message> {
    thread
        .turns
        .iter()
        .flat_map(|turn| &turn.items)
        .filter_map(|item| match item {
            ThreadItem::UserMessage { text, .. } => {
                Some(Message::plain(MessageRole::User, text.clone()))
            }
            ThreadItem::UserImage { .. } => {
                Some(Message::plain(MessageRole::User, "[Image]".into()))
            }
            ThreadItem::AgentMessage { text, .. } => {
                Some(Message::plain(MessageRole::Agent, text.clone()))
            }
            ThreadItem::Reasoning { .. }
            | ThreadItem::Plan { .. }
            | ThreadItem::ToolCall { .. }
            | ThreadItem::ToolResult { .. } => None,
        })
        .collect()
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
