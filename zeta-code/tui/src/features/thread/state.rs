use super::ThreadPresentationEvent;
use super::transcript::TranscriptCell;
use super::transcript::TranscriptCellId;
use super::transcript::TranscriptProjection;
use crate::components::chat_history::Message;
use crate::components::chat_history::MessageRole;
use std::collections::BTreeSet;

/// Owns the ordered transcript-cell projection for the currently subscribed Thread.
#[derive(Debug, Default)]
pub(crate) struct ThreadFeatureState {
    transcript: TranscriptProjection,
    messages: Vec<Message>,
}

impl ThreadFeatureState {
    pub(crate) fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub(crate) fn views(
        &self,
        expanded: &BTreeSet<TranscriptCellId>,
        selected: Option<&TranscriptCellId>,
    ) -> Vec<Message> {
        self.transcript.views(expanded, selected)
    }

    pub(crate) fn cells(&self) -> &[TranscriptCell] {
        self.transcript.cells()
    }

    pub(crate) fn details(&self, cell_id: &TranscriptCellId) -> Option<String> {
        self.transcript.details(cell_id)
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
                self.transcript.push_message(MessageRole::User, text);
            }
            ThreadPresentationEvent::CommandStarted(command) => {
                self.transcript.command_started(command);
            }
            ThreadPresentationEvent::CommandCompleted { command, result } => {
                self.transcript.command_completed(command, result);
            }
            ThreadPresentationEvent::NoticeReceived(text) => {
                self.transcript.push_notice(text);
            }
            ThreadPresentationEvent::FailureReported(text) => {
                self.transcript.push_error(text);
            }
            ThreadPresentationEvent::Interrupted => {
                self.transcript.push_notice("turn interrupted".into());
            }
            ThreadPresentationEvent::Cleared => {
                self.transcript.clear();
            }
        }
        self.messages = self.transcript.views(&BTreeSet::new(), None);
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
