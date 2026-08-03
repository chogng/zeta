use zeta_ui::{
    Border, CaretVisibility, Color, CornerRadii, Edges, Element, FontFamily, FontWeight,
    KeycapSequence, KeycapStyle, ListView, PaintRect, Point, Rect, ScrollState, Size, TextBlock,
    TextInputLayoutEngine, TextStyle, UiScene,
};
use zui::{
    AccessibilityRole, AccessibilitySelection, CursorFeedback, FocusBehavior, InteractionFrame,
    NodeAction, UiDispatch, UiNode,
};

use crate::agent_composer::ComposerMode;
use crate::composer_editor::{ComposerEditor, ComposerEditorFocus};
use crate::composer_interaction::{ComposerInteractionModel, ComposerInteractionView};
use crate::composer_interaction_pane::ComposerInteractionPaneState;
use crate::input_context_toolbar::InputContextToolbar;
use crate::shell_interaction::{
    COMPOSER, COMPOSER_INFO_BAR, COMPOSER_INTERACTION, COMPOSER_PANEL, MAIN_SURFACE,
    composer_interaction_item_id,
};
use crate::shell_style::ShellPalette;
use crate::workspace_context::WorkspaceContext;

const PANEL_HORIZONTAL_INSET: f32 = 24.0;
const PANEL_TOP_INSET: f32 = 8.0;
const PANEL_BOTTOM_INSET: f32 = 12.0;
const PANEL_SECTION_GAP: f32 = 8.0;
const INFO_BAR_HEIGHT: f32 = 24.0;
const INFO_KEYCAP_SIZE: f32 = 16.0;
const INFO_KEYCAP_LABEL_GAP: f32 = 6.0;
const INFO_KEYCAP_BACKGROUND: Color = Color::rgb(96, 97, 102);
const INFO_EDITOR_SEPARATOR_HEIGHT: f32 = 1.0;
const TOOLBAR_HEIGHT: f32 = 24.0;
const MIN_OUTPUT_HEIGHT: f32 = 40.0;
const INTERACTION_HEADER_HEIGHT: f32 = 30.0;
const INTERACTION_TEXT_INSET: f32 = 10.0;
const MAX_VISIBLE_INTERACTION_ROWS: usize = 8;
pub(crate) const INTERACTION_ROW_HEIGHT: f32 = 34.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ComposerPanelLayout {
    panel: Rect,
    interaction: Option<Rect>,
    info_bar: Rect,
    info_editor_separator: Rect,
    editor: Rect,
    toolbar: Rect,
    output: Rect,
}

impl ComposerPanelLayout {
    pub(crate) fn for_main(
        main: Rect,
        preferred_editor_height: f32,
        preferred_interaction_height: f32,
    ) -> Self {
        let editor_height = preferred_editor_height.max(1.0);
        let base_height = PANEL_TOP_INSET
            + INFO_BAR_HEIGHT
            + PANEL_SECTION_GAP
            + editor_height
            + PANEL_SECTION_GAP
            + TOOLBAR_HEIGHT
            + PANEL_BOTTOM_INSET;
        let requested_interaction_gap = (preferred_interaction_height > 0.0)
            .then_some(PANEL_SECTION_GAP)
            .unwrap_or(0.0);
        let maximum_interaction_height =
            (main.size.height - base_height - requested_interaction_gap - MIN_OUTPUT_HEIGHT)
                .max(0.0);
        let interaction_height = preferred_interaction_height
            .max(0.0)
            .min(maximum_interaction_height);
        let interaction_gap = (interaction_height > 0.0)
            .then_some(PANEL_SECTION_GAP)
            .unwrap_or(0.0);
        let panel_height = base_height + interaction_height + interaction_gap;
        let panel = Rect::from_xywh(
            main.origin.x,
            main.bottom() - panel_height,
            main.size.width,
            panel_height,
        );
        let content_x = main.origin.x + PANEL_HORIZONTAL_INSET;
        let content_width = (main.size.width - PANEL_HORIZONTAL_INSET * 2.0).max(1.0);
        let interaction = (interaction_height > 0.0).then(|| {
            Rect::from_xywh(
                content_x,
                panel.origin.y + PANEL_TOP_INSET,
                content_width,
                interaction_height,
            )
        });
        let toolbar = Rect::from_xywh(
            content_x,
            panel.bottom() - PANEL_BOTTOM_INSET - TOOLBAR_HEIGHT,
            content_width,
            TOOLBAR_HEIGHT,
        );
        let editor = Rect::from_xywh(
            content_x,
            toolbar.origin.y - PANEL_SECTION_GAP - editor_height,
            content_width,
            editor_height,
        );
        let info_bar = Rect::from_xywh(
            content_x,
            editor.origin.y - PANEL_SECTION_GAP - INFO_BAR_HEIGHT,
            content_width,
            INFO_BAR_HEIGHT,
        );
        let info_editor_separator = Rect::from_xywh(
            panel.origin.x,
            editor.origin.y - INFO_EDITOR_SEPARATOR_HEIGHT,
            panel.size.width,
            INFO_EDITOR_SEPARATOR_HEIGHT,
        );
        let output = Rect::from_xywh(
            main.origin.x,
            main.origin.y,
            main.size.width,
            (panel.origin.y - main.origin.y).max(1.0),
        );
        Self {
            panel,
            interaction,
            info_bar,
            info_editor_separator,
            editor,
            toolbar,
            output,
        }
    }

