use zeta_ui_components::ButtonBackgrounds;
use zeta_ui_components::ButtonState;
use zeta_ui_components::ButtonStyle;
use zeta_ui_components::ContextMenu;
use zeta_ui_components::ContextMenuItem;
use zeta_ui_components::ContextMenuSelection;
use zeta_ui_components::ContextMenuStyle as SharedContextMenuStyle;
use zeta_ui_components::ContextViewAnchorAxis;
use zeta_ui_components::ContextViewAnchorPosition;
use zeta_ui_components::ContextViewPlacement;
use zeta_ui_components::InputBox;
use zeta_ui_components::InputBoxState;
use zeta_ui_components::InputBoxStateColors;
use zeta_ui_components::InputBoxStyle;
use zeta_ui_components::InteractionRegion;
use zui::ui::AccessibilityRole;
use zui::ui::CaretVisibility;
use zui::ui::Color;
use zui::ui::Component;
use zui::ui::ComponentContext;
use zui::ui::ComponentElement;
use zui::ui::ComputedElement;
use zui::ui::CornerRadii;
use zui::ui::CursorFeedback;
use zui::ui::Edges;
use zui::ui::Element;
use zui::ui::ElementId;
use zui::ui::FocusBehavior;
use zui::ui::InteractionFrame;
use zui::ui::NavigationAxis;
use zui::ui::NavigationGroupId;
use zui::ui::NodeAction;
use zui::ui::Point;
use zui::ui::Rect;
use zui::ui::Size;
use zui::ui::TextInputLayoutEngine;
use zui::ui::TextStyle;
use zui::ui::UiDispatch;
use zui::ui::UiNode;

use super::TAB_CONTEXT_MENU;
use super::TAB_CONTEXT_MENU_GROUPS;
use super::TAB_CONTEXT_MENU_MOVE_TO_NEW_GROUP;
use super::TAB_RENAME_INPUT;
use super::TabContextMenuAction;
use super::TabContextMenuState;
use super::TabContextMenuView;
use super::tab_group_menu_element_id;
use crate::TabPart;

const MENU_WIDTH: f32 = 190.0;
const MENU_ITEM_HEIGHT: f32 = 30.0;
const MENU_GAP: f32 = 2.0;
const MENU_MARGIN: f32 = 6.0;
const RENAME_HEIGHT: f32 = 38.0;
const RENAME_INSET: f32 = 4.0;

/// Colors used by the Workbench-owned tab actions menu.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TabContextMenuStyle {
    surface: Color,
    border: Color,
    text: Color,
    highlight: Color,
}

impl TabContextMenuStyle {
    pub const fn new(surface: Color, border: Color, text: Color, highlight: Color) -> Self {
        Self {
            surface,
            border,
            text,
            highlight,
        }
    }
}

/// Workbench presentation for a tab actions menu and its group/rename child view.
#[derive(Clone, Debug, PartialEq)]
pub struct TabContextMenu {
    root: ContextMenu,
    groups: Option<ContextMenu>,
    rename: Option<InputBox>,
    root_regions: Vec<InteractionRegion>,
    group_regions: Vec<InteractionRegion>,
    rename_value: Option<String>,
    window: ElementId,
    bounds: Rect,
}

