//! Session canvas owned by the product workbench.

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
use zui::ui::{ElementId, UiNode};

use crate::interaction::SESSION_HEADER;
use crate::thread_state::ThreadState;

const SESSION_HEADER_HEIGHT: f32 = 64.0;
const SESSION_HEADER_HORIZONTAL_PADDING: f32 = 20.0;
const SESSION_HEADER_TOP_PADDING: f32 = 10.0;
const SESSION_TITLE_LINE_HEIGHT: f32 = 22.0;
const SESSION_METADATA_LINE_HEIGHT: f32 = 18.0;
const STATUS_HORIZONTAL_PADDING: f32 = 10.0;
const STATUS_HEIGHT: f32 = 24.0;

/// Product-owned geometry for the stable Session header and scrollable Thread timeline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SessionCanvasLayout {
    header: Rect,
    timeline: Rect,
}

impl SessionCanvasLayout {
    pub fn for_output(output: Rect) -> Self {
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

    pub const fn header(self) -> Rect {
        self.header
    }

    pub const fn timeline(self) -> Rect {
        self.timeline
    }
}

/// Colors needed by the Session header renderer.
#[derive(Clone, Copy)]
pub struct SessionHeaderStyle {
    pub surface: zeta_ui::Color,
    pub border: zeta_ui::Color,
    pub surface_raised: zeta_ui::Color,
    pub text: zeta_ui::Color,
    pub text_muted: zeta_ui::Color,
    pub success: zeta_ui::Color,
    pub accent: zeta_ui::Color,
    pub warning: zeta_ui::Color,
    pub error: zeta_ui::Color,
}

impl SessionHeaderStyle {
    /// Creates the resolved colors used by the header.
    pub const fn new(
        surface: zeta_ui::Color,
        border: zeta_ui::Color,
        surface_raised: zeta_ui::Color,
        text: zeta_ui::Color,
        text_muted: zeta_ui::Color,
        success: zeta_ui::Color,
        accent: zeta_ui::Color,
        warning: zeta_ui::Color,
        error: zeta_ui::Color,
    ) -> Self {
        Self {
            surface,
            border,
            surface_raised,
            text,
            text_muted,
            success,
            accent,
            warning,
            error,
        }
    }
}

/// Stable outcome-oriented header for the current Agent Session.
pub struct SessionHeader<'a> {
    bounds: Rect,
    title: &'a str,
    thread_state: &'a ThreadState,
    metadata: String,
    style: SessionHeaderStyle,
    parent: ElementId,
}

impl<'a> SessionHeader<'a> {
    /// Creates a header from host-provided title, metadata, state, and style.
    pub fn new(
        bounds: Rect,
        title: &'a str,
        metadata: String,
        thread_state: &'a ThreadState,
        style: SessionHeaderStyle,
        parent: ElementId,
    ) -> Self {
        Self {
            bounds,
            title,
            thread_state,
            metadata,
            style,
            parent,
        }
    }

    fn title(&self) -> &str {
        (!self.title.is_empty())
            .then_some(self.title)
            .unwrap_or("New session")
    }

    fn metadata(&self) -> &str {
        &self.metadata
    }

    fn status(&self) -> SessionActivity {
        let Some(thread) = self.thread_state.thread() else {
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
            .with_parent(self.parent),
        )
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_rect(
            PaintRect::new(self.bounds, self.style.surface).with_border(Border::new(
                Edges::new(0.0, 0.0, 1.0, 0.0),
                self.style.border,
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
            TextStyle::new(15.0, self.style.text)
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
            TextStyle::new(11.0, self.style.text_muted)
                .with_line_height(SESSION_METADATA_LINE_HEIGHT),
        ));
        scene.draw_rect(
            PaintRect::new(status_bounds, self.style.surface_raised)
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
            TextStyle::new(11.0, status.color(self.style))
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

    const fn color(self, style: SessionHeaderStyle) -> zeta_ui::Color {
        match self {
            Self::Ready | Self::Completed => style.success,
            Self::Working => style.accent,
            Self::NeedsAttention | Self::Stopping | Self::Interrupted => style.warning,
            Self::Failed => style.error,
            Self::Archived => style.text_muted,
        }
    }
}

#[cfg(test)]
#[path = "session_canvas_tests.rs"]
mod tests;
