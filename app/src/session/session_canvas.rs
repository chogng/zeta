//! Session canvas projection owned by the product workbench.

use zeta_protocol::ThreadStatus;
use zeta_protocol::TurnStatus;
use zeta_ui::Border;
use zeta_ui::Component;
use zeta_ui::ComponentElement;
use zeta_ui::ComputedElement;
use zeta_ui::CornerRadii;
use zeta_ui::Edges;
use zeta_ui::Element;
use zeta_ui::FontWeight;
use zeta_ui::PaintRect;
use zeta_ui::Point;
use zeta_ui::Rect;
use zeta_ui::Size;
use zeta_ui::TextBlock;
use zeta_ui::TextStyle;
use zeta_ui::UiScene;
use zui::ui::AccessibilityRole;
use zui::ui::UiNode;

use crate::shell_interaction::MAIN_SURFACE;
use crate::shell_interaction::SESSION_HEADER;
use crate::shell_style::ShellPalette;
use crate::thread_projection::ThreadProjection;
use crate::workspace_context::WorkspaceContext;

const SESSION_HEADER_HEIGHT: f32 = 64.0;
const SESSION_HEADER_HORIZONTAL_PADDING: f32 = 20.0;
const SESSION_HEADER_TOP_PADDING: f32 = 10.0;
const SESSION_TITLE_LINE_HEIGHT: f32 = 22.0;
const SESSION_METADATA_LINE_HEIGHT: f32 = 18.0;
const STATUS_HORIZONTAL_PADDING: f32 = 10.0;
const STATUS_HEIGHT: f32 = 24.0;

/// Product-owned geometry for the stable Session header and scrollable Thread timeline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SessionCanvasLayout {
    header: Rect,
    timeline: Rect,
}

impl SessionCanvasLayout {
    pub(crate) fn for_output(output: Rect) -> Self {
        let header_height = SESSION_HEADER_HEIGHT.min(output.size.height.max(0.0));
        Self {
            header: Rect::from_xywh(
                output.origin.x,
                output.origin.y,
                output.size.width,
                header_height,
            ),
            timeline: Rect::from_xywh(
                output.origin.x,
                output.origin.y + header_height,
                output.size.width,
                (output.size.height - header_height).max(0.0),
            ),
        }
    }

    pub(crate) const fn header(self) -> Rect {
        self.header
    }

    pub(crate) const fn timeline(self) -> Rect {
        self.timeline
    }
}

/// Stable outcome-oriented header for the current Agent Session.
pub(crate) struct SessionHeader<'a> {
    bounds: Rect,
    title: &'a str,
    projection: &'a ThreadProjection,
    workspace: &'a WorkspaceContext,
    palette: ShellPalette,
}

impl<'a> SessionHeader<'a> {
    pub(crate) const fn new(
        bounds: Rect,
        title: &'a str,
        projection: &'a ThreadProjection,
        workspace: &'a WorkspaceContext,
        palette: ShellPalette,
    ) -> Self {
        Self {
            bounds,
            title,
            projection,
            workspace,
            palette,
        }
    }

    fn title(&self) -> &str {
        (!self.title.is_empty())
            .then_some(self.title)
            .unwrap_or("New session")
    }

    fn metadata(&self) -> String {
        format!(
            "{}  ·  {}  ·  {}  ·  {}",
            self.workspace.location_label(),
            self.workspace.working_directory_label(),
            self.workspace.git_branch_label(),
            self.workspace.diff_summary_label(),
        )
    }

    fn status(&self) -> SessionActivity {
        let Some(thread) = self.projection.thread() else {
            return SessionActivity::Ready;
        };
        if thread.status == ThreadStatus::Archived {
            return SessionActivity::Archived;
        }
        match thread.turns.last().map(|turn| turn.status) {
            None => SessionActivity::Ready,
            Some(TurnStatus::Created | TurnStatus::Running) => SessionActivity::Working,
            Some(
                TurnStatus::WaitingForApproval
                | TurnStatus::WaitingForUserInput
                | TurnStatus::WaitingForCapability,
            ) => SessionActivity::NeedsAttention,
            Some(TurnStatus::Cancelling) => SessionActivity::Stopping,
            Some(TurnStatus::Completed) => SessionActivity::Completed,
            Some(TurnStatus::Failed) => SessionActivity::Failed,
            Some(TurnStatus::Interrupted) => SessionActivity::Interrupted,
        }
    }