impl TabContextMenu {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        viewport: Rect,
        tab_part: &TabPart,
        state: &TabContextMenuState,
        caret_visibility: CaretVisibility,
        style: TabContextMenuStyle,
        window: ElementId,
        text_layout: &mut TextInputLayoutEngine,
        dispatch: &UiDispatch,
    ) -> Option<Self> {
        let open = state.open.as_ref()?;
        let button_style = menu_button_style(style);
        let root_items = TabContextMenuAction::ALL
            .into_iter()
            .map(|action| {
                let id = action.element_id();
                ContextMenuItem::new(action.label(open.pinned), button_state(dispatch, id, true))
            })
            .collect();
        let root_selection = selected_action(dispatch)
            .map(ContextMenuSelection::Item)
            .unwrap_or(ContextMenuSelection::None);
        let root_style = SharedContextMenuStyle::new(
            style.surface,
            button_style.clone(),
            Size::new(MENU_WIDTH, MENU_ITEM_HEIGHT),
        )
        .with_placement(
            ContextViewPlacement::new()
                .with_gap(MENU_GAP)
                .with_viewport_margin(MENU_MARGIN),
        );
        let root_style = if open.view == TabContextMenuView::Rename {
            root_style.with_header_height(RENAME_HEIGHT)
        } else {
            root_style
        };
        let root = ContextMenu::new(viewport, open.anchor, root_items, root_style)
            .with_selection(root_selection);
        let root_regions = action_regions(&root, open.pinned);

        let groups = (open.view == TabContextMenuView::Groups).then(|| {
            let source = tab_part.input_group(&open.target_tab);
            let entries = tab_part
                .groups()
                .iter()
                .filter(|group| Some(group.id()) != source)
                .map(|group| {
                    (
                        tab_group_menu_element_id(group.id()),
                        group
                            .label()
                            .map(str::to_owned)
                            .unwrap_or_else(|| format!("Group {}", group.id().value())),
                    )
                })
                .chain(std::iter::once((
                    TAB_CONTEXT_MENU_MOVE_TO_NEW_GROUP,
                    "New group".to_owned(),
                )))
                .collect::<Vec<_>>();
            let items = entries
                .iter()
                .map(|(id, label)| ContextMenuItem::new(label, button_state(dispatch, *id, true)))
                .collect();
            let selection = entries
                .iter()
                .position(|(id, _)| is_active(dispatch, *id))
                .map(ContextMenuSelection::Item)
                .unwrap_or(ContextMenuSelection::None);
            let anchor = root.item_bounds(2).unwrap_or_else(|| {
                Rect::from_xywh(root.bounds().right(), root.bounds().origin.y, 1.0, 1.0)
            });
            let menu = ContextMenu::new(
                viewport,
                anchor,
                items,
                SharedContextMenuStyle::new(
                    style.surface,
                    button_style.clone(),
                    Size::new(MENU_WIDTH, MENU_ITEM_HEIGHT),
                )
                .with_placement(
                    ContextViewPlacement::new()
                        .with_axis(ContextViewAnchorAxis::Horizontal)
                        .with_position(ContextViewAnchorPosition::After)
                        .with_gap(MENU_GAP)
                        .with_viewport_margin(MENU_MARGIN),
                ),
            )
            .with_selection(selection);
            (menu, entries)
        });
        let (groups, group_regions) = match groups {
            Some((menu, entries)) => {
                let regions = group_action_regions(&menu, &entries);
                (Some(menu), regions)
            }
            None => (None, Vec::new()),
        };

        let rename = if open.view == TabContextMenuView::Rename {
            let header = root.header_bounds().expect("rename view reserves a header");
            let bounds = inset_rect(header, RENAME_INSET);
            let state = if dispatch.is_focused(TAB_RENAME_INPUT) {
                InputBoxState::Focused(caret_visibility)
            } else if dispatch.is_hovered(TAB_RENAME_INPUT) {
                InputBoxState::Hovered
            } else {
                InputBoxState::Resting
            };
            Some(InputBox::new(
                bounds,
                "Tab name",
                state,
                rename_style(style),
                &open.rename,
                text_layout,
            ))
        } else {
            None
        };
        let rename_value = rename.as_ref().map(|_| open.rename.text().to_owned());
        let bounds = groups
            .as_ref()
            .map(|groups| union_rect(root.bounds(), groups.bounds()))
            .unwrap_or_else(|| root.bounds());
        Some(Self {
            root,
            groups,
            rename,
            root_regions,
            group_regions,
            rename_value,
            window,
            bounds,
        })
    }

    pub fn item_bounds(&self, index: usize) -> Option<Rect> {
        self.root.item_bounds(index)
    }
}

impl Component for TabContextMenu {
    fn element(&self) -> ComponentElement {
        Element::leaf("TabContextMenu")
            .in_bounds(self.bounds)
            .with_identity(TAB_CONTEXT_MENU)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        Some(
            UiNode::new(
                TAB_CONTEXT_MENU,
                element.bounds(),
                AccessibilityRole::Menu,
                "Tab actions",
            )
            .with_parent(self.window),
        )
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        context.set_modal_root(TAB_CONTEXT_MENU);
        for region in &self.root_regions {
            context.draw_component(region);
        }
        for region in &self.group_regions {
            context.draw_component(region);
        }
        if let Some(rename) = &self.rename {
            context.draw_component(
                &InteractionRegion::new(
                    "TabRenameInput",
                    TAB_RENAME_INPUT,
                    rename.bounds(),
                    AccessibilityRole::TextInput,
                    "Rename tab",
                )
                .with_cursor(CursorFeedback::Text)
                .with_focus(FocusBehavior::TabStop)
                .with_value(self.rename_value.as_deref().unwrap_or_default()),
            );
        }
        self.root
            .draw_components_with_header(context, |context, _| {
                if let Some(rename) = &self.rename {
                    context.draw_component(rename);
                }
            });
        if let Some(groups) = &self.groups {
            context.draw_component(groups);
        }
    }
}

