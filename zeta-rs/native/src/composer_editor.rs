use zeta_editor::{
    CodeEditor, CodeEditorCommand, CodeEditorDocument, CodeEditorHeader, CodeEditorPresentation,
    CodeEditorRowSource, CodeEditorSelectionMode, CodeEditorStyle, CodeEditorViewport,
};
use zeta_ui::{
    CaretVisibility, Color, Component, FontFamily, Point, Rect, TextBlock,
    TextInputCompositionEvent, TextStyle, UiScene,
};

use crate::composer_syntax::{ComposerPlainTextSyntax, ComposerShellSyntax};

const MAX_VISIBLE_ROWS: usize = 8;
const MIN_EDITOR_HEIGHT: f32 = 44.0;
const PLACEHOLDER_HORIZONTAL_INSET: f32 = 12.0;

/// Focus projection used by the compact composer editor.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ComposerEditorFocus {
    #[default]
    Blurred,
    Focused(CaretVisibility),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ComposerEditorSyntax {
    #[default]
    PlainText,
    Shell,
}

/// Product-owned multiline document and retained viewport for the Agent composer.
pub(crate) struct ComposerEditor {
    document: CodeEditorDocument,
    viewport: CodeEditorViewport,
    syntax: ComposerEditorSyntax,
    shell_syntax: ComposerShellSyntax,
    style: CodeEditorStyle,
}

impl Default for ComposerEditor {
    fn default() -> Self {
        Self {
            document: CodeEditorDocument::from_text(""),
            viewport: CodeEditorViewport::default(),
            syntax: ComposerEditorSyntax::PlainText,
            shell_syntax: ComposerShellSyntax::new(),
            style: CodeEditorStyle::light(),
        }
    }
}

impl ComposerEditor {
    pub(crate) fn text(&self) -> &str {
        self.document.text()
    }

    pub(crate) fn selected_text(&self) -> Option<&str> {
        self.document.selected_text()
    }

    pub(crate) fn row_count(&self) -> usize {
        self.document.row_count()
    }

    pub(crate) fn visible_row_count(&self) -> usize {
        self.row_count().clamp(1, MAX_VISIBLE_ROWS)
    }

    pub(crate) fn preferred_height(&self) -> f32 {
        (self.visible_row_count() as f32 * CodeEditor::row_height()).max(MIN_EDITOR_HEIGHT)
    }

    pub(crate) fn apply(&mut self, command: CodeEditorCommand) {
        let changes_text = matches!(
            &command,
            CodeEditorCommand::Insert(_)
                | CodeEditorCommand::Newline
                | CodeEditorCommand::Backspace
                | CodeEditorCommand::DeleteForward
                | CodeEditorCommand::Undo
                | CodeEditorCommand::Redo
        );
        self.document.apply(command);
        if changes_text {
            self.refresh_syntax();
        }
        self.reveal_caret();
    }

    pub(crate) fn apply_composition(&mut self, event: TextInputCompositionEvent) {
        let commits_text = matches!(&event, TextInputCompositionEvent::Commit(_));
        self.document.apply_composition(event);
        if commits_text {
            self.refresh_syntax();
        }
        self.reveal_caret();
    }

    pub(crate) fn cancel_composition(&mut self) {
        self.document.cancel_composition();
    }

    pub(crate) fn clear(&mut self) {
        self.document.replace_text("");
        self.viewport = CodeEditorViewport::default();
        self.refresh_syntax();
    }

    pub(crate) fn set_text(&mut self, text: impl Into<String>) {
        self.document.replace_text(text);
        self.document.apply(CodeEditorCommand::SelectAll);
        self.document
            .apply(CodeEditorCommand::MoveRight(CodeEditorSelectionMode::Move));
        self.refresh_syntax();
        self.reveal_caret();
    }

    pub(crate) fn set_syntax(&mut self, syntax: ComposerEditorSyntax) {
        if self.syntax == syntax {
            return;
        }
        self.syntax = syntax;
        self.refresh_syntax();
    }

    pub(crate) fn set_style(&mut self, style: CodeEditorStyle) {
        self.style = style;
    }

