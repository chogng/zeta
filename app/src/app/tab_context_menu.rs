//! Desktop input adapter for the Workbench-owned tab context menu.

use crate::ProductApp;
use crate::terminal_input::text_input_command;
use zeta_workbench::TabContextMenuAction;
use zui::input::ElementState;
use zui::input::Key;
use zui::input::KeyEvent;
use zui::input::MouseButton;
use zui::input::NamedKey;
use zui::ui::DispatchInvalidation;
use zui::ui::DispatchOutcome;
use zui::ui::ElementId;
use zui::ui::FocusDirection;
use zui::ui::InteractionFrame;
use zui::ui::NavigationAxis;
use zui::ui::Point;
use zui::ui::UiDispatch;

impl ProductApp {
    pub(crate) fn route_tab_context_menu_pointer_move(&mut self, point: Point) -> bool {
        if !self.workbench.tab_context_menu().is_open() {
            return false;
        }
        let outcome =
            self.presentation
                .as_ref()
                .map_or_else(DispatchOutcome::default, |presentation| {
                    update_tab_context_menu_pointer(
                        &mut self.ui_dispatch,
                        point,
                        presentation.interaction_frame(),
                    )
                });
        self.update_cursor();
        self.apply_dispatch_outcome(outcome);
        true
    }

    pub(crate) fn route_tab_context_menu_button(
        &mut self,
        state: ElementState,
        button: MouseButton,
    ) -> bool {
        if button == MouseButton::Right {
            return self.route_tab_context_menu_secondary_button(state);
        }
        if button != MouseButton::Left || !self.workbench.tab_context_menu().is_open() {
            return false;
        }
        let target = self
            .cursor_position
            .zip(self.presentation.as_ref())
            .and_then(|(point, presentation)| presentation.interaction_frame().target_at(point));
        match state {
            ElementState::Pressed if target.is_some_and(TabContextMenuAction::is_menu_element) => {
                self.primary_button_changed(state);
            }
            ElementState::Pressed => {
                self.dismiss_tab_context_menu();
            }
            ElementState::Released => self.primary_button_changed(state),
        }
        true
    }

    pub(crate) fn route_tab_context_menu_keyboard(&mut self, event: &KeyEvent) -> bool {
        if !self.workbench.tab_context_menu().is_open() {
            return false;
        }
        if self.workbench.tab_context_menu().is_renaming() {
            match event.logical_key {
                Key::Named(NamedKey::Escape) => {
                    self.dismiss_tab_context_menu();
                }
                Key::Named(NamedKey::Enter) => {
                    if self.workbench.commit_tab_rename() {
                        self.rebuild_presentation();
                        self.request_redraw();
                    }
                }
                _ => {
                    if let Some(command) = text_input_command(event, self.modifiers)
                        && self.workbench.apply_tab_rename(command)
                    {
                        self.caret_blink.activity(std::time::Instant::now());
                        self.rebuild_presentation();
                        self.request_redraw();
                    }
                }
            }
            return true;
        }
        let Some(presentation) = self.presentation.as_ref() else {
            return true;
        };
        let frame = presentation.interaction_frame();
        let outcome = match &event.logical_key {
            Key::Named(NamedKey::Escape) => {
                self.dismiss_tab_context_menu();
                return true;
            }
            Key::Named(NamedKey::ArrowUp) => self.ui_dispatch.focus_within_group(
                frame,
                FocusDirection::Previous,
                NavigationAxis::Vertical,
            ),
            Key::Named(NamedKey::ArrowDown) | Key::Named(NamedKey::Tab) => self
                .ui_dispatch
                .focus_within_group(frame, FocusDirection::Next, NavigationAxis::Vertical),
            Key::Named(NamedKey::Enter) | Key::Named(NamedKey::ArrowRight) => {
                self.ui_dispatch.activate_focused(frame)
            }
            Key::Character(text) if text == " " => self.ui_dispatch.activate_focused(frame),
            _ => Default::default(),
        };
        self.apply_dispatch_outcome(outcome);
        true
    }

    pub(crate) fn open_tab_context_menu(
        &mut self,
        tab: zeta_workbench::TabInputKey,
        position: Point,
    ) -> bool {
        let restore_focus = self.ui_dispatch.focused();
        if !self
            .workbench
            .open_tab_context_menu(tab.clone(), position, restore_focus)
        {
            return false;
        }
        self.rebuild_presentation();
        self.focus_tab_context_menu_element(TabContextMenuAction::TogglePin.element_id());
        self.update_cursor();
        self.request_redraw();
        true
    }

    pub(crate) fn activate_tab_context_menu_element(&mut self, id: ElementId) -> bool {
        if !self.workbench.tab_context_menu().is_open()
            || !TabContextMenuAction::is_menu_element(id)
        {
            return false;
        }
        match self.workbench.activate_tab_context_menu(id) {
            zeta_workbench::TabContextMenuOutcome::Ignored => {}
            zeta_workbench::TabContextMenuOutcome::Changed => {
                self.rebuild_presentation();
                self.request_redraw();
            }
            zeta_workbench::TabContextMenuOutcome::Close(tab) => {
                let _ = self.close_workbench_tab(&tab);
            }
            zeta_workbench::TabContextMenuOutcome::Focus(element) => {
                self.rebuild_presentation();
                self.focus_tab_context_menu_element(element);
                self.request_redraw();
            }
        }
        true
    }

    pub(crate) fn dismiss_tab_context_menu(&mut self) -> bool {
        if !self.workbench.tab_context_menu().is_open() {
            return false;
        }
        let restore_focus = self.workbench.dismiss_tab_context_menu();
        self.rebuild_presentation();
        if let Some(restore_focus) = restore_focus {
            self.focus_tab_context_menu_element(restore_focus);
        }
        self.update_cursor();
        self.request_redraw();
        true
    }

    fn route_tab_context_menu_secondary_button(&mut self, state: ElementState) -> bool {
        if state == ElementState::Released {
            return self.workbench.tab_context_menu().is_open();
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
        let target_tab = target
            .and_then(|target| {
                zeta_workbench::tab_key_for_element(self.workbench.workbench().tab_part(), target)
            })
            .cloned();
        match target_tab {
            Some(tab) => self.open_tab_context_menu(tab, point),
            None => self.dismiss_tab_context_menu(),
        }
    }

    fn focus_tab_context_menu_element(&mut self, id: ElementId) {
        let outcome = self
            .presentation
            .as_ref()
            .map(|presentation| {
                self.ui_dispatch
                    .focus_element(presentation.interaction_frame(), id)
            })
            .unwrap_or_default();
        if outcome.invalidation == DispatchInvalidation::Paint {
            self.rebuild_presentation();
        }
    }
}

fn update_tab_context_menu_pointer(
    dispatch: &mut UiDispatch,
    point: Point,
    frame: &InteractionFrame,
) -> DispatchOutcome {
    zeta_workbench::update_tab_context_menu_pointer(dispatch, point, frame)
}