pub fn update_tab_context_menu_pointer(
    dispatch: &mut UiDispatch,
    point: Point,
    frame: &InteractionFrame,
) -> zui::ui::DispatchOutcome {
    dispatch.pointer_moved(point, frame)
}

fn action_regions(menu: &ContextMenu, pinned: bool) -> Vec<InteractionRegion> {
    TabContextMenuAction::ALL
        .into_iter()
        .enumerate()
        .filter_map(|(index, action)| {
            let bounds = menu.interactive_item_bounds(index)?;
            Some(menu_region(
                action.element_id(),
                bounds,
                action.label(pinned),
                TAB_CONTEXT_MENU,
            ))
        })
        .collect()
}

fn group_action_regions(
    menu: &ContextMenu,
    entries: &[(ElementId, String)],
) -> Vec<InteractionRegion> {
    entries
        .iter()
        .enumerate()
        .filter_map(|(index, (id, label))| {
            Some(menu_region(
                *id,
                menu.interactive_item_bounds(index)?,
                label,
                TAB_CONTEXT_MENU_GROUPS,
            ))
        })
        .collect()
}

fn menu_region(
    id: ElementId,
    bounds: Rect,
    label: impl Into<String>,
    group: ElementId,
) -> InteractionRegion {
    InteractionRegion::new(
        "TabContextMenuItem",
        id,
        bounds,
        AccessibilityRole::MenuItem,
        label,
    )
    .with_parent(TAB_CONTEXT_MENU)
    .with_cursor(CursorFeedback::Pointer)
    .with_focus(FocusBehavior::TabStop)
    .with_action(NodeAction::Activate)
    .with_navigation(NavigationGroupId::new(group), NavigationAxis::Vertical)
}

fn selected_action(dispatch: &UiDispatch) -> Option<usize> {
    TabContextMenuAction::ALL
        .into_iter()
        .position(|action| is_active(dispatch, action.element_id()))
}

fn button_state(dispatch: &UiDispatch, id: ElementId, enabled: bool) -> ButtonState {
    if !enabled {
        ButtonState::Disabled
    } else if dispatch.is_pressed(id) {
        ButtonState::Pressed
    } else if dispatch.is_focused(id) {
        ButtonState::Focused
    } else if dispatch.is_hovered(id) {
        ButtonState::Hovered
    } else {
        ButtonState::Resting
    }
}

fn is_active(dispatch: &UiDispatch, id: ElementId) -> bool {
    dispatch.is_pressed(id) || dispatch.is_focused(id) || dispatch.is_hovered(id)
}

fn menu_button_style(style: TabContextMenuStyle) -> ButtonStyle {
    let backgrounds = ButtonBackgrounds::new(Color::TRANSPARENT)
        .with_hovered(style.highlight)
        .with_focused(style.highlight)
        .with_pressed(style.highlight);
    ButtonStyle::new(backgrounds, TextStyle::new(12.0, style.text))
        .with_corner_radii(CornerRadii::uniform(3.0))
        .with_padding(Edges::new(6.0, 8.0, 6.0, 8.0))
}

fn rename_style(style: TabContextMenuStyle) -> InputBoxStyle {
    InputBoxStyle::new(
        InputBoxStateColors::new(style.surface, style.surface, style.surface),
        InputBoxStateColors::new(style.border, style.text, style.text),
        TextStyle::new(12.0, style.text),
        TextStyle::new(12.0, style.text),
    )
    .with_corner_radii(CornerRadii::uniform(3.0))
    .with_padding(Edges::new(4.0, 6.0, 4.0, 6.0))
}

fn inset_rect(bounds: Rect, inset: f32) -> Rect {
    Rect::from_xywh(
        bounds.origin.x + inset,
        bounds.origin.y + inset,
        (bounds.size.width - inset * 2.0).max(0.0),
        (bounds.size.height - inset * 2.0).max(0.0),
    )
}

fn union_rect(left: Rect, right: Rect) -> Rect {
    let x = left.origin.x.min(right.origin.x);
    let y = left.origin.y.min(right.origin.y);
    let right_edge = left.right().max(right.right());
    let bottom = (left.origin.y + left.size.height).max(right.origin.y + right.size.height);
    Rect::from_xywh(x, y, right_edge - x, bottom - y)
}