    pub(crate) fn is_collapsed_at_first_row(&self) -> bool {
        self.document.anchor() == self.document.cursor()
            && self
                .document
                .caret()
                .is_some_and(|caret| caret.row_index == 0)
    }

    pub(crate) fn is_collapsed_at_last_row(&self) -> bool {
        self.document.anchor() == self.document.cursor()
            && self
                .document
                .caret()
                .is_some_and(|caret| caret.row_index + 1 == self.document.row_count())
    }

    pub(crate) fn move_caret_to_point(
        &mut self,
        bounds: Rect,
        point: Point,
        mode: CodeEditorSelectionMode,
    ) -> bool {
        let editor = self.code_editor(bounds, CaretVisibility::Visible);
        let Some(position) = editor.text_position_at(point) else {
            return false;
        };
        self.document.move_to(position, mode);
        self.reveal_caret();
        true
    }

    pub(crate) fn view<'a>(
        &'a self,
        bounds: Rect,
        placeholder: &'a str,
        focus: ComposerEditorFocus,
        placeholder_color: Color,
    ) -> ComposerEditorView<'a> {
        ComposerEditorView {
            bounds,
            editor: self,
            placeholder,
            focus,
            placeholder_color,
        }
    }

    fn reveal_caret(&mut self) {
        let Some(caret) = self.document.caret() else {
            return;
        };
        self.viewport
            .reveal_row(caret.row_index, self.document.row_count(), MAX_VISIBLE_ROWS);
    }

    fn refresh_syntax(&mut self) {
        match self.syntax {
            ComposerEditorSyntax::PlainText => self.document.apply_syntax(&ComposerPlainTextSyntax),
            ComposerEditorSyntax::Shell => {
                if let Some(projection) = self.shell_syntax.synchronize(self.document.text()) {
                    self.document.apply_syntax(&projection);
                } else {
                    self.document.apply_syntax(&ComposerPlainTextSyntax);
                }
            }
        }
    }

    fn code_editor(&self, bounds: Rect, caret_visibility: CaretVisibility) -> CodeEditor<'_> {
        CodeEditor::new(
            bounds,
            &self.document,
            self.viewport,
            CodeEditorHeader::Hidden,
            self.style.clone(),
        )
        .with_presentation(CodeEditorPresentation::Compact)
        .with_caret_visibility(caret_visibility)
    }
}

/// Compact CodeEditor projection with Agent-composer placeholder semantics.
pub(crate) struct ComposerEditorView<'a> {
    bounds: Rect,
    editor: &'a ComposerEditor,
    placeholder: &'a str,
    focus: ComposerEditorFocus,
    placeholder_color: Color,
}

impl ComposerEditorView<'_> {
    pub(crate) fn caret_bounds(&self) -> Option<Rect> {
        let ComposerEditorFocus::Focused(CaretVisibility::Visible) = self.focus else {
            return None;
        };
        self.editor
            .code_editor(self.bounds, CaretVisibility::Visible)
            .caret_bounds()
    }
}

impl Component for ComposerEditorView<'_> {
    fn paint(&self, scene: &mut UiScene) {
        let caret_visibility = match self.focus {
            ComposerEditorFocus::Blurred => CaretVisibility::Hidden,
            ComposerEditorFocus::Focused(visibility) => visibility,
        };
        scene.draw_component(&self.editor.code_editor(self.bounds, caret_visibility));
        if self.editor.text().is_empty() {
            scene.with_clip(self.bounds, |scene| {
                scene.draw_text(TextBlock::new(
                    self.placeholder,
                    Point::new(
                        self.bounds.origin.x + PLACEHOLDER_HORIZONTAL_INSET,
                        self.bounds.origin.y,
                    ),
                    zeta_ui::Size::new(
                        (self.bounds.size.width - PLACEHOLDER_HORIZONTAL_INSET).max(0.0),
                        self.bounds.size.height,
                    ),
                    TextStyle::new(13.0, self.placeholder_color)
                        .with_family(FontFamily::Monospace)
                        .with_line_height(CodeEditor::row_height()),
                ));
            });
        }
    }
}

#[cfg(test)]
#[path = "composer_editor_tests.rs"]
mod tests;
