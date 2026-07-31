use zeta_editor::{CodeEditorCommand, CodeEditorSelectionMode};
use zeta_ui::{Point, Rect, TextInputCompositionEvent};

use crate::composer_editor::ComposerEditor;

/// Explicit submission mode for the shared Agent Console composer.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ComposerMode {
    #[default]
    Agent,
    Shell,
}

pub(crate) enum ComposerSubmission {
    AgentMessage(String),
    ShellCommand(String),
}

/// Host-owned editor shared by Agent messages and direct Shell commands.
#[derive(Default)]
pub(crate) struct AgentComposer {
    editor: ComposerEditor,
    mode: ComposerMode,
    shell_history: Vec<String>,
    shell_history_index: Option<usize>,
    shell_history_draft: Option<String>,
}

impl AgentComposer {
    pub(crate) const fn editor(&self) -> &ComposerEditor {
        &self.editor
    }

    pub(crate) const fn mode(&self) -> ComposerMode {
        self.mode
    }

    pub(crate) fn set_mode(&mut self, mode: ComposerMode) {
        self.mode = mode;
        self.editor.cancel_composition();
        self.leave_shell_history();
    }

    pub(crate) fn toggle_mode(&mut self) {
        self.set_mode(match self.mode {
            ComposerMode::Agent => ComposerMode::Shell,
            ComposerMode::Shell => ComposerMode::Agent,
        });
    }

    pub(crate) fn submission(&self) -> Option<ComposerSubmission> {
        (!self.editor.text().trim().is_empty()).then(|| match self.mode {
            ComposerMode::Agent => ComposerSubmission::AgentMessage(self.editor.text().to_owned()),
            ComposerMode::Shell => ComposerSubmission::ShellCommand(self.editor.text().to_owned()),
        })
    }

    pub(crate) fn apply(&mut self, command: CodeEditorCommand) {
        if self.mode == ComposerMode::Shell {
            match command {
                CodeEditorCommand::MoveUp(CodeEditorSelectionMode::Move)
                    if self.editor.is_collapsed_at_first_row() =>
                {
                    if self.older_shell_history() {
                        return;
                    }
                }
                CodeEditorCommand::MoveDown(CodeEditorSelectionMode::Move)
                    if self.editor.is_collapsed_at_last_row() =>
                {
                    if self.newer_shell_history() {
                        return;
                    }
                }
                _ => self.leave_shell_history(),
            }
        }
        self.editor.apply(command);
    }

    pub(crate) fn apply_composition(&mut self, event: TextInputCompositionEvent) {
        self.leave_shell_history();
        self.editor.apply_composition(event);
    }

    pub(crate) fn cancel_composition(&mut self) {
        self.editor.cancel_composition();
    }

    pub(crate) fn clear_after_submit(&mut self) {
        if self.mode == ComposerMode::Shell {
            let command = self.editor.text().to_owned();
            if self.shell_history.last() != Some(&command) {
                self.shell_history.push(command);
            }
        }
        self.editor.clear();
        self.leave_shell_history();
    }

    pub(crate) fn move_caret_to_point(
        &mut self,
        bounds: Rect,
        point: Point,
        mode: CodeEditorSelectionMode,
    ) -> bool {
        self.leave_shell_history();
        self.editor.move_caret_to_point(bounds, point, mode)
    }

    fn older_shell_history(&mut self) -> bool {
        if self.shell_history.is_empty() {
            return false;
        }
        let index = match self.shell_history_index {
            Some(index) => index.saturating_sub(1),
            None => {
                self.shell_history_draft = Some(self.editor.text().to_owned());
                self.shell_history.len() - 1
            }
        };
        self.shell_history_index = Some(index);
        self.editor.set_text(self.shell_history[index].clone());
        true
    }

    fn newer_shell_history(&mut self) -> bool {
        let Some(index) = self.shell_history_index else {
            return false;
        };
        if index + 1 < self.shell_history.len() {
            let next = index + 1;
            self.shell_history_index = Some(next);
            self.editor.set_text(self.shell_history[next].clone());
        } else {
            let draft = self.shell_history_draft.take().unwrap_or_default();
            self.shell_history_index = None;
            self.editor.set_text(draft);
        }
        true
    }

    fn leave_shell_history(&mut self) {
        self.shell_history_index = None;
        self.shell_history_draft = None;
    }
}

#[cfg(test)]
#[path = "agent_composer_tests.rs"]
mod tests;
