//! Session tab context-menu state and presentation.

use zeta_ui::{
    ButtonBackgrounds, ButtonState, ButtonStyle, Component, ComponentContext, ComponentElement,
    ComputedElement, ContextMenu, ContextMenuItem, ContextMenuSelection, ContextMenuStyle,
    ContextViewPlacement, CornerRadii, Edges, Element, InteractionRegion, Point, Rect, Size,
    TextStyle, UiScene,
};
use zui::ui::{
    AccessibilityRole, AccessibilitySelection, CursorFeedback, DispatchInvalidation,
    DispatchOutcome, ElementId, FocusBehavior, InteractionFrame, NavigationAxis, NavigationGroupId,
    NodeAction, UiDispatch, UiNode,
};

use crate::interaction::{SESSION_CONTEXT_MENU, SessionContextMenuAction};
use zeta_workbench::TabInputKey;

const MENU_CONTENT_WIDTH: f32 = 160.0;
const MENU_ITEM_HEIGHT: f32 = 30.0;
const MENU_VIEWPORT_MARGIN: f32 = 6.0;
const MENU_ANCHOR_GAP: f32 = 2.0;

/// Colors needed by the Session context-menu renderer.
#[derive(Clone, Copy)]
pub struct SessionContextMenuStyle {
    pub surface: zeta_ui::Color,
    pub border: zeta_ui::Color,
    pub text: zeta_ui::Color,
    pub session_tab_highlight: zeta_ui::Color,
}

