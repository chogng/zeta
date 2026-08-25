use zeta_composer::ComposerInput;
use zeta_composer::ComposerInputFocus;
use zeta_composer::ComposerInteractionModel;
use zeta_composer::ComposerInteractionPaneState;
use zeta_composer::ComposerInteractionView;
use zeta_composer::ComposerPanelLayout;
use zeta_composer::ComposerRoute;
use zeta_composer::INTERACTION_ROW_HEIGHT;
use zeta_composer::interaction_list_bounds;
use zeta_ui::{
    Border, CaretVisibility, Color, ComponentContext, CornerRadii, Edges, FontFamily, FontWeight,
    InteractionRegion, KeycapSequence, KeycapStyle, ListView, PaintRect, Point, Rect, ScrollState,
    Size, TextBlock, TextInputLayoutEngine, TextStyle,
};
use zui::{
    AccessibilityRole, AccessibilitySelection, CursorFeedback, FocusBehavior, NodeAction,
    UiDispatch,
};

use crate::input_context_toolbar::InputContextToolbar;
use crate::shell_interaction::{
    COMPOSER, COMPOSER_INFO_BAR, COMPOSER_INTERACTION, COMPOSER_PANEL, MAIN_SURFACE,
    composer_interaction_item_id,
};
use crate::shell_style::ShellPalette;
use crate::workspace_context::WorkspaceContext;

const INFO_KEYCAP_SIZE: f32 = 16.0;
const INFO_KEYCAP_LABEL_GAP: f32 = 6.0;
const INFO_KEYCAP_BACKGROUND: Color = Color::rgb(96, 97, 102);
const INTERACTION_TEXT_INSET: f32 = 10.0;

#[derive(Clone, Copy)]
pub(crate) struct ComposerPanelView<'a> {
    pub(crate) context: &'a WorkspaceContext,
    pub(crate) editor: &'a ComposerInput,
    pub(crate) interaction: &'a ComposerInteractionModel,
    pub(crate) interaction_pane: &'a ComposerInteractionPaneState,
    pub(crate) route: ComposerRoute,
    pub(crate) caret_visibility: CaretVisibility,
    pub(crate) dispatch: &'a UiDispatch,
}