    fn status_bounds(&self, status: SessionActivity) -> Rect {
        let width = status.label().chars().count() as f32 * 7.0 + STATUS_HORIZONTAL_PADDING * 2.0;
        Rect::from_xywh(
            self.bounds.right() - SESSION_HEADER_HORIZONTAL_PADDING - width,
            self.bounds.origin.y + SESSION_HEADER_TOP_PADDING,
            width,
            STATUS_HEIGHT,
        )
    }
}

impl Component for SessionHeader<'_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("SessionHeader")
            .in_bounds(self.bounds)
            .with_identity(SESSION_HEADER)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        Some(
            UiNode::new(
                SESSION_HEADER,
                element.bounds(),
                AccessibilityRole::Group,
                format!("{} · {}", self.title(), self.status().label()),
            )
            .with_parent(MAIN_SURFACE),
        )
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_rect(
            PaintRect::new(self.bounds, self.palette.surface).with_border(Border::new(
                Edges::new(0.0, 0.0, 1.0, 0.0),
                self.palette.border,
            )),
        );
        let status = self.status();
        let status_bounds = self.status_bounds(status);
        let title_width = (status_bounds.origin.x
            - SESSION_HEADER_HORIZONTAL_PADDING
            - (self.bounds.origin.x + SESSION_HEADER_HORIZONTAL_PADDING))
            .max(1.0);
        scene.draw_text(TextBlock::new(
            self.title(),
            Point::new(
                self.bounds.origin.x + SESSION_HEADER_HORIZONTAL_PADDING,
                self.bounds.origin.y + SESSION_HEADER_TOP_PADDING,
            ),
            Size::new(title_width, SESSION_TITLE_LINE_HEIGHT),
            TextStyle::new(15.0, self.palette.text)
                .with_weight(FontWeight::Bold)
                .with_line_height(SESSION_TITLE_LINE_HEIGHT),
        ));
        scene.draw_text(TextBlock::new(
            self.metadata(),
            Point::new(
                self.bounds.origin.x + SESSION_HEADER_HORIZONTAL_PADDING,
                self.bounds.origin.y + SESSION_HEADER_TOP_PADDING + SESSION_TITLE_LINE_HEIGHT,
            ),
            Size::new(
                (self.bounds.size.width - SESSION_HEADER_HORIZONTAL_PADDING * 2.0).max(1.0),
                SESSION_METADATA_LINE_HEIGHT,
            ),
            TextStyle::new(11.0, self.palette.text_muted)
                .with_line_height(SESSION_METADATA_LINE_HEIGHT),
        ));
        scene.draw_rect(
            PaintRect::new(status_bounds, self.palette.surface_raised)
                .with_corner_radii(CornerRadii::uniform(STATUS_HEIGHT / 2.0)),
        );
        scene.draw_text(TextBlock::new(
            status.label(),
            Point::new(
                status_bounds.origin.x + STATUS_HORIZONTAL_PADDING,
                status_bounds.origin.y + 3.0,
            ),
            Size::new(
                (status_bounds.size.width - STATUS_HORIZONTAL_PADDING * 2.0).max(1.0),
                18.0,
            ),
            TextStyle::new(11.0, status.color(self.palette))
                .with_weight(FontWeight::Bold)
                .with_line_height(18.0),
        ));
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SessionActivity {
    Ready,
    Working,
    NeedsAttention,
    Stopping,
    Completed,
    Failed,
    Interrupted,
    Archived,
}

impl SessionActivity {
    const fn label(self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::Working => "Working",
            Self::NeedsAttention => "Needs input",
            Self::Stopping => "Stopping",
            Self::Completed => "Completed",
            Self::Failed => "Failed",
            Self::Interrupted => "Interrupted",
            Self::Archived => "Archived",
        }
    }

    const fn color(self, palette: ShellPalette) -> zeta_ui::Color {
        match self {
            Self::Ready | Self::Completed => palette.success,
            Self::Working => palette.accent,
            Self::NeedsAttention | Self::Stopping | Self::Interrupted => palette.warning,
            Self::Failed => palette.error,
            Self::Archived => palette.text_muted,
        }
    }
}

#[cfg(test)]
#[path = "session_canvas_tests.rs"]
mod tests;
