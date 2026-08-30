use std::time::Instant;

use zeta_commands::AppCommandId;
use zeta_keybinding::HostPlatform;
use zeta_keybindings_host::recording_chord;
use zeta_ui_components::ScrollAxis;
use zeta_ui_components::ScrollCommand;
use zeta_ui_components::ScrollState;
use zeta_ui_components::ScrollbarController;
use zeta_ui_components::ScrollbarDrag;
use zeta_ui_components::ScrollbarPart;
use zeta_ui_components::ScrollbarPointerPresence;
use zeta_ui_components::ScrollbarPresentation;
use zui::input::ElementState;
use zui::input::Key;
use zui::input::KeyEvent;
use zui::input::ModifiersState;
use zui::input::NamedKey;
use zui::ui::ElementId;
use zui::ui::Point;
use zui::ui::TextInput;
use zui::ui::TextInputCommand;
use zui::ui::TextInputCompositionEvent;

use crate::KeyboardShortcutsState;
use crate::SETTINGS_NAV_APPEARANCE;
use crate::SETTINGS_NAV_BACK;
use crate::SETTINGS_NAV_GENERAL;
use crate::SETTINGS_NAV_KEYBINDINGS;
use crate::SETTINGS_NAV_REMOTE;
use crate::SettingsKeybindingsViewport;
use crate::SettingsPageSection;
use crate::SettingsScrollbarPointerOutcome;
use crate::keybindings::ShortcutCommit;
use crate::keybindings::command_for_keyboard_shortcut_row;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsActivation {
    Ignored,
    Changed,
    OpenRemote,
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum SettingsScrollbarCapture {
    Thumb(ScrollbarDrag),
    Track,
}

pub struct SettingsState {
    section: SettingsPageSection,
    search: TextInput,
    keyboard_shortcuts: KeyboardShortcutsState,
    keybindings_scroll: ScrollState,
    keybindings_scrollbar: ScrollbarController,
    keybindings_scrollbar_capture: Option<SettingsScrollbarCapture>,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            section: SettingsPageSection::default(),
            search: TextInput::default(),
            keyboard_shortcuts: KeyboardShortcutsState::default(),
            keybindings_scroll: ScrollState::default(),
            keybindings_scrollbar: ScrollbarController::default(),
            keybindings_scrollbar_capture: None,
        }
    }
}

impl SettingsState {
    pub const fn section(&self) -> SettingsPageSection {
        self.section
    }

    pub const fn search_input(&self) -> &TextInput {
        &self.search
    }

    pub const fn keybindings_scroll_state(&self) -> ScrollState {
        self.keybindings_scroll
    }

    pub fn keybindings_scrollbar_presentation(&self) -> ScrollbarPresentation {
        self.keybindings_scrollbar.presentation()
    }

    pub fn scroll_keybindings(
        &mut self,
        command: ScrollCommand,
        viewport: SettingsKeybindingsViewport,
        now: Instant,
    ) -> bool {
        let previous_presentation = self.keybindings_scrollbar.presentation();
        let previous_deadline = self.keybindings_scrollbar.next_deadline();
        let view = viewport.list(self.keybindings_scroll, previous_presentation);
        let changed = self.keybindings_scroll.apply(
            command,
            view.scroll_view().metrics(),
            ScrollAxis::Vertical,
        );
        self.keybindings_scrollbar.activity(now);
        changed
            || self.keybindings_scrollbar.presentation() != previous_presentation
            || self.keybindings_scrollbar.next_deadline() != previous_deadline
    }

    pub fn ensure_keybinding_visible(
        &mut self,
        element: ElementId,
        viewport: SettingsKeybindingsViewport,
        now: Instant,
    ) -> bool {
        let Some(command) = command_for_keyboard_shortcut_row(element) else {
            return false;
        };
        let Some(index) = AppCommandId::BINDABLE
            .into_iter()
            .position(|candidate| candidate == command)
        else {
            return false;
        };
        let list = viewport.list(
            self.keybindings_scroll,
            self.keybindings_scrollbar.presentation(),
        );
        let Some(command) = list.ensure_visible_command(index) else {
            return false;
        };
        self.scroll_keybindings(command, viewport, now)
    }

