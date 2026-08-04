use std::time::Instant;

use zeta_editor::{CodeEditorCommand, CodeEditorSelectionMode};
use zeta_terminal::{KeyModifiers, TerminalCore, TerminalKey};
use zeta_ui::{TextInputCommand, TextInputSelectionMode};
use zeta_winit::{ElementState, Key, KeyEvent, ModifiersState, NamedKey};

use crate::NativeApp;
use crate::agent_composer::{ComposerMode, ComposerSubmission};
use crate::composer_interaction::{ComposerInteractionActivation, SelectionDirection};
use crate::keybindings::{
    NativeKeybindingContext, NativeKeybindingFacts, NativeKeybindingResolution,
};
use crate::language_server_settings::LANGUAGE_SERVER_EXECUTABLE_INPUT;
use crate::shell_interaction::{
    AGENT_FILE_SEARCH_INPUT, COMPOSER, COMPOSER_INTERACTION, FILE_EDITOR_FIND_INPUT,
    FILE_EDITOR_REPLACE_INPUT, SESSION_SEARCH_INPUT,
};
use crate::terminal_selection::{read_clipboard_text, write_clipboard_text};
use zeta_agent_sidebar::AgentSidebarAction;
use zeta_settings::SETTINGS_SEARCH_INPUT;
use zui::{FocusDirection, NavigationAxis};

