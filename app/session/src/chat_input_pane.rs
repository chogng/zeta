mod layout;

use std::path::PathBuf;

pub use layout::ComposerPanelLayout;
pub use layout::INTERACTION_ROW_HEIGHT;
pub use layout::interaction_content_size;
pub use layout::interaction_list_bounds;
pub use layout::interaction_preferred_height;
pub use layout::interaction_selection_scroll_command;

use crate::ChatInput;
use crate::ChatInputEditor;
use crate::ChatInputFocus;
use crate::ChatInputInteractionPaneState;
use crate::ChatInputInteractionState;
use crate::ComposerRoute;
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
use crate::chat_input_interaction_pane::draw_chat_input_interaction_pane;
use crate::chat_input_toolbar::ChatInputToolbar;
use crate::interaction::{COMPOSER, COMPOSER_INFO_BAR, COMPOSER_PANEL};

const INFO_KEYCAP_SIZE: f32 = 16.0;
const INFO_KEYCAP_LABEL_GAP: f32 = 6.0;
const INFO_KEYCAP_BACKGROUND: Color = Color::rgb(96, 97, 102);

/// Retained input and interaction-surface state for one ChatWidget.
pub(crate) struct ChatInputPaneState {
    input: ChatInput,
    interaction_pane: ChatInputInteractionPaneState,
}

impl ChatInputPaneState {
    pub(crate) fn for_working_directory(working_directory: impl Into<PathBuf>) -> Self {
        Self {
            input: ChatInput::for_working_directory(working_directory),
            interaction_pane: ChatInputInteractionPaneState::default(),
        }
    }

    pub(crate) const fn input(&self) -> &ChatInput {
        &self.input
    }

    pub(crate) fn update_input<R>(&mut self, update: impl FnOnce(&mut ChatInput) -> R) -> R {
        let previous_surface = self.input.interaction().surface();
        let result = update(&mut self.input);
        if previous_surface != self.input.interaction().surface() {
            self.interaction_pane.reset();
        }
        result
    }

    pub(crate) const fn interaction_pane(&self) -> &ChatInputInteractionPaneState {
        &self.interaction_pane
    }

    pub(crate) fn interaction_pane_mut(&mut self) -> &mut ChatInputInteractionPaneState {
        &mut self.interaction_pane
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ChatInputPaneView<'a> {
    pub context: &'a SessionPaneContext,
    pub editor: &'a ChatInputEditor,
    pub interaction: &'a ChatInputInteractionState,
    pub interaction_pane: &'a ChatInputInteractionPaneState,
    pub route: ComposerRoute,
    pub caret_visibility: CaretVisibility,
    pub dispatch: &'a UiDispatch,
    pub parent: ElementId,
}

pub(crate) fn draw_chat_input_pane(
    context: &mut ComponentContext<'_, '_>,
    layout: ComposerPanelLayout,
    view: ChatInputPaneView<'_>,
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
        if let (Some(bounds), Some(interaction)) = (layout.interaction(), view.interaction.view()) {
            draw_chat_input_interaction_pane(
                context,
                bounds,
                interaction,
                view.interaction_pane.scroll_state(),
                view.dispatch,
                style,
            );
        }
        draw_info_bar(context, layout.info_bar(), view.route, style);
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
            .with_value(view.editor.text()),
        );
        let editor_focus = if view.dispatch.is_focused(COMPOSER) {
            ChatInputFocus::Focused(view.caret_visibility)
        } else {
            ChatInputFocus::Blurred
        };
        let placeholder = match view.route {
            ComposerRoute::Agent => "Ask Zeta anything…",
            ComposerRoute::Shell => "Enter a shell command…",
        };
        let editor = view
            .editor
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
#[path = "chat_input_pane_tests.rs"]
mod tests;