    pub fn keybindings_scrollbar_pointer_moved(
        &mut self,
        point: Point,
        viewport: SettingsKeybindingsViewport,
        now: Instant,
    ) -> SettingsScrollbarPointerOutcome {
        let previous_presentation = self.keybindings_scrollbar.presentation();
        match self.keybindings_scrollbar_capture {
            Some(SettingsScrollbarCapture::Thumb(drag)) => {
                let view = viewport.list(
                    self.keybindings_scroll,
                    self.keybindings_scrollbar.presentation(),
                );
                let offset_changed = self.keybindings_scroll.apply(
                    drag.command_at(point),
                    view.scroll_view().metrics(),
                    ScrollAxis::Vertical,
                );
                SettingsScrollbarPointerOutcome {
                    handled: true,
                    presentation_changed: offset_changed
                        || self.keybindings_scrollbar.presentation() != previous_presentation,
                }
            }
            Some(SettingsScrollbarCapture::Track) => SettingsScrollbarPointerOutcome {
                handled: true,
                presentation_changed: false,
            },
            None => {
                let presence = self.keybindings_scrollbar_presence(point, viewport);
                self.keybindings_scrollbar.pointer_presence(presence, now);
                SettingsScrollbarPointerOutcome {
                    handled: false,
                    presentation_changed: self.keybindings_scrollbar.presentation()
                        != previous_presentation,
                }
            }
        }
    }

    pub fn press_keybindings_scrollbar(
        &mut self,
        point: Point,
        viewport: SettingsKeybindingsViewport,
        now: Instant,
    ) -> SettingsScrollbarPointerOutcome {
        let view = viewport.list(
            self.keybindings_scroll,
            self.keybindings_scrollbar.presentation(),
        );
        let scroll_view = view.scroll_view();
        let Some(hit) = scroll_view.hit_test_scrollbar(point) else {
            return SettingsScrollbarPointerOutcome::default();
        };
        let previous_presentation = self.keybindings_scrollbar.presentation();
        let mut offset_changed = false;
        self.keybindings_scrollbar_capture = match hit.part() {
            ScrollbarPart::Thumb => scroll_view
                .begin_scrollbar_drag(hit, point)
                .map(SettingsScrollbarCapture::Thumb),
            ScrollbarPart::Track => {
                if let Some(command) = scroll_view.track_click_command(hit, point) {
                    offset_changed = self.keybindings_scroll.apply(
                        command,
                        scroll_view.metrics(),
                        ScrollAxis::Vertical,
                    );
                }
                Some(SettingsScrollbarCapture::Track)
            }
        };
        self.keybindings_scrollbar.begin_drag(now);
        SettingsScrollbarPointerOutcome {
            handled: true,
            presentation_changed: offset_changed
                || self.keybindings_scrollbar.presentation() != previous_presentation,
        }
    }

    pub fn release_keybindings_scrollbar(
        &mut self,
        point: Point,
        viewport: SettingsKeybindingsViewport,
        now: Instant,
    ) -> SettingsScrollbarPointerOutcome {
        if self.keybindings_scrollbar_capture.take().is_none() {
            return SettingsScrollbarPointerOutcome::default();
        }
        let previous_presentation = self.keybindings_scrollbar.presentation();
        let presence = self.keybindings_scrollbar_presence(point, viewport);
        self.keybindings_scrollbar.end_drag(presence, now);
        SettingsScrollbarPointerOutcome {
            handled: true,
            presentation_changed: self.keybindings_scrollbar.presentation()
                != previous_presentation,
        }
    }

    pub fn keybindings_scrollbar_pointer_left(&mut self, now: Instant) -> bool {
        if self.keybindings_scrollbar_capture.is_some() {
            return false;
        }
        let previous = self.keybindings_scrollbar.presentation();
        self.keybindings_scrollbar
            .pointer_presence(ScrollbarPointerPresence::Outside, now);
        self.keybindings_scrollbar.presentation() != previous
    }

    pub fn cancel_keybindings_scrollbar(&mut self) {
        self.keybindings_scrollbar_capture = None;
        self.keybindings_scrollbar.cancel();
    }

    pub fn advance_keybindings_scrollbar(&mut self, now: Instant) -> bool {
        self.keybindings_scrollbar.advance(now)
    }

    pub const fn keybindings_scrollbar_deadline(&self) -> Option<Instant> {
        self.keybindings_scrollbar.next_deadline()
    }

