use zeta_ui::{
    ButtonBackgrounds, ButtonState, ButtonStyle, Component, ComponentElement, ContextMenu,
    ContextMenuItem, ContextMenuSelection, ContextMenuStyle, ContextViewPlacement, CornerRadii,
    Edges, Element, Point, Rect, Size, TextStyle, UiScene,
};
use zeta_winit::{ElementState, Key, KeyEvent, MouseButton, NamedKey};
use zui::{
    AccessibilityRole, AccessibilitySelection, CursorFeedback, DispatchInvalidation,
    DispatchOutcome, ElementId, FocusBehavior, FocusDirection, InteractionFrame, NavigationAxis,
    NavigationGroupId, NodeAction, UiDispatch, UiNode,
};

use crate::NativeApp;
use crate::shell_interaction::{
    ACTIVE_SESSION_TAB, SESSION_CONTEXT_MENU, SessionContextMenuAction, WINDOW,
};
use crate::shell_style::ShellPalette;

const MENU_CONTENT_WIDTH: f32 = 160.0;
const MENU_ITEM_HEIGHT: f32 = 30.0;
const MENU_VIEWPORT_MARGIN: f32 = 6.0;
const MENU_ANCHOR_GAP: f32 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct OpenSessionContextMenu {
    target_session: ElementId,
    anchor: Rect,
    restore_focus: Option<ElementId>,
}

/// Product-owned transient state for the Session Tab context menu.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct SessionContextMenuState {
    open: Option<OpenSessionContextMenu>,
}

impl SessionContextMenuState {
    pub(crate) fn open(
        &mut self,
        target_session: ElementId,
        position: Point,
        restore_focus: Option<ElementId>,
    ) {
        self.open = Some(OpenSessionContextMenu {
            target_session,
            anchor: Rect::from_xywh(position.x, position.y, 1.0, 1.0),
            restore_focus,
        });
    }

    pub(crate) const fn is_open(self) -> bool {
        self.open.is_some()
    }

    pub(crate) fn dismiss(&mut self) -> Option<ElementId> {
        self.open.take().and_then(|open| open.restore_focus)
    }

    pub(crate) const fn target_session(self) -> Option<ElementId> {
        match self.open {
            Some(open) => Some(open.target_session),
            None => None,
        }
    }
}

/// Session-specific menu presentation composed from the shared ContextMenu.
pub(crate) struct SessionContextMenu {
    context_menu: ContextMenu,
}

