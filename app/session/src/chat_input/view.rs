use zeta_ui_components::InteractionRegion;
use zeta_ui_components::KeycapSequence;
use zeta_ui_components::KeycapStyle;
use zui::ui::{AccessibilityRole, CursorFeedback, FocusBehavior, UiDispatch};
use zui::ui::{
    Border, CaretVisibility, Color, ComponentContext, CornerRadii, Edges, ElementId, FontFamily,
    PaintRect, Point, Rect, Size, TextBlock, TextInputLayoutEngine, TextStyle,
};

use crate::SessionPaneContext;
use crate::SessionPaneStyle;
use crate::interaction::{COMPOSER, COMPOSER_INFO_BAR, COMPOSER_PANEL};

use super::ChatInput;
use super::ComposerPanelLayout;
use super::ComposerRoute;
use super::editor::ChatInputFocus;
use super::interaction_view::draw_chat_input_interaction;
use super::toolbar::ChatInputToolbar;

const INFO_KEYCAP_SIZE: f32 = 16.0;
const INFO_KEYCAP_LABEL_GAP: f32 = 6.0;
const INFO_KEYCAP_BACKGROUND: Color = Color::rgb(96, 97, 102);

#[derive(Clone, Copy)]
pub(crate) struct ChatInputView<'a> {
    pub(crate) input: &'a ChatInput,
    pub(crate) context: &'a SessionPaneContext,
    pub(crate) caret_visibility: CaretVisibility,
    pub(crate) dispatch: &'a UiDispatch,
    pub(crate) parent: ElementId,
}

pub(crate) fn draw_chat_input(
    context: &mut ComponentContext<'_, '_>,
    layout: ComposerPanelLayout,
    view: ChatInputView<'_>,
    text_layout: &mut TextInputLayoutEngine,
    style: SessionPaneStyle,
) -> Option<Rect> {
    let panel = InteractionRegion::new(
        "ComposerPanel",
        COMPOSER_PANEL,
        layout.panel(),
        AccessibilityRole::Group,
        "Command composer",
    )
    .with_parent(view.parent);
    context.with_component(&panel, |context, _| {
        context.scene_mut().draw_rect(
            PaintRect::new(layout.panel(), style.surface)
                .with_border(Border::new(Edges::new(1.0, 0.0, 0.0, 0.0), style.border)),
        );
        if let (Some(bounds), Some(interaction)) =
            (layout.interaction(), view.input.interaction().view())
        {
            draw_chat_input_interaction(
                context,
                bounds,
                interaction,
                view.input.interaction_scroll(),
                view.dispatch,
                style,
            );
        }
        draw_info_bar(context, layout.info_bar(), view.input.route(), style);
        context
            .scene_mut()
            .draw_rect(PaintRect::new(layout.info_editor_separator(), style.border));
        context.draw_component(
            &InteractionRegion::new(
                "ComposerInput",
                COMPOSER,
                layout.editor(),
                AccessibilityRole::TextInput,
                "Command input",
            )
            .with_parent(COMPOSER_PANEL)
            .with_cursor(CursorFeedback::Text)
            .with_focus(FocusBehavior::TabStop)
            .with_value(view.input.input().text()),
        );
        let editor_focus = if view.dispatch.is_focused(COMPOSER) {
            ChatInputFocus::Focused(view.caret_visibility)
        } else {
            ChatInputFocus::Blurred
        };
        let placeholder = match view.input.route() {
            ComposerRoute::Agent => "Ask Zeta anything…",
            ComposerRoute::Shell => "Enter a shell command…",
        };
        let editor =
            view.input
                .input()
                .view(layout.editor(), placeholder, editor_focus, style.text_muted);
        let caret_bounds = editor.caret_bounds();
        context.draw_component(&editor);
        let toolbar = ChatInputToolbar::new(
            layout.toolbar(),
            view.context,
            style,
            text_layout,
            view.dispatch,
        );
        context.draw_component(&toolbar);
        caret_bounds
    })
}

fn draw_info_bar(
    context: &mut ComponentContext<'_, '_>,
    bounds: Rect,
    route: ComposerRoute,
    style: SessionPaneStyle,
) {
    let (accessibility_label, keycaps, label) = match route {
        ComposerRoute::Agent => ("/ for commands", vec![vec!["/".to_owned()]], "for commands"),
        ComposerRoute::Shell => (
            "Up and Down for command history",
            vec![vec!["↑".to_owned(), "↓".to_owned()]],
            "for command history",
        ),
    };
    let info_bar = InteractionRegion::new(
        "ComposerInfoBar",
        COMPOSER_INFO_BAR,
        bounds,
        AccessibilityRole::Group,
        accessibility_label,
    )
    .with_parent(COMPOSER_PANEL);
    context.with_component(&info_bar, |context, _| {
        let keycaps = KeycapSequence::new(
            Point::new(
                bounds.origin.x,
                bounds.origin.y + (bounds.size.height - INFO_KEYCAP_SIZE).max(0.0) * 0.5,
            ),
            keycaps,
            info_keycap_style(),
        );
        let label_x = keycaps.bounds().right() + INFO_KEYCAP_LABEL_GAP;
        context.draw_component(&keycaps);
        context.scene_mut().draw_text(TextBlock::new(
            label,
            Point::new(label_x, bounds.origin.y + 2.0),
            Size::new((bounds.right() - label_x).max(1.0), 20.0),
            TextStyle::new(12.0, style.text_muted)
                .with_family(FontFamily::Monospace)
                .with_line_height(20.0),
        ));
    });
}

fn info_keycap_style() -> KeycapStyle {
    KeycapStyle::new(INFO_KEYCAP_BACKGROUND, Color::WHITE)
        .with_text_style(
            TextStyle::new(10.0, Color::WHITE)
                .with_family(FontFamily::Monospace)
                .with_line_height(12.0),
        )
        .with_corner_radii(CornerRadii::uniform(3.0))
        .with_height(INFO_KEYCAP_SIZE)
        .with_minimum_width(INFO_KEYCAP_SIZE)
        .with_horizontal_padding(3.0)
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
