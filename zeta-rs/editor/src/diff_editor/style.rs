//! Diff-specific decorations layered on CodeEditor tokens.

use zeta_diff::DiffRowKind;
use zeta_ui::Color;

use super::DiffEditorSide;
use crate::code_editor::CodeEditorStyle;

/// Diff-specific decorations layered onto the shared CodeEditor style.
#[derive(Clone, Debug, PartialEq)]
pub struct DiffEditorStyle {
    code_editor: CodeEditorStyle,
    divider: Color,
    pub(super) removed_line: Color,
    pub(super) added_line: Color,
    pub(super) removed_inline: Color,
    pub(super) added_inline: Color,
    pub(super) missing_line: Color,
}

impl DiffEditorStyle {
    pub fn light() -> Self {
        Self {
            code_editor: CodeEditorStyle::light(),
            divider: Color::rgb(222, 222, 224),
            removed_line: Color::rgb(255, 235, 233),
            added_line: Color::rgb(218, 251, 225),
            removed_inline: Color::rgb(255, 198, 194),
            added_inline: Color::rgb(166, 235, 183),
            missing_line: Color::rgb(248, 248, 249),
        }
    }

    pub(super) const fn code_editor(&self) -> &CodeEditorStyle {
        &self.code_editor
    }

    pub(super) const fn divider(&self) -> Color {
        self.divider
    }

    pub(super) const fn marker_color(&self, side: DiffEditorSide) -> Color {
        match side {
            DiffEditorSide::Original => Color::rgb(207, 34, 46),
            DiffEditorSide::Modified => Color::rgb(26, 127, 55),
        }
    }

    pub(super) const fn inline_color(&self, side: DiffEditorSide) -> Color {
        match side {
            DiffEditorSide::Original => self.removed_inline,
            DiffEditorSide::Modified => self.added_inline,
        }
    }

    pub(super) fn line_background(
        &self,
        kind: DiffRowKind,
        side: DiffEditorSide,
        has_line: bool,
    ) -> Color {
        if !has_line {
            return self.missing_line;
        }
        match (kind, side) {
            (DiffRowKind::Added, DiffEditorSide::Modified)
            | (DiffRowKind::Modified, DiffEditorSide::Modified) => self.added_line,
            (DiffRowKind::Removed, DiffEditorSide::Original)
            | (DiffRowKind::Modified, DiffEditorSide::Original) => self.removed_line,
            _ => self.code_editor.surface(),
        }
    }
}
