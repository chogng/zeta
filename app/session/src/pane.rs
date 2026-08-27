//! Layout and presentation for one Agent Session Pane.

use zeta_ui_components::InteractionRegion;
use zui::ui::AccessibilityRole;
use zui::ui::CaretVisibility;
use zui::ui::ComponentContext;
use zui::ui::CursorFeedback;
use zui::ui::ElementId;
use zui::ui::Rect;
use zui::ui::TextInputLayoutEngine;
use zui::ui::UiDispatch;

use crate::ComposerPanelLayout;
use crate::ComposerPanelView;
use crate::SessionCanvasLayout;
use crate::SessionHeader;
use crate::SessionHeaderStyle;
use crate::SessionPaneContext;
use crate::SessionPaneState;
use crate::SessionPaneStyle;
use crate::ThreadTimeline;
use crate::ThreadTimelineStyle;
use crate::draw_composer_panel;
use crate::interaction::THREAD_TIMELINE;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SessionPaneLayout {
    composer: ComposerPanelLayout,
    canvas: SessionCanvasLayout,
}

impl SessionPaneLayout {
    pub fn for_bounds(
        bounds: Rect,
        preferred_composer_height: f32,
        preferred_interaction_height: f32,
    ) -> Self {
        let composer = ComposerPanelLayout::for_main(
            bounds,
            preferred_composer_height,
            preferred_interaction_height,
        );
        Self {
            canvas: SessionCanvasLayout::for_output(composer.output()),
            composer,
        }
    }

    pub const fn header(self) -> Rect {
        self.canvas.header()
    }

    pub const fn timeline(self) -> Rect {
        self.canvas.timeline()
    }

    pub const fn composer(self) -> ComposerPanelLayout {
        self.composer
    }
}

pub struct SessionPaneView<'a> {
    pub title: &'a str,
    pub state: &'a SessionPaneState,
    pub context: &'a SessionPaneContext,
    pub caret_visibility: CaretVisibility,
    pub dispatch: &'a UiDispatch,
    pub parent: ElementId,
}

pub fn draw_session_pane(
    component_context: &mut ComponentContext<'_, '_>,
    layout: SessionPaneLayout,
    view: SessionPaneView<'_>,
    text_layout: &mut TextInputLayoutEngine,
    style: SessionPaneStyle,
) -> Option<Rect> {
    component_context.draw_component(&SessionHeader::new(
        layout.header(),
        view.title,
        view.context.metadata(),
        view.state.thread(),
        SessionHeaderStyle::new(
            style.surface,
            style.border,
            style.surface_raised,
            style.text,
            style.text_muted,
            style.success,
            style.accent,
            style.warning,
            style.error,
        ),
        view.parent,
    ));
    let timeline = InteractionRegion::new(
        "ThreadTimeline",
        THREAD_TIMELINE,
        layout.timeline(),
        AccessibilityRole::Group,
        "Agent Thread timeline",
    )
    .with_parent(view.parent)
    .with_cursor(CursorFeedback::Text);
    component_context.with_component(&timeline, |component_context, _| {
        component_context.draw_component(&ThreadTimeline::new(
            layout.timeline(),
            view.state.transcript(),
            view.state.timeline_scroll().offset(),
            ThreadTimelineStyle::new(
                style.surface_raised,
                style.text,
                style.text_muted,
                style.error,
            ),
        ));
    });
    draw_composer_panel(
        component_context,
        layout.composer(),
        ComposerPanelView {
            context: view.context,
            editor: view.state.input(),
            interaction: view.state.interaction(),
            interaction_pane: view.state.interaction_pane(),
            route: view.state.composer_route(),
            caret_visibility: view.caret_visibility,
            dispatch: view.dispatch,
            parent: view.parent,
        },
        text_layout,
        style,
    )
}