    fn keybindings_scrollbar_presence(
        &self,
        point: Point,
        viewport: SettingsKeybindingsViewport,
    ) -> ScrollbarPointerPresence {
        let view = viewport.list(
            self.keybindings_scroll,
            self.keybindings_scrollbar.presentation(),
        );
        if view.scroll_view().hit_test_scrollbar(point).is_some() {
            ScrollbarPointerPresence::Over
        } else {
            ScrollbarPointerPresence::Outside
        }
    }

    pub fn selected_search_text(&self) -> Option<&str> {
        self.search.selected_text()
    }

    pub fn apply_search(&mut self, command: TextInputCommand) {
        self.search.apply(command);
    }

    pub fn apply_search_composition(&mut self, event: TextInputCompositionEvent) {
        self.search.apply_composition(event);
    }

    pub fn cancel_search_composition(&mut self) {
        self.search.cancel_composition();
    }

    pub const fn keyboard_shortcuts(&self) -> &KeyboardShortcutsState {
        &self.keyboard_shortcuts
    }

    pub fn reopen(&mut self) {
        self.keyboard_shortcuts.reset();
        self.cancel_keybindings_scrollbar();
    }

    pub fn reset_keyboard_shortcut_recording(&mut self) {
        self.keyboard_shortcuts.reset();
    }

    pub fn start_keyboard_shortcut_recording(&mut self, command: AppCommandId) {
        self.keyboard_shortcuts.start_recording(command);
    }

    pub fn close(&mut self) {
        self.search.cancel_composition();
        self.keyboard_shortcuts.reset();
        self.cancel_keybindings_scrollbar();
    }

    pub fn activate(&mut self, id: ElementId) -> SettingsActivation {
        match id {
            SETTINGS_NAV_BACK => SettingsActivation::Close,
            SETTINGS_NAV_GENERAL => self.select(SettingsPageSection::General),
            SETTINGS_NAV_APPEARANCE => self.select(SettingsPageSection::Appearance),
            SETTINGS_NAV_KEYBINDINGS => self.select(SettingsPageSection::Keybindings),
            SETTINGS_NAV_REMOTE => {
                self.section = SettingsPageSection::Remote;
                SettingsActivation::OpenRemote
            }
            _ => {
                let Some(command) = command_for_keyboard_shortcut_row(id) else {
                    return SettingsActivation::Ignored;
                };
                if self.section != SettingsPageSection::Keybindings {
                    return SettingsActivation::Ignored;
                }
                self.keyboard_shortcuts.start_recording(command);
                SettingsActivation::Changed
            }
        }
    }

    pub fn route_keyboard_shortcut_input(
        &mut self,
        event: &KeyEvent,
        modifiers: ModifiersState,
        platform: HostPlatform,
        now: Instant,
    ) -> bool {
        if event.state != ElementState::Pressed || !self.keyboard_shortcuts.is_recording() {
            return false;
        }
        if event.logical_key == Key::Named(NamedKey::Escape) {
            self.keyboard_shortcuts.cancel_recording();
            return true;
        }
        if !event.repeat
            && let Some(chord) = recording_chord(event, modifiers, platform)
        {
            self.keyboard_shortcuts.record(chord, now);
        }
        true
    }

    pub fn advance_keyboard_shortcuts(
        &mut self,
        now: Instant,
    ) -> Option<ShortcutCommit<AppCommandId>> {
        self.keyboard_shortcuts.advance(now)
    }

    pub fn keyboard_shortcuts_saved(&mut self, command: AppCommandId) {
        self.keyboard_shortcuts.saved(command.label());
    }

    pub fn keyboard_shortcuts_save_failed(&mut self, error: impl Into<String>) {
        self.keyboard_shortcuts.save_failed(error);
    }

    pub fn keyboard_shortcuts_window_blurred(&mut self) {
        self.keyboard_shortcuts.window_blurred();
        self.cancel_keybindings_scrollbar();
    }

    pub fn keyboard_shortcuts_deadline(&self) -> Option<Instant> {
        self.keyboard_shortcuts.deadline()
    }

    fn select(&mut self, section: SettingsPageSection) -> SettingsActivation {
        if section != SettingsPageSection::Keybindings {
            self.cancel_keybindings_scrollbar();
        }
        self.section = section;
        SettingsActivation::Changed
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
