//! Retained transcript projection for one Session Pane.

use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_thread_transcript::ThreadTranscriptChange;
use zeta_thread_transcript::ThreadTranscriptEntry;
use zeta_thread_transcript::ThreadTranscriptSnapshot;
use zeta_thread_transcript::ThreadTranscriptUpdateEnvelope;

/// Result of mechanically applying one backend-assembled transcript update.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TranscriptUpdateResult {
    Applied,
    Ignored,
}

/// Retained transcript entries for one Thread.
///
/// Entry assembly, ordering decisions, stream accumulation, and transient bounds belong to
/// `zeta-thread-transcript`. This state only applies the backend-provided list changes.
#[derive(Default)]
pub(crate) struct TranscriptState {
    session_id: Option<SessionId>,
    thread_id: Option<ThreadId>,
    durable_sequence: u64,
    entries: Vec<ThreadTranscriptEntry>,
}

impl TranscriptState {
    pub fn replace_snapshot(&mut self, snapshot: ThreadTranscriptSnapshot) {
        self.session_id = Some(snapshot.session_id);
        self.thread_id = Some(snapshot.thread_id);
        self.durable_sequence = snapshot.durable_sequence;
        self.entries = snapshot.entries;
    }

    pub fn entries(&self) -> &[ThreadTranscriptEntry] {
        &self.entries
    }

    pub fn apply_update(
        &mut self,
        update: ThreadTranscriptUpdateEnvelope,
    ) -> TranscriptUpdateResult {
        if self.session_id.as_ref() != Some(&update.session_id)
            || self.thread_id.as_ref() != Some(&update.thread_id)
            || update.durable_sequence < self.durable_sequence
        {
            return TranscriptUpdateResult::Ignored;
        }
        for change in update.changes {
            match change {
                ThreadTranscriptChange::Upsert { entry } => {
                    if let Some(index) = self
                        .entries
                        .iter()
                        .position(|current| current.entry_id() == entry.entry_id())
                    {
                        self.entries[index] = entry;
                    } else {
                        self.entries.push(entry);
                    }
                }
                ThreadTranscriptChange::Remove { entry_ids } => {
                    self.entries
                        .retain(|entry| !entry_ids.iter().any(|id| id == entry.entry_id()));
                }
                ThreadTranscriptChange::ClearTransient => {
                    self.entries.retain(|entry| !entry.is_transient());
                }
            }
        }
        self.durable_sequence = self.durable_sequence.max(update.durable_sequence);
        TranscriptUpdateResult::Applied
    }
}

#[cfg(test)]
#[path = "transcript_tests.rs"]
mod tests;