impl NativeApp {
    pub(super) fn keyboard_input(&mut self, event: KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }
        if self.route_layout_inspector_keyboard(&event) {
            return;
        }
        if self.keyboard_shortcuts.is_visible() {
            if self.route_keyboard_shortcuts_keyboard(&event) {
                return;
            }
            let _ = self.dispatch_primary_keyboard_input(&event);
            return;
        }
        if self.route_language_server_settings_keyboard(&event) {
            return;
        }
        if self.route_git_branch_context_menu_keyboard(&event) {
            return;
        }
        if self.route_workspace_path_picker_keyboard(&event) {
            return;
        }
        if self.route_session_context_menu_keyboard(&event) {
            return;
        }
        let direct_terminal = self.is_direct_terminal_input();
        let context = NativeKeybindingContext::from_facts(NativeKeybindingFacts {
            direct_terminal,
            terminal_surface_visible: self.workspace_surface.is_terminal(),
            session_sidebar_visible: self.session_sidebar.is_expanded(),
            agent_sidebar_visible: self.agent_sidebar.is_expanded(),
            file_search_visible: self.agent_sidebar_workspace.search_visible(),
            composer_mode: match self.composer.mode() {
                ComposerMode::Agent => "agent",
                ComposerMode::Shell => "shell",
            },
        });
        match self.keybindings.resolve(&event, self.modifiers, &context) {
            NativeKeybindingResolution::Command(command) => {
                self.dispatch_command(command.into());
                self.sync_input_focus();
                self.rebuild_presentation();
                self.request_redraw();
                return;
            }
            NativeKeybindingResolution::Consumed => return,
            NativeKeybindingResolution::NoMatch => {}
        }
        if direct_terminal {
            self.direct_terminal_keyboard_input(&event);
        } else if !self.file_editor_keyboard_input(&event)
            && !self.dispatch_primary_keyboard_input(&event)
        {
            if self.ui_dispatch.is_focused(SESSION_SEARCH_INPUT) {
                self.session_search_keyboard_input(&event);
            } else if self.ui_dispatch.is_focused(AGENT_FILE_SEARCH_INPUT) {
                self.file_search_keyboard_input(&event);
            } else {
                self.composer_keyboard_input(&event);
            }
        }
    }

    fn file_search_keyboard_input(&mut self, event: &KeyEvent) {
        if event.logical_key == Key::Named(NamedKey::Escape) {
            if self
                .agent_sidebar_workspace
                .file_search_input()
                .text()
                .is_empty()
            {
                self.agent_sidebar_workspace.set_search_visible(false);
            } else {
                self.agent_sidebar_workspace.clear_file_search();
            }
            self.file_search_changed();
            return;
        }
        if let Some(command) = text_input_command(event, self.modifiers) {
            self.agent_sidebar_workspace.apply_file_search(command);
            self.file_search_changed();
        }
    }

    fn composer_keyboard_input(&mut self, event: &KeyEvent) {
        if self.composer_interaction.is_visible() {
            match event.logical_key {
                Key::Named(NamedKey::ArrowUp) => {
                    self.composer_interaction
                        .move_selection(SelectionDirection::Previous);
                    self.reveal_composer_interaction_selection();
                    self.composer_changed();
                    return;
                }
                Key::Named(NamedKey::ArrowDown) => {
                    self.composer_interaction
                        .move_selection(SelectionDirection::Next);
                    self.reveal_composer_interaction_selection();
                    self.composer_changed();
                    return;
                }
                Key::Named(NamedKey::Enter) => {
                    let activation = self.composer_interaction.activate_selected();
                    if activation.is_none() && !self.composer_interaction.is_model_picker_visible()
                    {
                        self.submit_composer();
                        return;
                    }
                    self.apply_composer_interaction_activation(activation);
                    return;
                }
                Key::Named(NamedKey::Tab) => {
                    if let Some(completion) = self.composer_interaction.complete_selected_slash() {
                        self.composer.set_text(completion);
                    }
                    self.composer_changed();
                    return;
                }
                Key::Named(NamedKey::Escape) => {
                    if self
                        .composer_interaction
                        .dismiss(self.composer.editor().text())
                    {
                        self.composer_interaction_pane.reset();
                    }
                    self.composer_changed();
                    return;
                }
                _ if self.composer_interaction.is_model_picker_visible() => return,
                _ => {}
            }
        }
        if event.logical_key == Key::Named(NamedKey::Enter) {
            if self.modifiers.shift_key() {
                self.composer.apply(CodeEditorCommand::Newline);
                self.composer_changed();
            } else {
                self.submit_composer();
            }
            return;
        }
        if event.logical_key == Key::Named(NamedKey::Escape) {
            self.composer.cancel_composition();
            self.composer_changed();
            return;
        }
        if let Some(command) = code_editor_command(event, self.modifiers) {
            self.composer.apply(command);
            self.composer_changed();
        }
    }

    fn session_search_keyboard_input(&mut self, event: &KeyEvent) {
        if event.logical_key == Key::Named(NamedKey::Escape) {
            self.session_search.clear();
            self.session_search_changed();
            return;
        }
        if let Some(command) = text_input_command(event, self.modifiers) {
            self.session_search.apply(command);
            self.session_search_changed();
        }
    }

    pub(super) fn dispatch_primary_keyboard_input(&mut self, event: &KeyEvent) -> bool {
        if self.route_file_tree_keyboard(event) {
            return true;
        }
        let Some(presentation) = self.presentation.as_ref() else {
            return false;
        };
        let frame = presentation.interaction_frame();
        let outcome = if event.logical_key == Key::Named(NamedKey::Tab)
            && !self.composer_interaction.is_visible()
        {
            let direction = if self.modifiers.shift_key() {
                FocusDirection::Previous
            } else {
                FocusDirection::Next
            };
            Some(self.ui_dispatch.focus_in_order(frame, direction))
        } else if !matches!(
            self.ui_dispatch.focused(),
            Some(COMPOSER | SESSION_SEARCH_INPUT | AGENT_FILE_SEARCH_INPUT)
        ) {
            match &event.logical_key {
                Key::Named(NamedKey::ArrowLeft) => Some(self.ui_dispatch.focus_within_group(
                    frame,
                    FocusDirection::Previous,
                    NavigationAxis::Horizontal,
                )),
                Key::Named(NamedKey::ArrowRight) => Some(self.ui_dispatch.focus_within_group(
                    frame,
                    FocusDirection::Next,
                    NavigationAxis::Horizontal,
                )),
                Key::Named(NamedKey::ArrowUp) => Some(self.ui_dispatch.focus_within_group(
                    frame,
                    FocusDirection::Previous,
                    NavigationAxis::Vertical,
                )),
                Key::Named(NamedKey::ArrowDown) => Some(self.ui_dispatch.focus_within_group(
                    frame,
                    FocusDirection::Next,
                    NavigationAxis::Vertical,
                )),
                Key::Named(NamedKey::Enter) => Some(self.ui_dispatch.activate_focused(frame)),
                Key::Character(text) if text == " " => {
                    Some(self.ui_dispatch.activate_focused(frame))
                }
                Key::Named(NamedKey::Escape) => {
                    Some(self.ui_dispatch.focus_element(frame, COMPOSER))
                }
                _ => Some(Default::default()),
            }
        } else {
            None
        };
        let Some(outcome) = outcome else {
            return false;
        };
        self.apply_dispatch_outcome(outcome);
        true
    }

    fn route_file_tree_keyboard(&mut self, event: &KeyEvent) -> bool {
        let Some(focused) = self.ui_dispatch.focused() else {
            return false;
        };
        let navigation = match &event.logical_key {
            Key::Named(NamedKey::ArrowRight) => self
                .agent_sidebar_workspace
                .navigate_file_tree_right(focused),
            Key::Named(NamedKey::ArrowLeft) => self
                .agent_sidebar_workspace
                .navigate_file_tree_left(focused),
            _ => return false,
        };
        let Some(navigation) = navigation else {
            return false;
        };
        match navigation {
            AgentSidebarAction::Handled => {}
            AgentSidebarAction::StateChanged => {
                self.rebuild_presentation();
                self.request_redraw();
            }
            AgentSidebarAction::Focus(target) => {
                let outcome = self
                    .presentation
                    .as_ref()
                    .map(|presentation| {
                        self.ui_dispatch
                            .focus_element(presentation.interaction_frame(), target)
                    })
                    .unwrap_or_default();
                self.apply_dispatch_outcome(outcome);
            }
            AgentSidebarAction::OpenFile { path } => self.open_workspace_file(path),
            AgentSidebarAction::LoadChildren { element, path } => {
                self.load_file_tree_directory(element, path);
                self.rebuild_presentation();
                self.request_redraw();
            }
        }
        true
    }

    fn direct_terminal_keyboard_input(&mut self, event: &KeyEvent) {
        let Some(terminal) = self.terminal.as_ref() else {
            return;
        };
        let input = encode_key_event(terminal.core(), event, self.modifiers);
        self.send_terminal_input(input, "could not send terminal input");
    }

    fn submit_composer(&mut self) {
        let Some(submission) = self.composer.submission() else {
            return;
        };
        match submission {
            ComposerSubmission::AgentMessage(text) => {
                let Some(session) = self.agent_session.as_ref() else {
                    return;
                };
                if let Err(error) = session.submit_agent_message(text) {
                    eprintln!("could not submit Agent message: {error}");
                    return;
                }
            }
            ComposerSubmission::ShellCommand(command) => {
                let Some(session) = self.agent_session.as_ref() else {
                    return;
                };
                if let Err(error) = session.submit_shell_command(command) {
                    eprintln!("could not submit Shell Turn: {error}");
                    return;
                }
            }
        }
        self.composer.clear_after_submit();
        self.thread_timeline_scroll.reset();
        self.composer_changed();
    }

    pub(super) fn activate_composer_interaction_item(&mut self, index: usize) -> bool {
        if !self.composer_interaction.select_item(index) {
            return false;
        }
        let activation = self.composer_interaction.activate_selected();
        self.apply_composer_interaction_activation(activation);
        true
    }

    fn apply_composer_interaction_activation(
        &mut self,
        activation: Option<ComposerInteractionActivation>,
    ) {
        match activation {
            Some(ComposerInteractionActivation::ComposerText(text)) => {
                self.composer.set_text(text);
            }
            Some(ComposerInteractionActivation::Model(model)) => {
                if let Some(session) = self.agent_session.as_ref()
                    && let Err(error) = session.select_model(model)
                {
                    eprintln!("could not select Agent model: {error}");
                }
                self.composer.clear_after_submit();
            }
            Some(ComposerInteractionActivation::ViewChanged) => {
                self.composer_interaction_pane.reset();
            }
            None => {}
        }
        self.composer_changed();
    }

    fn reveal_composer_interaction_selection(&mut self) {
        let Some(view) = self.composer_interaction.view() else {
            return;
        };
        let Some(interaction_bounds) = self.presentation.as_ref().and_then(|presentation| {
            presentation
                .accessibility_nodes
                .iter()
                .find(|node| node.id == COMPOSER_INTERACTION)
                .map(|node| node.bounds)
        }) else {
            return;
        };
        let viewport = zeta_composer::interaction_list_bounds(interaction_bounds);
        let content = zeta_composer::interaction_content_size(viewport, view.items().len());
        let Some(command) = zeta_composer::interaction_selection_scroll_command(
            view.selected(),
            view.items().len(),
            content.width,
        ) else {
            return;
        };
        self.composer_interaction_pane
            .apply_scroll(command, viewport.size, content);
    }

    pub(super) fn copy_composer_selection(&mut self) -> bool {
        let Some(text) = self.composer.editor().selected_text() else {
            return false;
        };
        if let Err(error) = write_clipboard_text(text.to_string()) {
            eprintln!("could not copy command text: {error}");
        }
        true
    }

    pub(super) fn paste_into_composer(&mut self) {
        if self.composer_interaction.is_model_picker_visible() {
            return;
        }
        let text = match read_clipboard_text() {
            Ok(text) => text,
            Err(error) => {
                eprintln!("could not paste clipboard text: {error}");
                return;
            }
        };
        self.composer.apply(CodeEditorCommand::Insert(text));
        self.composer_changed();
    }

    pub(super) fn paste_into_terminal(&mut self) -> bool {
        let Some(terminal) = self.terminal.as_ref() else {
            return false;
        };
        let text = match read_clipboard_text() {
            Ok(text) => text,
            Err(error) => {
                eprintln!("could not paste clipboard text: {error}");
                return true;
            }
        };
        let input = terminal.core().encode_paste(&text);
        self.send_terminal_input(input, "could not send terminal paste");
        true
    }

    pub(super) fn composer_changed(&mut self) {
        let was_visible = self.composer_interaction.is_visible();
        self.composer_interaction.sync_for_composer(
            self.composer.editor().text(),
            self.composer.mode() == ComposerMode::Agent,
        );
        if was_visible != self.composer_interaction.is_visible() {
            self.composer_interaction_pane.reset();
        } else {
            self.reveal_composer_interaction_selection();
        }
        self.caret_blink.activity(Instant::now());
        self.rebuild_presentation();
        self.update_ime_cursor_area();
        self.request_redraw();
    }

    pub(super) fn session_search_changed(&mut self) {
        self.caret_blink.activity(Instant::now());
        self.rebuild_presentation();
        self.update_ime_cursor_area();
        self.request_redraw();
    }

    pub(super) fn file_search_changed(&mut self) {
        self.caret_blink.activity(Instant::now());
        self.rebuild_presentation();
        self.update_ime_cursor_area();
        self.request_redraw();
    }

    pub(super) fn send_terminal_input(&mut self, input: Vec<u8>, error_context: &str) {
        if input.is_empty() {
            return;
        }
        if let Some(terminal) = self.terminal.as_mut()
            && let Err(error) = terminal.send_input(input)
        {
            eprintln!("{error_context}: {error}");
            return;
        }
        self.terminal_scroll.reset();
        self.terminal_selection.clear();
        self.rebuild_presentation();
        self.request_redraw();
    }

    pub(super) fn copy_keybinding_target(&mut self) {
        if self.ui_dispatch.is_focused(SETTINGS_SEARCH_INPUT) {
            if let Some(text) = self.language_server_settings.selected_search_text()
                && let Err(error) = write_clipboard_text(text.to_owned())
            {
                eprintln!("could not copy settings search text: {error}");
            }
            return;
        }
        if self
            .ui_dispatch
            .is_focused(LANGUAGE_SERVER_EXECUTABLE_INPUT)
        {
            if let Some(text) = self.language_server_settings.selected_executable_text()
                && let Err(error) = write_clipboard_text(text.to_owned())
            {
                eprintln!("could not copy language server executable path: {error}");
            }
            return;
        }
        if self.ui_dispatch.is_focused(SESSION_SEARCH_INPUT) {
            if let Some(text) = self.session_search.selected_text()
                && let Err(error) = write_clipboard_text(text.to_owned())
            {
                eprintln!("could not copy session search text: {error}");
            }
            return;
        }
        if self.ui_dispatch.is_focused(AGENT_FILE_SEARCH_INPUT) {
            if let Some(text) = self.agent_sidebar_workspace.selected_file_search_text()
                && let Err(error) = write_clipboard_text(text.to_owned())
            {
                eprintln!("could not copy file search text: {error}");
            }
            return;
        }
        if self.ui_dispatch.is_focused(FILE_EDITOR_FIND_INPUT) {
            if let Some(text) = self.file_editor_search.selected_query_text()
                && let Err(error) = write_clipboard_text(text.to_owned())
            {
                eprintln!("could not copy file editor find text: {error}");
            }
            return;
        }
        if self.ui_dispatch.is_focused(FILE_EDITOR_REPLACE_INPUT) {
            if let Some(text) = self.file_editor_search.selected_replacement_text()
                && let Err(error) = write_clipboard_text(text.to_owned())
            {
                eprintln!("could not copy file editor replacement text: {error}");
            }
            return;
        }
        if self.copy_file_editor_selection() {
            return;
        }
        if !self.is_direct_terminal_input() && self.copy_composer_selection() {
            return;
        }
        self.copy_terminal_selection();
    }

    pub(super) fn paste_keybinding_target(&mut self) {
        if self.ui_dispatch.is_focused(SETTINGS_SEARCH_INPUT) {
            let Some(text) = clipboard_text("could not paste settings search text") else {
                return;
            };
            self.language_server_settings
                .apply_search(TextInputCommand::Insert(text));
            self.rebuild_presentation();
            self.request_redraw();
            return;
        }
        if self
            .ui_dispatch
            .is_focused(LANGUAGE_SERVER_EXECUTABLE_INPUT)
        {
            let Some(text) = clipboard_text("could not paste language server executable path")
            else {
                return;
            };
            self.language_server_settings
                .apply_executable(TextInputCommand::Insert(text));
            self.rebuild_presentation();
            self.request_redraw();
            return;
        }
        if self.ui_dispatch.is_focused(SESSION_SEARCH_INPUT) {
            let Some(text) = clipboard_text("could not paste session search text") else {
                return;
            };
            self.session_search.apply(TextInputCommand::Insert(text));
            self.session_search_changed();
            return;
        }
        if self.ui_dispatch.is_focused(AGENT_FILE_SEARCH_INPUT) {
            let Some(text) = clipboard_text("could not paste file search text") else {
                return;
            };
            self.agent_sidebar_workspace
                .apply_file_search(TextInputCommand::Insert(text));
            self.file_search_changed();
            return;
        }
        if self.ui_dispatch.is_focused(FILE_EDITOR_FIND_INPUT) {
            let Some(text) = clipboard_text("could not paste file editor find text") else {
                return;
            };
            self.file_editor_search
                .apply_query(TextInputCommand::Insert(text));
            let query = self.file_editor_search.query();
            if !query.text().is_empty() {
                self.file_editor_host.find_nearest(&query);
            }
            self.file_editor_changed();
            return;
        }
        if self.ui_dispatch.is_focused(FILE_EDITOR_REPLACE_INPUT) {
            let Some(text) = clipboard_text("could not paste file editor replacement text") else {
                return;
            };
            self.file_editor_search
                .apply_replacement(TextInputCommand::Insert(text));
            self.file_editor_changed();
            return;
        }
        if self.paste_into_file_editor() {
            return;
        }
        if self.is_direct_terminal_input() {
            self.paste_into_terminal();
        } else {
            self.paste_into_composer();
        }
    }

    fn is_direct_terminal_input(&self) -> bool {
        self.workspace_surface.is_terminal()
            && !self
                .ui_dispatch
                .is_focused(LANGUAGE_SERVER_EXECUTABLE_INPUT)
            && !self.ui_dispatch.is_focused(SETTINGS_SEARCH_INPUT)
            && !self.ui_dispatch.is_focused(SESSION_SEARCH_INPUT)
            && !self.ui_dispatch.is_focused(AGENT_FILE_SEARCH_INPUT)
    }
}

