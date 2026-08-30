//! Hover summary for the directories attached to one Workbench tab.

use crate::AccessibilityExpansion;
use crate::AccessibilityRole;
use crate::Border;
use crate::BoxShadow;
use crate::Button;
use crate::ButtonBackgrounds;
use crate::ButtonState;
use crate::ButtonStyle;
use crate::Color;
use crate::Component;
use crate::ComponentContext;
use crate::ComponentElement;
use crate::ComputedElement;
use crate::ContextView;
use crate::ContextViewAnchorAxis;
use crate::ContextViewAnchorPosition;
use crate::ContextViewPlacement;
use crate::ContextViewStyle;
use crate::CornerRadii;
use crate::Edges;
use crate::Element;
use crate::FontWeight;
use crate::InteractionRegion;
use crate::PaintRect;
use crate::Point;
use crate::Rect;
use crate::ScrollAxis;
use crate::ScrollCommand;
use crate::ScrollState;
use crate::ScrollView;
use crate::ScrollViewStyle;
use crate::ScrollbarStyle;
use crate::Size;
use crate::TextBlock;
use crate::TextStyle;
use crate::UiScene;
use zui::ui::CursorFeedback;
use zui::ui::ElementId;
use zui::ui::FocusBehavior;
use zui::ui::NodeAction;
use zui::ui::UiDispatch;

use super::WorkbenchUiStyle;
use super::tab_mount::WorkbenchTab;

const WIDTH: f32 = 360.0;
const HEADER_HEIGHT: f32 = 44.0;
const ROW_HEIGHT: f32 = 24.0;
const BUTTON_HEIGHT: f32 = 28.0;
const SECTION_GAP: f32 = 8.0;
const COLLAPSED_ROOTS: usize = 3;
const MAX_EXPANDED_ROOTS: usize = 8;
const SCROLL_BUTTON_WIDTH: f32 = 32.0;
const BUTTON_GAP: f32 = 6.0;
const PREVIEW_SCOPE: u32 = 31;
const DISCLOSURE_SCOPE: u32 = 32;
const SCROLL_SCOPE: u32 = 33;
const SCROLL_BACKWARD_SCOPE: u32 = 34;
const SCROLL_FORWARD_SCOPE: u32 = 35;

pub(super) fn dirs_preview_id(tab: ElementId) -> ElementId {
    ElementId::scoped(PREVIEW_SCOPE, mounted_element_local_id(tab))
}

pub(super) fn dirs_preview_disclosure_id(tab: ElementId) -> ElementId {
    ElementId::scoped(DISCLOSURE_SCOPE, mounted_element_local_id(tab))
}

pub(super) fn dirs_preview_scroll_id(tab: ElementId) -> ElementId {
    ElementId::scoped(SCROLL_SCOPE, mounted_element_local_id(tab))
}

fn dirs_preview_scroll_backward_id(tab: ElementId) -> ElementId {
    ElementId::scoped(SCROLL_BACKWARD_SCOPE, mounted_element_local_id(tab))
}

pub(super) fn dirs_preview_scroll_forward_id(tab: ElementId) -> ElementId {
    ElementId::scoped(SCROLL_FORWARD_SCOPE, mounted_element_local_id(tab))
}

fn mounted_element_local_id(element: ElementId) -> u32 {
    (element.into_raw() & u64::from(u32::MAX)) as u32
}

pub(super) struct TabDirsPreview<'a> {
    viewport: Rect,
    anchor: Rect,
    tab: &'a WorkbenchTab<'a>,
    style: WorkbenchUiStyle,
    dispatch: &'a UiDispatch,
}

impl<'a> TabDirsPreview<'a> {
    pub(super) const fn new(
        viewport: Rect,
        anchor: Rect,
        tab: &'a WorkbenchTab<'a>,
        style: WorkbenchUiStyle,
        dispatch: &'a UiDispatch,
    ) -> Self {
        Self {
            viewport,
            anchor,
            tab,
            style,
            dispatch,
        }
    }

