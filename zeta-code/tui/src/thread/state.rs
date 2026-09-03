use super::ThreadPresentationEvent;
use super::presentation::ActiveTurnUpdate;
use super::presentation::evaluate_active_turn;
use super::presentation::recover_active_turn;
use super::transcript::TranscriptCell;
use super::transcript::TranscriptCellId;
use super::transcript::TranscriptProjection;
use crate::thread::transcript::Message;
use crate::thread::transcript::MessageRole;
use std::collections::BTreeSet;
use zeta_protocol::ApprovalMode;
use zeta_protocol::Turn;
use zeta_protocol::TurnId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TurnApprovalModes {
    pub(crate) current: Option<ApprovalMode>,
    pub(crate) next: ApprovalMode,
}

impl Default for TurnApprovalModes {
    fn default() -> Self {
        Self {
            current: None,
            next: ApprovalMode::AskPermissions,
        }
    }
}

impl From<ApprovalMode> for TurnApprovalModes {
    fn from(next: ApprovalMode) -> Self {
        Self {
            current: None,
            next,
        }
    }
}

/// Owns current-Turn lifecycle inputs and ordered transcript state for the subscribed Thread.
#[derive(Debug, Default)]
pub(crate) struct ThreadState {
    active_turn: Option<TurnId>,
    approval_modes: TurnApprovalModes,
    transcript: TranscriptProjection,
    messages: Vec<Message>,
}

impl ThreadState {
    pub(crate) fn active_turn(&self) -> Option<&TurnId> {
        self.active_turn.as_ref()
    }

    pub(crate) fn set_active_turn(&mut self, turn_id: TurnId) {
        self.active_turn = Some(turn_id);
    }

    pub(crate) fn set_active_turn_if_idle(&mut self, turn_id: TurnId) {
        if self.active_turn.is_none() {
            self.active_turn = Some(turn_id);
        }
    }

    pub(crate) fn clear_active_turn(&mut self) {
        self.active_turn = None;
    }

    pub(crate) fn sync_active_turn(&mut self, turns: &[Turn]) -> Vec<ActiveTurnUpdate> {
        if self.active_turn.is_none() {
            self.active_turn = recover_active_turn(turns);
        }
        let mut updates = vec![evaluate_active_turn(&mut self.active_turn, turns)];
        if self.active_turn.is_none() {
            self.active_turn = recover_active_turn(turns);
            if self.active_turn.is_some() {
                updates.push(evaluate_active_turn(&mut self.active_turn, turns));
            }
        }
        updates
    }

    pub(crate) fn approval_modes(&self) -> TurnApprovalModes {
        self.approval_modes
    }

    pub(crate) fn approval_mode(&self) -> ApprovalMode {
        self.approval_modes.next
    }

    pub(crate) fn cycle_approval_mode(&mut self) {
        self.approval_modes.next = match self.approval_modes.next {
            ApprovalMode::AskPermissions => ApprovalMode::AutoReview,
            ApprovalMode::AutoReview => ApprovalMode::BypassPermissions,
            ApprovalMode::BypassPermissions => ApprovalMode::AskPermissions,
        };
    }

    #[cfg(test)]
    pub(crate) fn set_next_approval_mode(&mut self, approval_mode: ApprovalMode) {
        self.approval_modes.next = approval_mode;
    }

    pub(crate) fn set_current_approval_mode(&mut self, approval_mode: Option<ApprovalMode>) {
        self.approval_modes.current = approval_mode;
    }

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
            ThreadPresentationEvent::CommandSubmitted(command) => {
                self.transcript.command_submitted(command);
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
