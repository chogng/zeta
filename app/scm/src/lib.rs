//! Source-control snapshots for the Changes Workspace Pane.

use zeta_editor::DiffEditorDocument;
use zeta_ui_theme::UiTheme;
use zui::ui::Color;

#[path = "scm/layout.rs"]
mod layout;
#[path = "scm/pane.rs"]
mod pane;

pub use layout::ScmLayout;
pub use pane::EditorPane;
pub use pane::EditorPaneState;
pub use pane::ScrollbarPointerOutcome;

pub const CHANGES_PANE: zui::ui::ElementId = zui::ui::ElementId::scoped(1, 29);
pub const MULTI_DIFF_EDITOR: zui::ui::ElementId = zui::ui::ElementId::scoped(1, 30);
pub const MULTI_DIFF_SCROLLBAR: zui::ui::ElementId = zui::ui::ElementId::scoped(1, 31);

/// Theme values required by the SCM pane. Shell theme ownership remains in the host.
#[derive(Clone, Copy)]
pub struct ScmPaneStyle {
    pub surface: Color,
    pub border: Color,
    pub text_muted: Color,
}

impl ScmPaneStyle {
    pub const fn from_theme(theme: UiTheme) -> Self {
        Self {
            surface: theme.content_background,
            border: theme.border,
            text_muted: theme.muted_foreground,
        }
    }
}

/// One changed-file snapshot supplied by the workspace host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScmDiff {
    path: String,
    document: DiffEditorDocument,
}

impl ScmDiff {
    pub fn new(path: impl Into<String>, document: DiffEditorDocument) -> Self {
        Self {
            path: path.into(),
            document,
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }
    pub const fn document(&self) -> &DiffEditorDocument {
        &self.document
    }
}

/// Retained source-control snapshot for the Changes pane.
///
/// The host maps its repository service result to this model; pane code must not
/// depend on a particular Git transport or workspace implementation.
pub struct ScmState {
    diffs: Vec<ScmDiff>,
    editor: EditorPaneState,
}

impl Default for ScmState {
    fn default() -> Self {
        Self {
            diffs: Vec::new(),
            editor: EditorPaneState::default(),
        }
    }
}

impl ScmState {
    pub fn replace_diffs(
        &mut self,
        diffs: impl IntoIterator<Item = ScmDiff>,
    ) -> Vec<zeta_editor::MultiDiffEditorItemIdentity> {
        self.diffs = diffs.into_iter().collect();
        self.editor.replace_diffs(&self.diffs)
    }

    pub const fn editor(&self) -> &EditorPaneState {
        &self.editor
    }
    pub fn editor_mut(&mut self) -> &mut EditorPaneState {
        &mut self.editor
    }
}

#[cfg(test)]
pub(crate) const TEST_SCM_PANE_STYLE: ScmPaneStyle = ScmPaneStyle {
    surface: Color::WHITE,
    border: Color::rgb(222, 222, 224),
    text_muted: Color::rgb(126, 126, 132),
};