    fn button_state(&self, id: ElementId) -> ButtonState {
        if self.dispatch.is_pressed(id) {
            ButtonState::Pressed
        } else if self.dispatch.is_focused(id) {
            ButtonState::Focused
        } else if self.dispatch.is_hovered(id) {
            ButtonState::Hovered
        } else {
            ButtonState::Resting
        }
    }

    fn paint_header(&self, scene: &mut UiScene, bounds: Rect) {
        scene.draw_text(TextBlock::new(
            self.tab.name,
            bounds.origin,
            Size::new(bounds.size.width, 20.0),
            TextStyle::new(13.0, self.style.colors.hover_foreground)
                .with_weight(FontWeight::Bold)
                .with_line_height(20.0),
        ));
        scene.draw_text(TextBlock::new(
            self.tab.status.label(),
            Point::new(bounds.origin.x, bounds.origin.y + 22.0),
            Size::new(bounds.size.width, 18.0),
            TextStyle::new(11.0, preview_muted_foreground(&self.style)).with_line_height(18.0),
        ));
    }
}

impl Component for TabDirsPreview<'_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("TabDirsPreviewHost").in_bounds(self.anchor)
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        let disclosure_id = dirs_preview_disclosure_id(self.tab.id);
        let expanded = self.dispatch.is_expanded(disclosure_id);
        let root_count = self.tab.dirs.len().max(1);
        let visible_root_count = if expanded {
            root_count.min(MAX_EXPANDED_ROOTS)
        } else {
            root_count.min(COLLAPSED_ROOTS)
        };
        let has_disclosure = root_count > COLLAPSED_ROOTS;
        let button_height = if has_disclosure {
            SECTION_GAP + BUTTON_HEIGHT
        } else {
            0.0
        };
        let desired_height =
            HEADER_HEIGHT + SECTION_GAP + visible_root_count as f32 * ROW_HEIGHT + button_height;
        let preview = ContextView::new(
            self.viewport,
            self.anchor,
            Size::new(WIDTH, desired_height),
            ContextViewPlacement::new()
                .with_axis(ContextViewAnchorAxis::Horizontal)
                .with_position(ContextViewAnchorPosition::After)
                .with_gap(8.0)
                .with_viewport_margin(8.0),
            ContextViewStyle::new(self.style.colors.hover_background)
                .with_corner_radii(CornerRadii::uniform(6.0))
                .with_padding(Edges::uniform(12.0))
                .with_shadow(
                    BoxShadow::new(self.style.colors.hover_shadow)
                        .with_offset(Point::new(0.0, 4.0))
                        .with_blur_radius(12.0),
                ),
        );
        preview.draw_components(context, |context, content_bounds| {
            context.draw_component(
                &InteractionRegion::new(
                    "TabDirsPreview",
                    dirs_preview_id(self.tab.id),
                    preview.bounds(),
                    AccessibilityRole::Group,
                    format!("Directory summary for {}", self.tab.name),
                )
                .with_parent(self.tab.id),
            );
            context.scene_mut().draw_rect(
                PaintRect::new(preview.bounds(), Color::TRANSPARENT)
                    .with_border(Border::uniform(1.0, self.style.colors.hover_border))
                    .with_corner_radii(CornerRadii::uniform(6.0)),
            );
            self.paint_header(context.scene_mut(), content_bounds);
            let rows_top = content_bounds.origin.y + HEADER_HEIGHT + SECTION_GAP;
            let button_top = if has_disclosure {
                content_bounds.bottom() - BUTTON_HEIGHT
            } else {
                content_bounds.bottom()
            };
            let rows_bottom = if has_disclosure {
                button_top - SECTION_GAP
            } else {
                button_top
            };
            let rows_bounds = Rect::from_xywh(
                content_bounds.origin.x,
                rows_top,
                content_bounds.size.width,
                (rows_bottom - rows_top).max(0.0),
            );
            let scroll_id = dirs_preview_scroll_id(self.tab.id);
            context.draw_component(
                &InteractionRegion::new(
                    "TabDirsPreviewRoots",
                    scroll_id,
                    rows_bounds,
                    AccessibilityRole::List,
                    "Directories",
                )
                .with_parent(dirs_preview_id(self.tab.id)),
            );
            let content_size = Size::new(rows_bounds.size.width, root_count as f32 * ROW_HEIGHT);
            let mut scroll_state = ScrollState::default();
            let scroll_metrics = crate::ScrollMetrics::new(rows_bounds.size, content_size);
            let maximum_scroll = scroll_metrics.maximum_offset().y.ceil() as i32;
            scroll_state.apply(
                ScrollCommand::ToOffset(Point::new(
                    0.0,
                    self.dispatch.value(scroll_id).clamp(0, maximum_scroll) as f32,
                )),
                scroll_metrics,
                ScrollAxis::Vertical,
            );
            let scroll_view = ScrollView::new(
                rows_bounds,
                content_size,
                scroll_state,
                ScrollAxis::Vertical,
                ScrollViewStyle::new(
                    ScrollbarStyle::new(
                        preview_control_background(&self.style),
                        preview_muted_foreground(&self.style),
                    )
                    .with_thickness(6.0),
                ),
            );
            scroll_view.draw_components(context, |context, viewport| {
                let visible = viewport.visible_content_bounds();
                let first = (visible.origin.y / ROW_HEIGHT).floor().max(0.0) as usize;
                let last = ((visible.bottom() / ROW_HEIGHT).ceil() as usize).min(root_count);
                for index in first..last {
                    let label = self
                        .tab
                        .dirs
                        .get(index)
                        .map(|root| root.to_string_lossy())
                        .unwrap_or_else(|| self.tab.location.into());
                    let row_bounds = Rect::from_xywh(
                        viewport.content_origin().x,
                        viewport.content_origin().y + index as f32 * ROW_HEIGHT,
                        (rows_bounds.size.width - 10.0).max(0.0),
                        ROW_HEIGHT,
                    );
                    context.scene_mut().draw_text(TextBlock::new(
                        label,
                        Point::new(row_bounds.origin.x, row_bounds.origin.y + 3.0),
                        Size::new(row_bounds.size.width, 18.0),
                        TextStyle::new(12.0, self.style.colors.hover_foreground)
                            .with_line_height(18.0),
                    ));
                }
            });
            if has_disclosure {
                let scroll_needed = expanded && maximum_scroll > 0;
                let scroll_controls_width = if scroll_needed {
                    BUTTON_GAP + SCROLL_BUTTON_WIDTH * 2.0 + BUTTON_GAP
                } else {
                    0.0
                };
                let button_bounds = Rect::from_xywh(
                    content_bounds.origin.x,
                    button_top,
                    (content_bounds.size.width - scroll_controls_width).max(0.0),
                    BUTTON_HEIGHT,
                );
                let hidden_count = root_count.saturating_sub(COLLAPSED_ROOTS);
                let label = if expanded {
                    "Collapse".to_owned()
                } else {
                    format!("Show all ({hidden_count} more)")
                };
                context.draw_component(
                    &InteractionRegion::new(
                        "TabDirsPreviewDisclosure",
                        disclosure_id,
                        button_bounds,
                        AccessibilityRole::Button,
                        label.clone(),
                    )
                    .with_parent(dirs_preview_id(self.tab.id))
                    .with_cursor(CursorFeedback::Pointer)
                    .with_focus(FocusBehavior::TabStop)
                    .with_action(NodeAction::ToggleExpansion)
                    .with_expansion(if expanded {
                        AccessibilityExpansion::Expanded
                    } else {
                        AccessibilityExpansion::Collapsed
                    }),
                );
                context.scene_mut().draw_component(&Button::new(
                    button_bounds,
                    label,
                    self.button_state(disclosure_id),
                    ButtonStyle::new(
                        preview_button_backgrounds(&self.style),
                        TextStyle::new(12.0, self.style.colors.hover_foreground),
                    )
                    .with_corner_radii(CornerRadii::uniform(4.0))
                    .with_padding(Edges::new(6.0, 8.0, 6.0, 8.0)),
                ));
                if scroll_needed {
                    let current = self.dispatch.value(scroll_id).clamp(0, maximum_scroll);
                    let backward_id = dirs_preview_scroll_backward_id(self.tab.id);
                    let backward_bounds = Rect::from_xywh(
                        button_bounds.right() + BUTTON_GAP,
                        button_top,
                        SCROLL_BUTTON_WIDTH,
                        BUTTON_HEIGHT,
                    );
                    let backward_enabled = current > 0;
                    if backward_enabled {
                        context.draw_component(
                            &InteractionRegion::new(
                                "TabDirsPreviewScrollBackward",
                                backward_id,
                                backward_bounds,
                                AccessibilityRole::Button,
                                "Scroll directories up",
                            )
                            .with_parent(dirs_preview_id(self.tab.id))
                            .with_cursor(CursorFeedback::Pointer)
                            .with_focus(FocusBehavior::TabStop)
                            .with_action(NodeAction::AdjustValue {
                                target: scroll_id,
                                delta: -(ROW_HEIGHT as i32),
                                minimum: 0,
                                maximum: maximum_scroll,
                            }),
                        );
                    }
                    context.scene_mut().draw_component(&Button::new(
                        backward_bounds,
                        "↑",
                        if backward_enabled {
                            self.button_state(backward_id)
                        } else {
                            ButtonState::Disabled
                        },
                        preview_button_style(&self.style),
                    ));

                    let forward_id = dirs_preview_scroll_forward_id(self.tab.id);
                    let forward_bounds = Rect::from_xywh(
                        backward_bounds.right() + BUTTON_GAP,
                        button_top,
                        SCROLL_BUTTON_WIDTH,
                        BUTTON_HEIGHT,
                    );
                    let forward_enabled = current < maximum_scroll;
                    if forward_enabled {
                        context.draw_component(
                            &InteractionRegion::new(
                                "TabDirsPreviewScrollForward",
                                forward_id,
                                forward_bounds,
                                AccessibilityRole::Button,
                                "Scroll directories down",
                            )
                            .with_parent(dirs_preview_id(self.tab.id))
                            .with_cursor(CursorFeedback::Pointer)
                            .with_focus(FocusBehavior::TabStop)
                            .with_action(NodeAction::AdjustValue {
                                target: scroll_id,
                                delta: ROW_HEIGHT as i32,
                                minimum: 0,
                                maximum: maximum_scroll,
                            }),
                        );
                    }
                    context.scene_mut().draw_component(&Button::new(
                        forward_bounds,
                        "↓",
                        if forward_enabled {
                            self.button_state(forward_id)
                        } else {
                            ButtonState::Disabled
                        },
                        preview_button_style(&self.style),
                    ));
                }
            }
        });
    }

    fn paint(&self, _scene: &mut UiScene) {}
}

fn preview_button_style(style: &WorkbenchUiStyle) -> ButtonStyle {
    ButtonStyle::new(
        preview_button_backgrounds(style),
        TextStyle::new(12.0, style.colors.hover_foreground),
    )
    .with_corner_radii(CornerRadii::uniform(4.0))
    .with_padding(Edges::new(6.0, 8.0, 6.0, 8.0))
}

fn preview_button_backgrounds(style: &WorkbenchUiStyle) -> ButtonBackgrounds {
    ButtonBackgrounds::new(preview_control_background(style))
        .with_hovered(with_alpha(style.colors.hover_foreground, 28))
        .with_focused(with_alpha(style.colors.hover_foreground, 28))
        .with_pressed(with_alpha(style.colors.hover_foreground, 40))
}

fn preview_control_background(style: &WorkbenchUiStyle) -> Color {
    with_alpha(style.colors.hover_foreground, 16)
}

fn preview_muted_foreground(style: &WorkbenchUiStyle) -> Color {
    with_alpha(style.colors.hover_foreground, 184)
}

fn with_alpha(color: Color, alpha: u8) -> Color {
    let [red, green, blue, _] = color.components();
    Color::rgba(red, green, blue, alpha)
}