pub(crate) fn draw_composer_panel(
    context: &mut ComponentContext<'_, '_>,
    layout: ComposerPanelLayout,
    view: ComposerPanelView<'_>,
    text_layout: &mut TextInputLayoutEngine,
    palette: ShellPalette,
) -> Option<Rect> {
    let panel = InteractionRegion::new(
        "ComposerPanel",
        COMPOSER_PANEL,
        layout.panel(),
        AccessibilityRole::Group,
        "Command composer",
    )
    .with_parent(MAIN_SURFACE);
    context.with_component(&panel, |context, _| {
        context.scene_mut().draw_rect(
            PaintRect::new(layout.panel(), palette.surface)
                .with_border(Border::new(Edges::new(1.0, 0.0, 0.0, 0.0), palette.border)),
        );
        if let (Some(bounds), Some(interaction)) = (layout.interaction(), view.interaction.view()) {
            draw_interaction(
                context,
                bounds,
                interaction,
                view.interaction_pane.scroll_state(),
                view.dispatch,
                palette,
            );
        }
        draw_info_bar(context, layout.info_bar(), view.route, palette);
        context.scene_mut().draw_rect(PaintRect::new(
            layout.info_editor_separator(),
            palette.border,
        ));
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
            ComposerInputFocus::Focused(view.caret_visibility)
        } else {
            ComposerInputFocus::Blurred
        };
        let placeholder = match view.route {
            ComposerRoute::Agent => "Ask Zeta anything…",
            ComposerRoute::Shell => "Enter a shell command…",
        };
        let editor = view.editor.view(
            layout.editor(),
            placeholder,
            editor_focus,
            palette.text_muted,
        );
        let caret_bounds = editor.caret_bounds();
        context.draw_component(&editor);
        let toolbar = InputContextToolbar::new(
            layout.toolbar(),
            view.context,
            palette,
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
    palette: ShellPalette,
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
            TextStyle::new(12.0, palette.text_muted)
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

fn draw_interaction(
    context: &mut ComponentContext<'_, '_>,
    bounds: Rect,
    view: ComposerInteractionView<'_>,
    scroll_state: ScrollState,
    dispatch: &UiDispatch,
    palette: ShellPalette,
) {
    let interaction = InteractionRegion::new(
        "ComposerInteraction",
        COMPOSER_INTERACTION,
        bounds,
        AccessibilityRole::List,
        view.title(),
    )
    .with_parent(COMPOSER_PANEL);
    context.with_component(&interaction, |context, _| {
        context.scene_mut().draw_rect(
            PaintRect::new(bounds, palette.surface_raised)
                .with_border(Border::uniform(1.0, palette.border))
                .with_corner_radii(CornerRadii::uniform(6.0)),
        );
        let title = if view.can_go_back() {
            format!("← {}", view.title())
        } else {
            view.title().to_owned()
        };
        context.scene_mut().draw_text(TextBlock::new(
            title,
            Point::new(
                bounds.origin.x + INTERACTION_TEXT_INSET,
                bounds.origin.y + 5.0,
            ),
            Size::new(
                (bounds.size.width - INTERACTION_TEXT_INSET * 2.0).max(1.0),
                20.0,
            ),
            TextStyle::new(12.0, palette.text)
                .with_family(FontFamily::Monospace)
                .with_weight(FontWeight::Bold)
                .with_line_height(20.0),
        ));
        let list_bounds = interaction_list_bounds(bounds);
        if view.items().is_empty() {
            context.scene_mut().draw_text(TextBlock::new(
                "No matching items",
                Point::new(
                    list_bounds.origin.x + INTERACTION_TEXT_INSET,
                    list_bounds.origin.y + 7.0,
                ),
                Size::new(
                    (list_bounds.size.width - INTERACTION_TEXT_INSET * 2.0).max(1.0),
                    INTERACTION_ROW_HEIGHT,
                ),
                TextStyle::new(12.0, palette.text_muted)
                    .with_family(FontFamily::Monospace)
                    .with_line_height(20.0),
            ));
            return;
        }
        let list = ListView::new(
            list_bounds,
            view.items().len(),
            INTERACTION_ROW_HEIGHT,
            scroll_state,
            palette.file_list_scroll_view_style(),
        );
        for index in list.visible_range() {
            let item_bounds = list
                .item_bounds(index)
                .expect("visible interaction item")
                .intersection(list_bounds);
            let id = composer_interaction_item_id(index);
            context.draw_component(
                &InteractionRegion::new(
                    "ComposerInteractionItem",
                    id,
                    item_bounds,
                    AccessibilityRole::ListItem,
                    format!(
                        "{}, {}",
                        view.items()[index].label(),
                        view.items()[index].description()
                    ),
                )
                .with_parent(COMPOSER_INTERACTION)
                .with_cursor(CursorFeedback::Pointer)
                .with_action(NodeAction::Activate)
                .with_selection(if index == view.selected() {
                    AccessibilitySelection::Selected
                } else {
                    AccessibilitySelection::Unselected
                }),
            );
        }
        list.draw(context.scene_mut(), |scene, layout| {
            let index = layout.index();
            let item = &view.items()[index];
            let item_bounds = layout.bounds();
            let y = item_bounds.origin.y;
            let id = composer_interaction_item_id(index);
            let selected = index == view.selected();
            if selected || dispatch.is_hovered(id) || dispatch.is_pressed(id) {
                scene.draw_rect(PaintRect::new(
                    item_bounds,
                    if selected {
                        palette.session_tab_highlight
                    } else {
                        palette.surface_hovered
                    },
                ));
            }
            let label_width = (item_bounds.size.width * 0.34).max(100.0);
            scene.draw_text(TextBlock::new(
                item.label(),
                Point::new(item_bounds.origin.x + INTERACTION_TEXT_INSET, y + 7.0),
                Size::new(label_width, 20.0),
                TextStyle::new(12.0, palette.text)
                    .with_family(FontFamily::Monospace)
                    .with_line_height(20.0),
            ));
            scene.draw_text(TextBlock::new(
                item.description(),
                Point::new(
                    item_bounds.origin.x + INTERACTION_TEXT_INSET + label_width,
                    y + 7.0,
                ),
                Size::new(
                    (item_bounds.size.width - INTERACTION_TEXT_INSET * 3.0 - label_width).max(1.0),
                    20.0,
                ),
                TextStyle::new(12.0, palette.text_muted)
                    .with_family(FontFamily::Monospace)
                    .with_line_height(20.0),
            ));
        });
    });
}

#[cfg(test)]
#[path = "composer_panel_tests.rs"]
mod tests;
