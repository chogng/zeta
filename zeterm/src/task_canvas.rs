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
use zui::AccessibilityRole;
use zui::UiNode;

use crate::shell_interaction::MAIN_SURFACE;
use crate::shell_interaction::TASK_HEADER;
use crate::shell_style::ShellPalette;
use crate::thread_projection::ThreadProjection;
use crate::workspace_context::WorkspaceContext;

const TASK_HEADER_HEIGHT: f32 = 64.0;
const TASK_HEADER_HORIZONTAL_PADDING: f32 = 20.0;
const TASK_HEADER_TOP_PADDING: f32 = 10.0;
const TASK_TITLE_LINE_HEIGHT: f32 = 22.0;
const TASK_METADATA_LINE_HEIGHT: f32 = 18.0;
const STATUS_HORIZONTAL_PADDING: f32 = 10.0;
const STATUS_HEIGHT: f32 = 24.0;

/// Product-owned geometry for the stable task header and scrollable evidence timeline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TaskCanvasLayout {
    header: Rect,
    timeline: Rect,
}

impl TaskCanvasLayout {
    pub(crate) fn for_output(output: Rect) -> Self {
        let header_height = TASK_HEADER_HEIGHT.min(output.size.height.max(0.0));
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

/// Stable outcome-oriented header for the current Agent task.
pub(crate) struct TaskHeader<'a> {
    bounds: Rect,
    projection: &'a ThreadProjection,
    workspace: &'a WorkspaceContext,
    palette: ShellPalette,
}

impl<'a> TaskHeader<'a> {
    pub(crate) const fn new(
        bounds: Rect,
        projection: &'a ThreadProjection,
        workspace: &'a WorkspaceContext,
        palette: ShellPalette,
    ) -> Self {
        Self {
            bounds,
            projection,
            workspace,
            palette,
        }
    }

    fn title(&self) -> &str {
        self.projection
            .thread()
            .map(|thread| thread.title.as_str())
            .filter(|title| !title.is_empty())
            .unwrap_or("New task")
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

    fn status(&self) -> TaskStatus {
        let Some(thread) = self.projection.thread() else {
            return TaskStatus::Ready;
        };
        if thread.status == ThreadStatus::Archived {
            return TaskStatus::Archived;
        }
        match thread.turns.last().map(|turn| turn.status) {
            None => TaskStatus::Ready,
            Some(TurnStatus::Created | TurnStatus::Running) => TaskStatus::Working,
            Some(
                TurnStatus::WaitingForApproval
                | TurnStatus::WaitingForUserInput
                | TurnStatus::WaitingForCapability,
            ) => TaskStatus::NeedsAttention,
            Some(TurnStatus::Cancelling) => TaskStatus::Stopping,
            Some(TurnStatus::Completed) => TaskStatus::Completed,
            Some(TurnStatus::Failed) => TaskStatus::Failed,
            Some(TurnStatus::Interrupted) => TaskStatus::Interrupted,
        }
    }

    fn status_bounds(&self, status: TaskStatus) -> Rect {
        let width = status.label().chars().count() as f32 * 7.0 + STATUS_HORIZONTAL_PADDING * 2.0;
        Rect::from_xywh(
            self.bounds.right() - TASK_HEADER_HORIZONTAL_PADDING - width,
            self.bounds.origin.y + TASK_HEADER_TOP_PADDING,
            width,
            STATUS_HEIGHT,
        )
    }
}

impl Component for TaskHeader<'_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("TaskHeader")
            .in_bounds(self.bounds)
            .with_identity(TASK_HEADER)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        Some(
            UiNode::new(
                TASK_HEADER,
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
            - TASK_HEADER_HORIZONTAL_PADDING
            - (self.bounds.origin.x + TASK_HEADER_HORIZONTAL_PADDING))
            .max(1.0);
        scene.draw_text(TextBlock::new(
            self.title(),
            Point::new(
                self.bounds.origin.x + TASK_HEADER_HORIZONTAL_PADDING,
                self.bounds.origin.y + TASK_HEADER_TOP_PADDING,
            ),
            Size::new(title_width, TASK_TITLE_LINE_HEIGHT),
            TextStyle::new(15.0, self.palette.text)
                .with_weight(FontWeight::Bold)
                .with_line_height(TASK_TITLE_LINE_HEIGHT),
        ));
        scene.draw_text(TextBlock::new(
            self.metadata(),
            Point::new(
                self.bounds.origin.x + TASK_HEADER_HORIZONTAL_PADDING,
                self.bounds.origin.y + TASK_HEADER_TOP_PADDING + TASK_TITLE_LINE_HEIGHT,
            ),
            Size::new(
                (self.bounds.size.width - TASK_HEADER_HORIZONTAL_PADDING * 2.0).max(1.0),
                TASK_METADATA_LINE_HEIGHT,
            ),
            TextStyle::new(11.0, self.palette.text_muted)
                .with_line_height(TASK_METADATA_LINE_HEIGHT),
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
enum TaskStatus {
    Ready,
    Working,
    NeedsAttention,
    Stopping,
    Completed,
    Failed,
    Interrupted,
    Archived,
}

impl TaskStatus {
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
#[path = "task_canvas_tests.rs"]
mod tests;