impl SessionContextMenu {
    pub(crate) fn new(
        viewport: Rect,
        state: SessionContextMenuState,
        palette: ShellPalette,
        dispatch: &UiDispatch,
    ) -> Option<Self> {
        let open = state.open?;
        let resting_backgrounds = ButtonBackgrounds::new(zeta_ui::Color::TRANSPARENT);
        let selected_backgrounds = ButtonBackgrounds::new(palette.session_tab_highlight)
            .with_hovered(palette.session_tab_highlight)
            .with_focused(palette.session_tab_highlight)
            .with_pressed(palette.border);
        let button_style = ButtonStyle::new(
            resting_backgrounds,
            TextStyle::new(13.0, palette.text).with_line_height(18.0),
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
                palette.surface,
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
        Some(Self { context_menu })
    }

    pub(crate) fn register_interactions(&self, frame: &mut InteractionFrame) {
        frame.register(
            UiNode::new(
                SESSION_CONTEXT_MENU,
                self.context_menu.bounds(),
                AccessibilityRole::Menu,
                "Session actions",
            )
            .with_parent(WINDOW),
        );
        frame.set_modal_root(SESSION_CONTEXT_MENU);
        let navigation_group = NavigationGroupId::new(SESSION_CONTEXT_MENU);
        for (index, action) in SessionContextMenuAction::ALL.into_iter().enumerate() {
            let Some(bounds) = self
                .context_menu
                .interactive_item_bounds(index)
                .filter(|bounds| !bounds.is_empty())
            else {
                continue;
            };
            frame.register(
                UiNode::new(
                    action.element_id(),
                    bounds,
                    AccessibilityRole::MenuItem,
                    action.label(),
                )
                .with_parent(SESSION_CONTEXT_MENU)
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
    }

    #[cfg(test)]
    pub(crate) fn bounds(&self) -> Rect {
        self.context_menu.bounds()
    }

    #[cfg(test)]
    pub(crate) fn item_bounds(&self, index: usize) -> Option<Rect> {
        self.context_menu.item_bounds(index)
    }

    #[cfg(test)]
    pub(crate) fn selected_index(&self) -> Option<usize> {
        self.context_menu.selected_index()
    }
}

impl Component for SessionContextMenu {
    fn element(&self) -> ComponentElement {
        Element::leaf("SessionContextMenu").in_bounds(self.context_menu.bounds())
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_component(&self.context_menu);
    }
}

impl NativeApp {
    pub(super) fn route_session_context_menu_pointer_move(&mut self, point: Point) -> bool {
        if !self.session_context_menu.is_open() {
            return false;
        }
        let outcome =
            self.presentation
                .as_ref()
                .map_or_else(DispatchOutcome::default, |presentation| {
                    update_session_context_menu_pointer(
                        &mut self.ui_dispatch,
                        point,
                        &presentation.interaction_frame,
                    )
                });
        self.update_cursor();
        self.apply_dispatch_outcome(outcome);
        true
    }

    pub(super) fn route_session_context_menu_button(
        &mut self,
        state: ElementState,
        button: MouseButton,
    ) -> bool {
        if button == MouseButton::Right {
            return self.route_session_context_menu_secondary_button(state);
        }
        if button != MouseButton::Left || !self.session_context_menu.is_open() {
            return false;
        }
        let target = self
            .cursor_position
            .zip(self.presentation.as_ref())
            .and_then(|(point, presentation)| presentation.interaction_frame.target_at(point));
        match state {
            ElementState::Pressed
                if target.is_some_and(SessionContextMenuAction::is_menu_element) =>
            {
                self.primary_button_changed(state);
            }
            ElementState::Pressed => {
                self.dismiss_session_context_menu();
            }
            ElementState::Released => {
                self.primary_button_changed(state);
            }
        }
        true
    }

    pub(super) fn route_session_context_menu_keyboard(&mut self, event: &KeyEvent) -> bool {
        if !self.session_context_menu.is_open() {
            return false;
        }
        let Some(presentation) = self.presentation.as_ref() else {
            return true;
        };
        let frame = &presentation.interaction_frame;
        let outcome = match &event.logical_key {
            Key::Named(NamedKey::Escape) => {
                self.dismiss_session_context_menu();
                return true;
            }
            Key::Named(NamedKey::ArrowUp) => self.ui_dispatch.focus_within_group(
                frame,
                FocusDirection::Previous,
                NavigationAxis::Vertical,
            ),
            Key::Named(NamedKey::ArrowDown) => self.ui_dispatch.focus_within_group(
                frame,
                FocusDirection::Next,
                NavigationAxis::Vertical,
            ),
            Key::Named(NamedKey::Tab) => {
                let direction = if self.modifiers.shift_key() {
                    FocusDirection::Previous
                } else {
                    FocusDirection::Next
                };
                self.ui_dispatch
                    .focus_within_group(frame, direction, NavigationAxis::Vertical)
            }
            Key::Named(NamedKey::Enter) => self.ui_dispatch.activate_focused(frame),
            Key::Character(text) if text == " " => self.ui_dispatch.activate_focused(frame),
            _ => Default::default(),
        };
        self.apply_dispatch_outcome(outcome);
        true
    }

    pub(super) fn dismiss_session_context_menu(&mut self) -> bool {
        if !self.session_context_menu.is_open() {
            return false;
        }
        let restore_focus = self.session_context_menu.dismiss();
        self.rebuild_presentation();
        if let Some(restore_focus) = restore_focus {
            let focus_outcome = self
                .presentation
                .as_ref()
                .map(|presentation| {
                    self.ui_dispatch
                        .focus_element(&presentation.interaction_frame, restore_focus)
                })
                .unwrap_or_default();
            if focus_outcome.invalidation == DispatchInvalidation::Paint {
                self.rebuild_presentation();
            }
        }
        self.update_cursor();
        self.request_redraw();
        true
    }

    fn route_session_context_menu_secondary_button(&mut self, state: ElementState) -> bool {
        if state == ElementState::Released {
            return self.session_context_menu.is_open();
        }
        let Some((point, target)) = self
            .cursor_position
            .zip(self.presentation.as_ref())
            .map(|(point, presentation)| (point, presentation.interaction_frame.target_at(point)))
        else {
            return false;
        };
        if target != Some(ACTIVE_SESSION_TAB) {
            return self.dismiss_session_context_menu();
        }
        let restore_focus = self.ui_dispatch.focused();
        self.session_context_menu
            .open(ACTIVE_SESSION_TAB, point, restore_focus);
        self.rebuild_presentation();
        let focus_outcome = self
            .presentation
            .as_ref()
            .map(|presentation| {
                self.ui_dispatch.focus_element(
                    &presentation.interaction_frame,
                    SessionContextMenuAction::Pin.element_id(),
                )
            })
            .unwrap_or_default();
        if focus_outcome.invalidation == DispatchInvalidation::Paint {
            self.rebuild_presentation();
        }
        self.update_cursor();
        self.request_redraw();
        true
    }
}

fn update_session_context_menu_pointer(
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
    }
}

#[cfg(test)]
#[path = "session_context_menu_tests.rs"]
mod tests;
