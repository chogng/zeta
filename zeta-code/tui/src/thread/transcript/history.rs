use super::CellView;
use super::TranscriptCell;
use std::collections::BTreeSet;
use std::io;

/// Tracks successful terminal writes without keeping another copy of the conversation.
#[derive(Debug, Default)]
pub(crate) struct TranscriptHistory {
    scope: String,
    written: BTreeSet<String>,
    header_written: bool,
}

impl TranscriptHistory {
    pub(crate) fn header_written(&self, scope: &str) -> bool {
        self.scope == scope && self.header_written
    }

    pub(crate) fn mark_header_written(&mut self, scope: &str) {
        self.set_scope(scope);
        self.header_written = true;
    }

    fn set_scope(&mut self, scope: &str) {
        if self.scope != scope {
            self.scope = scope.to_owned();
            self.written.clear();
            self.header_written = false;
        }
    }

    pub(crate) fn write(
        &mut self,
        scope: &str,
        cells: &[TranscriptCell],
        output: &mut impl FnMut(&CellView<'_>) -> io::Result<()>,
    ) -> io::Result<()> {
        self.set_scope(scope);
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