impl SessionContextMenuStyle {
    /// Creates the resolved colors used by the menu.
    pub const fn new(
        surface: zeta_ui::Color,
        border: zeta_ui::Color,
        text: zeta_ui::Color,
        session_tab_highlight: zeta_ui::Color,
    ) -> Self {
        Self {
            surface,
            border,
            text,
            session_tab_highlight,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct OpenSessionContextMenu {
    target_tab: TabInputKey,
    anchor: Rect,
    restore_focus: Option<ElementId>,
}

/// Transient state for the Session tab context menu.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SessionContextMenuState {
    open: Option<OpenSessionContextMenu>,
}

impl SessionContextMenuState {
    pub fn open(
        &mut self,
        target_tab: TabInputKey,
        position: Point,
        restore_focus: Option<ElementId>,
    ) {
        self.open = Some(OpenSessionContextMenu {
            target_tab,
            anchor: Rect::from_xywh(position.x, position.y, 1.0, 1.0),
            restore_focus,
        });
    }

    pub const fn is_open(&self) -> bool {
        self.open.is_some()
    }

    pub fn dismiss(&mut self) -> Option<ElementId> {
        self.open.take().and_then(|open| open.restore_focus)
    }

    pub fn target_tab(&self) -> Option<&TabInputKey> {
        self.open.as_ref().map(|open| &open.target_tab)
    }
}

/// Session-specific menu presentation composed from the shared ContextMenu.
pub struct SessionContextMenu {
    context_menu: ContextMenu,
    parent: ElementId,
}

impl SessionContextMenu {
    pub fn new(
        viewport: Rect,
        state: &SessionContextMenuState,
        style: SessionContextMenuStyle,
        parent: ElementId,
        dispatch: &UiDispatch,
    ) -> Option<Self> {
        let open = state.open.as_ref()?;
        let resting_backgrounds = ButtonBackgrounds::new(zeta_ui::Color::TRANSPARENT);
        let selected_backgrounds = ButtonBackgrounds::new(style.session_tab_highlight)
            .with_hovered(style.session_tab_highlight)
            .with_focused(style.session_tab_highlight)
            .with_pressed(style.border);
        let button_style = ButtonStyle::new(
            resting_backgrounds,
            TextStyle::new(13.0, style.text).with_line_height(18.0),
        )
        .with_selected_backgrounds(selected_backgrounds)
        .with_corner_radii(CornerRadii::uniform(2.0))
        .with_padding(Edges::new(0.0, 10.0, 0.0, 10.0));
        let items = SessionContextMenuAction::ALL
            .into_iter()
            .map(|action| {
                let id = action.element_id();
                let state = if dispatch.is_pressed(id) {
                    ButtonState::Pressed
                } else if dispatch.is_focused(id) {
                    ButtonState::Focused
                } else if dispatch.is_hovered(id) {
                    ButtonState::Hovered
                } else {
                    ButtonState::Resting
                };
                ContextMenuItem::new(action.label(), state)
            })
            .collect();
        let selection = SessionContextMenuAction::ALL
            .into_iter()
            .position(|action| dispatch.is_pressed(action.element_id()))
            .or_else(|| {
                SessionContextMenuAction::ALL
                    .into_iter()
                    .position(|action| dispatch.is_hovered(action.element_id()))
            })
            .or_else(|| {
                SessionContextMenuAction::ALL
                    .into_iter()
                    .position(|action| dispatch.is_focused(action.element_id()))
            })
            .map(ContextMenuSelection::Item)
            .unwrap_or_default();
        let context_menu = ContextMenu::new(
            viewport,
            open.anchor,
            items,
            ContextMenuStyle::new(
                style.surface,
                button_style,
                Size::new(MENU_CONTENT_WIDTH, MENU_ITEM_HEIGHT),
            )
            .with_placement(
                ContextViewPlacement::new()
                    .with_gap(MENU_ANCHOR_GAP)
                    .with_viewport_margin(MENU_VIEWPORT_MARGIN),
            ),
        )
        .with_selection(selection);
        Some(Self {
            context_menu,
            parent,
        })
    }

    fn child_interaction_regions(&self) -> Vec<InteractionRegion> {
        let navigation_group = NavigationGroupId::new(SESSION_CONTEXT_MENU);
        let mut regions = Vec::new();
        for (index, action) in SessionContextMenuAction::ALL.into_iter().enumerate() {
            let Some(bounds) = self
                .context_menu
                .interactive_item_bounds(index)
                .filter(|bounds| !bounds.is_empty())
            else {
                continue;
            };
            regions.push(
                InteractionRegion::new(
                    "SessionContextMenuItem",
                    action.element_id(),
                    bounds,
                    AccessibilityRole::MenuItem,
                    action.label(),
                )
                .with_cursor(CursorFeedback::Pointer)
                .with_focus(FocusBehavior::TabStop)
                .with_action(NodeAction::Activate)
                .with_navigation(navigation_group, NavigationAxis::Vertical)
                .with_selection(
                    if self.context_menu.selected_index() == Some(index) {
                        AccessibilitySelection::Selected
                    } else {
                        AccessibilitySelection::Unselected
                    },
                ),
            );
        }
        regions
    }

    #[cfg(test)]
    pub fn bounds(&self) -> Rect {
        self.context_menu.bounds()
    }

    #[cfg(test)]
    pub fn item_bounds(&self, index: usize) -> Option<Rect> {
        self.context_menu.item_bounds(index)
    }

    #[cfg(test)]
    pub fn selected_index(&self) -> Option<usize> {
        self.context_menu.selected_index()
    }
}

impl Component for SessionContextMenu {
    fn element(&self) -> ComponentElement {
        Element::leaf("SessionContextMenu")
            .in_bounds(self.context_menu.bounds())
            .with_identity(SESSION_CONTEXT_MENU)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        Some(
            UiNode::new(
                SESSION_CONTEXT_MENU,
                element.bounds(),
                AccessibilityRole::Menu,
                "Session actions",
            )
            .with_parent(self.parent),
        )
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        context.set_modal_root(SESSION_CONTEXT_MENU);
        for region in self.child_interaction_regions() {
            context.draw_component(&region);
        }
        context.draw_component(&self.context_menu);
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_component(&self.context_menu);
    }
}

pub fn update_session_context_menu_pointer(
    dispatch: &mut UiDispatch,
    point: Point,
    frame: &InteractionFrame,
) -> DispatchOutcome {
    let pointer_outcome = dispatch.pointer_moved(point, frame);
    let focus_outcome = frame
        .target_at(point)
        .filter(|target| SessionContextMenuAction::is_menu_element(*target))
        .map(|target| dispatch.focus_element(frame, target))
        .unwrap_or_default();
    DispatchOutcome {
        invalidation: if pointer_outcome.invalidation == DispatchInvalidation::Paint
            || focus_outcome.invalidation == DispatchInvalidation::Paint
        {
            DispatchInvalidation::Paint
        } else {
            DispatchInvalidation::None
        },
        intent: None,
        fragment: None,
    }
}

#[cfg(test)]
#[path = "session_context_menu_tests.rs"]
mod tests;
