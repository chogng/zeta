//! Host adapter for the Session UI canvas.

use crate::shell_interaction::MAIN_SURFACE;
use crate::shell_style::ShellPalette;
use crate::thread_projection::ThreadProjection;
use crate::workspace_context::WorkspaceContext;
use zeta_ui::Component;
use zeta_ui::ComponentContext;
use zeta_ui::ComponentElement;
use zeta_ui::ComputedElement;
use zeta_ui::UiScene;
use zui::ui::UiNode;

pub(crate) use zeta_session_ui::SessionCanvasLayout;

/// Adapts product workspace metadata and the shell palette to the Session UI header.
pub(crate) struct SessionHeader<'a> {
    bounds: zeta_ui::Rect,
    title: &'a str,
    projection: &'a ThreadProjection,
    metadata: String,
    style: zeta_session_ui::SessionHeaderStyle,
}

impl<'a> SessionHeader<'a> {
    pub(crate) fn new(
        bounds: zeta_ui::Rect,
        title: &'a str,
        projection: &'a ThreadProjection,
        workspace: &WorkspaceContext,
        palette: ShellPalette,
    ) -> Self {
        let metadata = format!(
            "{}  ·  {}  ·  {}  ·  {}",
            workspace.location_label(),
            workspace.working_directory_label(),
            workspace.git_branch_label(),
            workspace.diff_summary_label(),
        );
        let style = zeta_session_ui::SessionHeaderStyle::new(
            palette.surface,
            palette.border,
            palette.surface_raised,
            palette.text,
            palette.text_muted,
            palette.success,
            palette.accent,
            palette.warning,
            palette.error,
        );
        Self {
            bounds,
            title,
            projection,
            metadata,
            style,
        }
    }

    fn inner(&self) -> zeta_session_ui::SessionHeader<'_> {
        zeta_session_ui::SessionHeader::new(
            self.bounds,
            self.title,
            self.metadata.clone(),
            self.projection,
            self.style,
            MAIN_SURFACE,
        )
    }
}

impl Component for SessionHeader<'_> {
    fn element(&self) -> ComponentElement {
        self.inner().element()
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        self.inner().interaction_node(element)
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, element: &ComputedElement) {
        self.inner().compose(context, element)
    }

    fn paint(&self, scene: &mut UiScene) {
        self.inner().paint(scene)
    }
}
