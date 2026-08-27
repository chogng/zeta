use zeta_ui_components::ButtonBackgrounds;
use zeta_ui_components::ButtonState;
use zeta_ui_components::ButtonStyle;
use zeta_ui_components::ContextMenu;
use zeta_ui_components::ContextMenuItem;
use zeta_ui_components::ContextMenuSelection;
use zeta_ui_components::ContextMenuStyle;
use zeta_ui_components::ContextViewPlacement;
use zeta_ui_components::InteractionRegion;
use zui::ui::AccessibilityRole;
use zui::ui::AccessibilitySelection;
use zui::ui::Color;
use zui::ui::Component;
use zui::ui::ComponentContext;
use zui::ui::ComponentElement;
use zui::ui::ComputedElement;
use zui::ui::CornerRadii;
use zui::ui::CursorFeedback;
use zui::ui::DispatchInvalidation;
use zui::ui::DispatchOutcome;
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
use zui::ui::TextStyle;
use zui::ui::UiDispatch;
use zui::ui::UiNode;
use zui::ui::UiScene;

use crate::TabInputKey;

const TAB_CONTEXT_MENU_SCOPE: u32 = 22;
pub const TAB_CONTEXT_MENU: ElementId = ElementId::scoped(TAB_CONTEXT_MENU_SCOPE, 1);
const TAB_CONTEXT_MENU_PIN: ElementId = ElementId::scoped(TAB_CONTEXT_MENU_SCOPE, 2);
const TAB_CONTEXT_MENU_CLOSE: ElementId = ElementId::scoped(TAB_CONTEXT_MENU_SCOPE, 3);
const TAB_CONTEXT_MENU_MOVE_TO_NEW_GROUP: ElementId = ElementId::scoped(TAB_CONTEXT_MENU_SCOPE, 4);

const MENU_CONTENT_WIDTH: f32 = 184.0;
const MENU_ITEM_HEIGHT: f32 = 30.0;
const MENU_VIEWPORT_MARGIN: f32 = 6.0;
const MENU_ANCHOR_GAP: f32 = 2.0;

/// Generic action emitted by the Workbench tab context menu.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TabContextMenuAction {
    TogglePin,
    Close,
    MoveToNewGroup,
}

impl TabContextMenuAction {
    pub const ALL: [Self; 3] = [Self::TogglePin, Self::Close, Self::MoveToNewGroup];

    pub const fn element_id(self) -> ElementId {
        match self {
            Self::TogglePin => TAB_CONTEXT_MENU_PIN,
            Self::Close => TAB_CONTEXT_MENU_CLOSE,
            Self::MoveToNewGroup => TAB_CONTEXT_MENU_MOVE_TO_NEW_GROUP,
        }
    }

    pub const fn label(self, pinned: bool) -> &'static str {
        match self {
            Self::TogglePin if pinned => "Unpin",
            Self::TogglePin => "Pin",
            Self::Close => "Close",
            Self::MoveToNewGroup => "Move to new group",
        }
    }

    pub const fn from_element_id(id: ElementId) -> Option<Self> {
        match id {
            TAB_CONTEXT_MENU_PIN => Some(Self::TogglePin),
            TAB_CONTEXT_MENU_CLOSE => Some(Self::Close),
            TAB_CONTEXT_MENU_MOVE_TO_NEW_GROUP => Some(Self::MoveToNewGroup),
            _ => None,
        }
    }

    pub fn is_menu_element(id: ElementId) -> bool {
        id == TAB_CONTEXT_MENU || Self::from_element_id(id).is_some()
    }
}

/// Colors needed by the Workbench tab context menu.
#[derive(Clone, Copy)]
pub struct TabContextMenuStyle {
    surface: Color,
    border: Color,
    text: Color,
    selected: Color,
}

