use super::CellView;
use super::TranscriptCell;
use std::collections::BTreeSet;
use std::io;

/// Tracks successful terminal writes without keeping another copy of the conversation.
#[derive(Debug, Default)]
pub(crate) struct TranscriptHistory {
    scope: String,
    written: BTreeSet<String>,
}

impl TranscriptHistory {
    pub(crate) fn write(
        &mut self,
        scope: &str,
        cells: &[TranscriptCell],
        output: &mut impl FnMut(&CellView<'_>) -> io::Result<()>,
    ) -> io::Result<()> {
        if self.scope != scope {
            self.scope = scope.to_owned();
            self.written.clear();
        }
        for cell in cells {
            let id = cell.cell_id().as_str();
            if self.written.contains(id) {
                continue;
            }
            let cell = cell.history_view();
            output(&cell)?;
            self.written.insert(id.to_owned());
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