fn clipboard_text(error_context: &str) -> Option<String> {
    match read_clipboard_text() {
        Ok(text) => Some(text),
        Err(error) => {
            eprintln!("{error_context}: {error}");
            None
        }
    }
}

pub(crate) fn text_input_command(
    event: &KeyEvent,
    modifiers: ModifiersState,
) -> Option<TextInputCommand> {
    let selection_mode = if modifiers.shift_key() {
        TextInputSelectionMode::Extend
    } else {
        TextInputSelectionMode::Move
    };
    let shortcut = modifiers.control_key() || modifiers.super_key();
    match &event.logical_key {
        Key::Named(NamedKey::Backspace) => Some(TextInputCommand::Backspace),
        Key::Named(NamedKey::Delete) => Some(TextInputCommand::DeleteForward),
        Key::Named(NamedKey::ArrowLeft) => Some(TextInputCommand::MoveLeft(selection_mode)),
        Key::Named(NamedKey::ArrowRight) => Some(TextInputCommand::MoveRight(selection_mode)),
        Key::Named(NamedKey::Home) => Some(TextInputCommand::MoveToStart(selection_mode)),
        Key::Named(NamedKey::End) => Some(TextInputCommand::MoveToEnd(selection_mode)),
        Key::Character(text) if shortcut && text.eq_ignore_ascii_case("a") => {
            Some(TextInputCommand::SelectAll)
        }
        _ if !shortcut => event
            .text
            .as_ref()
            .map(|text| TextInputCommand::Insert(text.to_string())),
        _ => None,
    }
}

