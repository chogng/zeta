//! Layout and presentation for one Agent Session Pane.

use zui::ui::CaretVisibility;
use zui::ui::ComponentContext;
use zui::ui::ElementId;
use zui::ui::Rect;
use zui::ui::TextInputLayoutEngine;
use zui::ui::UiDispatch;

use crate::ComposerPanelLayout;
use crate::SessionCanvasLayout;
use crate::SessionHeader;
use crate::SessionHeaderStyle;
use crate::SessionPaneContext;
use crate::SessionPaneState;
use crate::SessionPaneStyle;
use crate::chat_widget::ChatWidgetLayout;
use crate::chat_widget::ChatWidgetView;
use crate::chat_widget::draw_chat_widget;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SessionPaneLayout {
    chat_widget: ChatWidgetLayout,
    canvas: SessionCanvasLayout,
}

impl SessionPaneLayout {
    pub fn for_bounds(
        bounds: Rect,
        preferred_composer_height: f32,
        preferred_interaction_height: f32,
    ) -> Self {
        let input_pane = ComposerPanelLayout::for_main(
            bounds,
            preferred_composer_height,
            preferred_interaction_height,
        );
        let canvas = SessionCanvasLayout::for_output(input_pane.output());
        Self {
            chat_widget: ChatWidgetLayout::new(canvas.timeline(), input_pane),
            canvas,
        }
    }

    pub const fn header(self) -> Rect {
        self.canvas.header()
    }

    pub const fn timeline(self) -> Rect {
        self.chat_widget.timeline()
    }

    pub const fn composer(self) -> ComposerPanelLayout {
        self.chat_widget.input_pane()
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
    draw_chat_widget(
        component_context,
        layout.chat_widget,
        ChatWidgetView {
            state: view.state.chat_widget(),
            context: view.context,
            caret_visibility: view.caret_visibility,
            dispatch: view.dispatch,
            parent: view.parent,
        },
        text_layout,
        style,
    )
}
