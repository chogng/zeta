use super::Message;
use super::TranscriptCell;
use super::model::CellLifecycle;
use std::collections::BTreeMap;
use std::hash::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::io;

/// Tracks successful terminal writes without keeping another copy of the conversation.
#[derive(Debug, Default)]
pub(crate) struct TranscriptHistory {
    scope: String,
    written: BTreeMap<String, (u64, u64)>,
}

impl TranscriptHistory {
    pub(crate) fn write(
        &mut self,
        scope: &str,
        cells: &[TranscriptCell],
        output: &mut impl FnMut(&Message) -> io::Result<()>,
    ) -> io::Result<()> {
        if self.scope != scope {
            self.scope = scope.to_owned();
            self.written.clear();
        }
        for cell in cells {
            // A live cell may still change. Preserve ordering until it is final.
            if cell.lifecycle() == CellLifecycle::Live {
                break;
            }
            let id = cell.cell_id().as_str();
            let revision = cell.render_revision();
            if self
                .written
                .get(id)
                .is_some_and(|(written, _)| *written == revision)
            {
                continue;
            }
            let message = cell.history_view();
            let mut hash = DefaultHasher::new();
            std::mem::discriminant(&message.role).hash(&mut hash);
            message
                .command_status
                .map(|status| std::mem::discriminant(&status))
                .hash(&mut hash);
            message.text.hash(&mut hash);
            message.detail.hash(&mut hash);
            let fingerprint = hash.finish();
            if let Some((written_revision, written_fingerprint)) = self.written.get_mut(id)
                && *written_fingerprint == fingerprint
            {
                *written_revision = revision;
                continue;
            }
            output(&message)?;
            self.written.insert(id.to_owned(), (revision, fingerprint));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
