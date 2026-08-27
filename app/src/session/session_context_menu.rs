//! Host event adapter for the Session UI context menu.

use crate::NativeApp;
use crate::shell_interaction::{SessionContextMenuAction, WINDOW, session_tab_index};
use crate::shell_style::ShellPalette;
use zui::input::ElementState;
use zui::input::Key;
use zui::input::KeyEvent;
use zui::input::MouseButton;
use zui::input::NamedKey;
use zui::ui::Component;
use zui::ui::ComponentContext;
use zui::ui::ComponentElement;
use zui::ui::ComputedElement;
use zui::ui::DispatchInvalidation;
use zui::ui::DispatchOutcome;
use zui::ui::FocusDirection;
use zui::ui::InteractionFrame;
use zui::ui::NavigationAxis;
use zui::ui::Point;
use zui::ui::UiDispatch;
use zui::ui::UiNode;

pub(crate) use zeta_session_ui::SessionContextMenuState;

/// Adapts the product palette and host parent to the reusable Session UI menu.
pub(crate) struct SessionContextMenu {
    inner: zeta_session_ui::SessionContextMenu,
}

impl SessionContextMenu {
    pub(crate) fn new(
        viewport: zui::ui::Rect,
        state: &SessionContextMenuState,
        palette: ShellPalette,
        dispatch: &UiDispatch,
    ) -> Option<Self> {
        let style = zeta_session_ui::SessionContextMenuStyle::new(
            palette.surface,
            palette.border,
            palette.text,
            palette.session_tab_highlight,
        );
        Some(Self {
            inner: zeta_session_ui::SessionContextMenu::new(
                viewport, state, style, WINDOW, dispatch,
            )?,
        })
    }
}

impl Component for SessionContextMenu {
    fn element(&self) -> ComponentElement {
        self.inner.element()
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        self.inner.interaction_node(element)
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, element: &ComputedElement) {
        self.inner.compose(context, element)
    }
}

impl NativeApp {
    pub(crate) fn route_session_context_menu_pointer_move(&mut self, point: Point) -> bool {
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
                        presentation.interaction_frame(),
                    )
                });
        self.update_cursor();
        self.apply_dispatch_outcome(outcome);
        true
    }

    pub(crate) fn route_session_context_menu_button(
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
            .and_then(|(point, presentation)| presentation.interaction_frame().target_at(point));
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

    pub(crate) fn route_session_context_menu_keyboard(&mut self, event: &KeyEvent) -> bool {
        if !self.session_context_menu.is_open() {
            return false;
        }
        let Some(presentation) = self.presentation.as_ref() else {
            return true;
        };
        let frame = presentation.interaction_frame();
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

    pub(crate) fn dismiss_session_context_menu(&mut self) -> bool {
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
                        .focus_element(presentation.interaction_frame(), restore_focus)
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
        let Some((point, target)) =
            self.cursor_position
                .zip(self.presentation.as_ref())
                .map(|(point, presentation)| {
                    (point, presentation.interaction_frame().target_at(point))
                })
        else {
            return false;
        };
        let Some(index) = target.and_then(|target| {
            session_tab_index(
                target,
                0..self.workbench.workbench().tab_part().session_count(),
            )
        }) else {
            return self.dismiss_session_context_menu();
        };
        let Some(target_tab) = self
            .workbench
            .workbench()
            .tab_part()
            .session_input_at(index)
            .map(|input| input.key().clone())
        else {
            return self.dismiss_session_context_menu();
        };
        let restore_focus = self.ui_dispatch.focused();
        self.session_context_menu
            .open(target_tab, point, restore_focus);
        self.rebuild_presentation();
        let focus_outcome = self
            .presentation
            .as_ref()
            .map(|presentation| {
                self.ui_dispatch.focus_element(
                    presentation.interaction_frame(),
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
    zeta_session_ui::update_session_context_menu_pointer(dispatch, point, frame)
}