impl TabContextMenuStyle {
    pub const fn new(surface: Color, border: Color, text: Color, selected: Color) -> Self {
        Self {
            surface,
            border,
            text,
            selected,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct OpenTabContextMenu {
    target_tab: TabInputKey,
    anchor: Rect,
    restore_focus: Option<ElementId>,
    pinned: bool,
}

/// Transient state for the Workbench tab context menu.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TabContextMenuState {
    open: Option<OpenTabContextMenu>,
}

impl TabContextMenuState {
    pub fn open_pinned(
        &mut self,
        target_tab: TabInputKey,
        position: Point,
        restore_focus: Option<ElementId>,
    ) {
        self.open(target_tab, position, restore_focus, true);
    }

    pub fn open_unpinned(
        &mut self,
        target_tab: TabInputKey,
        position: Point,
        restore_focus: Option<ElementId>,
    ) {
        self.open(target_tab, position, restore_focus, false);
    }

    fn open(
        &mut self,
        target_tab: TabInputKey,
        position: Point,
        restore_focus: Option<ElementId>,
        pinned: bool,
    ) {
        self.open = Some(OpenTabContextMenu {
            target_tab,
            anchor: Rect::from_xywh(position.x, position.y, 1.0, 1.0),
            restore_focus,
            pinned,
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

    pub fn target_is_pinned(&self) -> bool {
        self.open.as_ref().is_some_and(|open| open.pinned)
    }
}

/// Workbench-owned context menu for generic tab actions.
pub struct TabContextMenu {
    context_menu: ContextMenu,
    parent: ElementId,
    pinned: bool,
}

impl TabContextMenu {
    pub fn new(
        viewport: Rect,
        state: &TabContextMenuState,
        style: TabContextMenuStyle,
        parent: ElementId,
        dispatch: &UiDispatch,
    ) -> Option<Self> {
        let open = state.open.as_ref()?;
        let resting_backgrounds = ButtonBackgrounds::new(Color::TRANSPARENT);
        let selected_backgrounds = ButtonBackgrounds::new(style.selected)
            .with_hovered(style.selected)
            .with_focused(style.selected)
            .with_pressed(style.border);
        let button_style = ButtonStyle::new(
            resting_backgrounds,
            TextStyle::new(13.0, style.text).with_line_height(18.0),
        )
        .with_selected_backgrounds(selected_backgrounds)
        .with_corner_radii(CornerRadii::uniform(2.0))
        .with_padding(Edges::new(0.0, 10.0, 0.0, 10.0));
        let items = TabContextMenuAction::ALL
            .into_iter()
            .map(|action| {
                let state = if dispatch.is_pressed(action.element_id()) {
                    ButtonState::Pressed
                } else if dispatch.is_focused(action.element_id()) {
                    ButtonState::Focused
                } else if dispatch.is_hovered(action.element_id()) {
                    ButtonState::Hovered
                } else {
                    ButtonState::Resting
                };
                ContextMenuItem::new(action.label(open.pinned), state)
            })
            .collect();
        let selection = TabContextMenuAction::ALL
            .into_iter()
            .position(|action| dispatch.is_pressed(action.element_id()))
            .or_else(|| {
                TabContextMenuAction::ALL
                    .into_iter()
                    .position(|action| dispatch.is_hovered(action.element_id()))
            })
            .or_else(|| {
                TabContextMenuAction::ALL
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
            pinned: open.pinned,
        })
    }

    fn child_interaction_regions(&self) -> Vec<InteractionRegion> {
        let navigation_group = NavigationGroupId::new(TAB_CONTEXT_MENU);
        TabContextMenuAction::ALL
            .into_iter()
            .enumerate()
            .filter_map(|(index, action)| {
                let bounds = self
                    .context_menu
                    .interactive_item_bounds(index)
                    .filter(|bounds| !bounds.is_empty())?;
                Some(
                    InteractionRegion::new(
                        "TabContextMenuItem",
                        action.element_id(),
                        bounds,
                        AccessibilityRole::MenuItem,
                        action.label(self.pinned),
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
                )
            })
            .collect()
    }

    pub fn bounds(&self) -> Rect {
        self.context_menu.bounds()
    }

    pub fn item_bounds(&self, index: usize) -> Option<Rect> {
        self.context_menu.item_bounds(index)
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.context_menu.selected_index()
    }
}

impl Component for TabContextMenu {
    fn element(&self) -> ComponentElement {
        Element::leaf("TabContextMenu")
            .in_bounds(self.context_menu.bounds())
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
            .with_parent(self.parent),
        )
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        context.set_modal_root(TAB_CONTEXT_MENU);
        for region in self.child_interaction_regions() {
            context.draw_component(&region);
        }
        context.draw_component(&self.context_menu);
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_component(&self.context_menu);
    }
}

pub fn update_tab_context_menu_pointer(
    dispatch: &mut UiDispatch,
    point: Point,
    frame: &InteractionFrame,
) -> DispatchOutcome {
    let pointer_outcome = dispatch.pointer_moved(point, frame);
    let focus_outcome = frame
        .target_at(point)
        .filter(|target| TabContextMenuAction::is_menu_element(*target))
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
#[path = "tab_context_menu_tests.rs"]
mod tests;