pub(crate) fn code_editor_command(
    event: &KeyEvent,
    modifiers: ModifiersState,
) -> Option<CodeEditorCommand> {
    let selection_mode = if modifiers.shift_key() {
        CodeEditorSelectionMode::Extend
    } else {
        CodeEditorSelectionMode::Move
    };
    let shortcut = modifiers.control_key() || modifiers.super_key();
    let word_modifier = modifiers.control_key() || modifiers.alt_key();
    match &event.logical_key {
        Key::Named(NamedKey::Backspace) if word_modifier => {
            Some(CodeEditorCommand::DeleteWordBackward)
        }
        Key::Named(NamedKey::Backspace) => Some(CodeEditorCommand::Backspace),
        Key::Named(NamedKey::Delete) if word_modifier => Some(CodeEditorCommand::DeleteWordForward),
        Key::Named(NamedKey::Delete) => Some(CodeEditorCommand::DeleteForward),
        Key::Named(NamedKey::Enter) if shortcut && modifiers.shift_key() => {
            Some(CodeEditorCommand::InsertLineAbove)
        }
        Key::Named(NamedKey::Enter) if shortcut => Some(CodeEditorCommand::InsertLineBelow),
        Key::Named(NamedKey::ArrowLeft) if word_modifier => {
            Some(CodeEditorCommand::MoveWordLeft(selection_mode))
        }
        Key::Named(NamedKey::ArrowLeft) => Some(CodeEditorCommand::MoveLeft(selection_mode)),
        Key::Named(NamedKey::ArrowRight) if word_modifier => {
            Some(CodeEditorCommand::MoveWordRight(selection_mode))
        }
        Key::Named(NamedKey::ArrowRight) => Some(CodeEditorCommand::MoveRight(selection_mode)),
        Key::Named(NamedKey::ArrowUp) if modifiers.alt_key() && modifiers.shift_key() => {
            Some(CodeEditorCommand::DuplicateLinesAbove)
        }
        Key::Named(NamedKey::ArrowDown) if modifiers.alt_key() && modifiers.shift_key() => {
            Some(CodeEditorCommand::DuplicateLinesBelow)
        }
        Key::Named(NamedKey::ArrowUp) if modifiers.alt_key() => {
            Some(CodeEditorCommand::MoveLinesUp)
        }
        Key::Named(NamedKey::ArrowDown) if modifiers.alt_key() => {
            Some(CodeEditorCommand::MoveLinesDown)
        }
        Key::Named(NamedKey::ArrowUp) => Some(CodeEditorCommand::MoveUp(selection_mode)),
        Key::Named(NamedKey::ArrowDown) => Some(CodeEditorCommand::MoveDown(selection_mode)),
        Key::Named(NamedKey::PageUp) => Some(CodeEditorCommand::MovePageUp(selection_mode)),
        Key::Named(NamedKey::PageDown) => Some(CodeEditorCommand::MovePageDown(selection_mode)),
        Key::Named(NamedKey::Home) => Some(CodeEditorCommand::MoveToLineStart(selection_mode)),
        Key::Named(NamedKey::End) => Some(CodeEditorCommand::MoveToLineEnd(selection_mode)),
        Key::Named(NamedKey::Tab) if modifiers.shift_key() => Some(CodeEditorCommand::Outdent),
        Key::Named(NamedKey::Tab) => Some(CodeEditorCommand::Indent),
        Key::Character(text)
            if shortcut && modifiers.shift_key() && text.eq_ignore_ascii_case("k") =>
        {
            Some(CodeEditorCommand::DeleteLines)
        }
        Key::Character(text)
            if shortcut && modifiers.shift_key() && text.eq_ignore_ascii_case("d") =>
        {
            Some(CodeEditorCommand::DeleteEmptyLines)
        }
        Key::Character(text) if shortcut && text.eq_ignore_ascii_case("j") => {
            Some(CodeEditorCommand::JoinLines)
        }
        Key::Character(text) if shortcut && text == "/" => {
            Some(CodeEditorCommand::ToggleLineComment)
        }
        Key::Character(text) if shortcut && text.eq_ignore_ascii_case("a") => {
            Some(CodeEditorCommand::SelectAll)
        }
        Key::Character(text) if shortcut && text.eq_ignore_ascii_case("z") => {
            if modifiers.shift_key() {
                Some(CodeEditorCommand::Redo)
            } else {
                Some(CodeEditorCommand::Undo)
            }
        }
        _ if !shortcut => event
            .text
            .as_ref()
            .map(|text| CodeEditorCommand::Insert(text.to_string())),
        _ => None,
    }
}