    pub(crate) const fn panel(self) -> Rect {
        self.panel
    }

    #[cfg(test)]
    pub(crate) const fn interaction(self) -> Option<Rect> {
        self.interaction
    }

    pub(crate) const fn editor(self) -> Rect {
        self.editor
    }

    pub(crate) const fn info_bar(self) -> Rect {
        self.info_bar
    }

    pub(crate) const fn info_editor_separator(self) -> Rect {
        self.info_editor_separator
    }

    pub(crate) const fn toolbar(self) -> Rect {
        self.toolbar
    }

    pub(crate) const fn output(self) -> Rect {
        self.output
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ComposerPanelView<'a> {
    pub(crate) context: &'a WorkspaceContext,
    pub(crate) editor: &'a ComposerEditor,
    pub(crate) interaction: &'a ComposerInteractionModel,
    pub(crate) interaction_pane: &'a ComposerInteractionPaneState,
    pub(crate) mode: ComposerMode,
    pub(crate) caret_visibility: CaretVisibility,
    pub(crate) dispatch: &'a UiDispatch,
}

pub(crate) fn draw_composer_panel(
    scene: &mut UiScene,
    interaction_frame: &mut InteractionFrame,
    layout: ComposerPanelLayout,
    view: ComposerPanelView<'_>,
    text_layout: &mut TextInputLayoutEngine,
    palette: ShellPalette,
) -> Option<Rect> {
    scene.with_element(
        Element::leaf("ComposerPanel").in_bounds(layout.panel),
        |scene, _| {
            scene.draw_rect(
                PaintRect::new(layout.panel, palette.surface)
                    .with_border(Border::new(Edges::new(1.0, 0.0, 0.0, 0.0), palette.border)),
            );
            interaction_frame.register(
                UiNode::new(
                    COMPOSER_PANEL,
                    layout.panel,
                    AccessibilityRole::Group,
                    "Command composer",
                )
                .with_parent(MAIN_SURFACE),
            );
            if let (Some(bounds), Some(interaction)) = (layout.interaction, view.interaction.view())
            {
                draw_interaction(
                    scene,
                    interaction_frame,
                    bounds,
                    interaction,
                    view.interaction_pane.scroll_state(),
                    view.dispatch,
                    palette,
                );
            }
            draw_info_bar(
                scene,
                interaction_frame,
                layout.info_bar,
                view.mode,
                palette,
            );
            scene.draw_rect(PaintRect::new(layout.info_editor_separator, palette.border));
            interaction_frame.register(
                UiNode::new(
                    COMPOSER,
                    layout.editor,
                    AccessibilityRole::TextInput,
                    "Command input",
                )
                .with_parent(COMPOSER_PANEL)
                .with_cursor(CursorFeedback::Text)
                .with_focus(FocusBehavior::TabStop)
                .with_value(view.editor.text()),
            );
            let editor_focus = if view.dispatch.is_focused(COMPOSER) {
                ComposerEditorFocus::Focused(view.caret_visibility)
            } else {
                ComposerEditorFocus::Blurred
            };
            let placeholder = match view.mode {
                ComposerMode::Agent => "Ask Zeta anything…",
                ComposerMode::Shell => "Enter a shell command…",
            };
            let editor =
                view.editor
                    .view(layout.editor, placeholder, editor_focus, palette.text_muted);
            let caret_bounds = editor.caret_bounds();
            scene.draw_component(&editor);
            let toolbar = InputContextToolbar::new(
                layout.toolbar,
                view.context,
                view.mode,
                palette,
                text_layout,
                view.dispatch,
            );
            toolbar.register_interactions(interaction_frame);
            scene.draw_component(&toolbar);
            caret_bounds
        },
    )
}

fn draw_info_bar(
    scene: &mut UiScene,
    frame: &mut InteractionFrame,
    bounds: Rect,
    mode: ComposerMode,
    palette: ShellPalette,
) {
    let (accessibility_label, keycaps, label) = match mode {
        ComposerMode::Agent => ("/ for commands", vec![vec!["/".to_owned()]], "for commands"),
        ComposerMode::Shell => (
            "Up and Down for command history",
            vec![vec!["↑".to_owned(), "↓".to_owned()]],
            "for command history",
        ),
    };
    frame.register(
        UiNode::new(
            COMPOSER_INFO_BAR,
            bounds,
            AccessibilityRole::Group,
            accessibility_label,
        )
        .with_parent(COMPOSER_PANEL),
    );
    scene.with_element(
        Element::leaf("ComposerInfoBar").in_bounds(bounds),
        |scene, _| {
            let keycaps = KeycapSequence::new(
                Point::new(
                    bounds.origin.x,
                    bounds.origin.y + (bounds.size.height - INFO_KEYCAP_SIZE).max(0.0) * 0.5,
                ),
                keycaps,
                info_keycap_style(),
            );
            let label_x = keycaps.bounds().right() + INFO_KEYCAP_LABEL_GAP;
            scene.draw_component(&keycaps);
            scene.draw_text(TextBlock::new(
                label,
                Point::new(label_x, bounds.origin.y + 2.0),
                Size::new((bounds.right() - label_x).max(1.0), 20.0),
                TextStyle::new(12.0, palette.text_muted)
                    .with_family(FontFamily::Monospace)
                    .with_line_height(20.0),
            ));
        },
    );
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
    scene: &mut UiScene,
    frame: &mut InteractionFrame,
    bounds: Rect,
    view: ComposerInteractionView<'_>,
    scroll_state: ScrollState,
    dispatch: &UiDispatch,
    palette: ShellPalette,
) {
    scene.draw_rect(
        PaintRect::new(bounds, palette.surface_raised)
            .with_border(Border::uniform(1.0, palette.border))
            .with_corner_radii(CornerRadii::uniform(6.0)),
    );
    frame.register(
        UiNode::new(
            COMPOSER_INTERACTION,
            bounds,
            AccessibilityRole::List,
            view.title(),
        )
        .with_parent(COMPOSER_PANEL),
    );
    let title = if view.can_go_back() {
        format!("← {}", view.title())
    } else {
        view.title().to_owned()
    };
    scene.draw_text(TextBlock::new(
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
        scene.draw_text(TextBlock::new(
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
        frame.register(
            UiNode::new(
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
    list.draw(scene, |scene, layout| {
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
}

pub(crate) fn interaction_preferred_height(view: Option<ComposerInteractionView<'_>>) -> f32 {
    let Some(view) = view else {
        return 0.0;
    };
    let rows = view.items().len().clamp(1, MAX_VISIBLE_INTERACTION_ROWS);
    INTERACTION_HEADER_HEIGHT + rows as f32 * INTERACTION_ROW_HEIGHT
}

pub(crate) fn interaction_list_bounds(bounds: Rect) -> Rect {
    Rect::from_xywh(
        bounds.origin.x + 1.0,
        bounds.origin.y + INTERACTION_HEADER_HEIGHT,
        (bounds.size.width - 2.0).max(1.0),
        (bounds.size.height - INTERACTION_HEADER_HEIGHT - 1.0).max(1.0),
    )
}

pub(crate) fn interaction_content_size(viewport: Rect, item_count: usize) -> Size {
    zeta_ui::VirtualListLayout::new(item_count, INTERACTION_ROW_HEIGHT)
        .content_size(viewport.size.width)
}

pub(crate) fn interaction_selection_scroll_command(
    index: usize,
    item_count: usize,
    content_width: f32,
) -> Option<zeta_ui::ScrollCommand> {
    zeta_ui::VirtualListLayout::new(item_count, INTERACTION_ROW_HEIGHT)
        .ensure_visible_command(index, content_width)
}

#[cfg(test)]
#[path = "composer_panel_tests.rs"]
mod tests;
