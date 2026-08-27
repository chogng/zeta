use super::ThreadPresentationEvent;
use super::transcript::TranscriptMessages;
use crate::components::transcript::CommandStatus;
use crate::components::transcript::Message;
use crate::components::transcript::MessageRole;

/// Owns the transcript rows currently displayed by the TUI.
#[derive(Debug, Default)]
pub(crate) struct ThreadFeatureState {
    transcript: TranscriptMessages,
}

impl ThreadFeatureState {
    pub(crate) fn messages(&self) -> &[Message] {
        &self.transcript.messages
    }

    pub(crate) fn update(&mut self, event: ThreadPresentationEvent) {
        match event {
            ThreadPresentationEvent::TranscriptSnapshotReceived(snapshot) => {
                self.transcript.replace(snapshot);
            }
            ThreadPresentationEvent::TranscriptHistoryPageReceived(page) => {
                self.transcript.prepend_history(page);
            }
            ThreadPresentationEvent::TranscriptUpdateReceived(update) => {
                self.transcript.apply(*update);
            }
            ThreadPresentationEvent::UserSubmitted(text) => {
                self.push_message(MessageRole::User, text);
            }
            ThreadPresentationEvent::CommandStarted(command) => {
                self.transcript.messages.push(Message::command(
                    command,
                    CommandStatus::Running,
                    None,
                ));
            }
            ThreadPresentationEvent::CommandCompleted { command, result } => {
                if let Some(message) = self.transcript.messages.iter_mut().rev().find(|message| {
                    message.role == MessageRole::Command
                        && message.text == command
                        && message.command_status == Some(CommandStatus::Running)
                }) {
                    message.command_status = Some(CommandStatus::Succeeded);
                    message.detail = Some(result);
                } else {
                    self.transcript.messages.push(Message::command(
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
                self.transcript.clear();
            }
        }
    }

    fn push_message(&mut self, role: MessageRole, text: String) {
        self.transcript.messages.push(Message::plain(role, text));
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