fn encode_key_event(
    terminal: &TerminalCore,
    event: &KeyEvent,
    modifiers: ModifiersState,
) -> Vec<u8> {
    if modifiers.super_key() {
        return Vec::new();
    }
    let Some(key) = terminal_key(event) else {
        return Vec::new();
    };
    terminal.encode_key(key, terminal_modifiers(modifiers))
}

fn terminal_modifiers(modifiers: ModifiersState) -> KeyModifiers {
    let mut terminal = KeyModifiers::NONE;
    if modifiers.shift_key() {
        terminal = terminal.with_shift();
    }
    if modifiers.alt_key() {
        terminal = terminal.with_alt();
    }
    if modifiers.control_key() {
        terminal = terminal.with_control();
    }
    terminal
}

fn terminal_key(event: &KeyEvent) -> Option<TerminalKey<'_>> {
    match &event.logical_key {
        Key::Character(text) => Some(TerminalKey::Text(
            event.text.as_deref().unwrap_or(text.as_str()),
        )),
        Key::Named(NamedKey::Enter) => Some(TerminalKey::Enter),
        Key::Named(NamedKey::Tab) => Some(TerminalKey::Tab),
        Key::Named(NamedKey::Backspace) => Some(TerminalKey::Backspace),
        Key::Named(NamedKey::Escape) => Some(TerminalKey::Escape),
        Key::Named(NamedKey::ArrowUp) => Some(TerminalKey::ArrowUp),
        Key::Named(NamedKey::ArrowDown) => Some(TerminalKey::ArrowDown),
        Key::Named(NamedKey::ArrowRight) => Some(TerminalKey::ArrowRight),
        Key::Named(NamedKey::ArrowLeft) => Some(TerminalKey::ArrowLeft),
        Key::Named(NamedKey::Home) => Some(TerminalKey::Home),
        Key::Named(NamedKey::End) => Some(TerminalKey::End),
        Key::Named(NamedKey::Insert) => Some(TerminalKey::Insert),
        Key::Named(NamedKey::Delete) => Some(TerminalKey::Delete),
        Key::Named(NamedKey::PageUp) => Some(TerminalKey::PageUp),
        Key::Named(NamedKey::PageDown) => Some(TerminalKey::PageDown),
        Key::Named(NamedKey::F1) => Some(TerminalKey::F1),
        Key::Named(NamedKey::F2) => Some(TerminalKey::F2),
        Key::Named(NamedKey::F3) => Some(TerminalKey::F3),
        Key::Named(NamedKey::F4) => Some(TerminalKey::F4),
        Key::Named(NamedKey::F5) => Some(TerminalKey::F5),
        Key::Named(NamedKey::F6) => Some(TerminalKey::F6),
        Key::Named(NamedKey::F7) => Some(TerminalKey::F7),
        Key::Named(NamedKey::F8) => Some(TerminalKey::F8),
        Key::Named(NamedKey::F9) => Some(TerminalKey::F9),
        Key::Named(NamedKey::F10) => Some(TerminalKey::F10),
        Key::Named(NamedKey::F11) => Some(TerminalKey::F11),
        Key::Named(NamedKey::F12) => Some(TerminalKey::F12),
        _ => None,
    }
}
