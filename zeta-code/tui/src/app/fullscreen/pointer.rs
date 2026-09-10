use super::selection::ClickCount;
use super::selection::ScreenSelectionOutcome;
use crate::app::App;
use crate::app::AppCommand;
use crate::host;
use crate::terminal;
use crate::thread::composer as chat_composer;
use crate::thread::composer::ChatComposerPointerTarget;
use crate::thread::transcript::ChatHistoryPointerState;
use crate::thread::transcript::ChatHistoryView;
use crate::thread::transcript::TranscriptScrollDirection;
use crossterm::event::MouseButton;
use crossterm::event::MouseEvent;
use crossterm::event::MouseEventKind;
use ratatui::layout::Rect;
use std::time::Instant;

/// Keeps pointer hover separate from keyboard selection and click activation.
///
/// A pointer move may change only `hovered`. Components continue to own their keyboard cursor,
/// while a click carries its resolved target directly to the component activation path.
/// Rendering resolves hover, press, and keyboard selection independently; that visual choice must
/// never merge their state or make hover affect keyboard behavior.
#[derive(Debug)]
pub(crate) struct PointerInteraction<T> {
    hovered: Option<T>,
    pressed: Option<T>,
}

impl<T> Default for PointerInteraction<T> {
    fn default() -> Self {
        Self {
            hovered: None,
            pressed: None,
        }
    }
}

impl<T> PointerInteraction<T> {
    pub(crate) fn update_hover(&mut self, target: Option<T>) {
        self.hovered = target;
    }

    pub(crate) fn update_pressed(&mut self, target: Option<T>) {
        self.pressed = target;
    }

    pub(crate) fn clear_pressed(&mut self) {
        self.pressed = None;
    }

    pub(crate) fn clear(&mut self) {
        self.hovered = None;
        self.pressed = None;
    }

    pub(crate) fn hovered(&self) -> Option<&T> {
        self.hovered.as_ref()
    }

    pub(crate) fn pressed(&self) -> Option<&T> {
        self.pressed.as_ref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PointerTarget {
    Home,
    HomeAction(super::home::Action),
    Modal(super::modal::Target),
    Composer(ChatComposerPointerTarget),
    TranscriptJumpToBottom,
}

pub(super) fn transcript_pointer(app: &App) -> ChatHistoryPointerState<'_> {
    ChatHistoryPointerState {
        hovered_jump_to_bottom: app.fullscreen.pointer.hovered()
            == Some(&PointerTarget::TranscriptJumpToBottom),
        pressed_jump_to_bottom: app.fullscreen.pointer.pressed()
            == Some(&PointerTarget::TranscriptJumpToBottom),
        ..Default::default()
    }
}

pub(crate) fn target_at(
    app: &App,
    terminal_area: Rect,
    column: u16,
    row: u16,
) -> Option<PointerTarget> {
    if !app.mouse_mode().enables_pointer_actions() {
        return None;
    }
    let areas = super::layout(app, terminal_area);
    let position = ratatui::layout::Position::new(column, row);
    if super::modal::is_open(app) {
        return super::modal::target_at(app, terminal_area, position).map(PointerTarget::Modal);
    }
    if app.approval_view().is_none()
        && app.query_view().is_none()
        && areas.header.height > 0
        && Rect::new(areas.header.x, areas.header.y, 4.min(areas.header.width), 1)
            .contains(position)
    {
        return Some(PointerTarget::Home);
    }
    if app.completion_visible() && overlay_contains(app, terminal_area, position) {
        return chat_composer::pointer_target_at(
            areas.completion_area(),
            &app.chat_composer_view(),
            true,
            column,
            row,
        )
        .map(PointerTarget::Composer);
    }
    if app.fullscreen.home_visible() {
        return super::home::action_at(app, areas.session.transcript, position)
            .map(PointerTarget::HomeAction);
    }
    if app.issue_manager().is_some() {
        return None;
    }
    if app.session_manager_view().is_some() && app.session_preview().is_none() {
        return None;
    }
    let context = app.render_context();
    let messages = if let Some(preview) = app.session_preview() {
        preview.messages()
    } else {
        app.visible_transcript_views()
    };
    let (scroll, render_cache) = if app.session_preview().is_some() {
        (
            &app.fullscreen.preview.scroll,
            &app.fullscreen.preview.cache,
        )
    } else {
        (app.transcript_scroll(), app.transcript_render_cache())
    };
    ChatHistoryView {
        jump_label: super::JUMP_LABEL,
        header: None,
        messages: &messages,
        scroll,
        render_cache,
        pointer: transcript_pointer(app),
    }
    .jump_area(areas.session.transcript, context)
    .filter(|area| area.contains(position))
    .map(|_| PointerTarget::TranscriptJumpToBottom)
}

pub(crate) fn overlay_contains(
    app: &App,
    terminal_area: Rect,
    position: ratatui::layout::Position,
) -> bool {
    if !app.mouse_mode().captures_terminal_input() {
        return false;
    }
    let areas = super::layout(app, terminal_area);
    if super::modal::is_open(app) {
        return super::modal::layout(terminal_area)
            .surface
            .contains(position);
    }
    if !app.completion_visible() {
        return false;
    }
    chat_composer::pointer_target_at(
        areas.completion_area(),
        &app.chat_composer_view(),
        true,
        position.x,
        position.y,
    )
    .is_some()
}

pub(in crate::app) enum MouseAction {
    Selection(Option<ScreenSelectionOutcome>),
    Command(Option<AppCommand>),
}

pub(in crate::app) fn handle_mouse(
    app: &mut App,
    area: ratatui::layout::Rect,
    mouse: MouseEvent,
) -> MouseAction {
    let mouse_mode = app.mouse_mode();
    if !mouse_mode.captures_terminal_input() {
        app.fullscreen.clear();
        return MouseAction::Selection(None);
    }
    let position = ratatui::layout::Position::new(mouse.column, mouse.row);
    if super::modal::is_open(app) {
        let target = target_at(app, area, mouse.column, mouse.row);
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                app.fullscreen.pointer.update_pressed(target)
            }
            MouseEventKind::Drag(MouseButton::Left) => app.fullscreen.pointer.clear_pressed(),
            MouseEventKind::Up(MouseButton::Left) => {
                let activate = target
                    .as_ref()
                    .is_some_and(|target| app.fullscreen.pointer.pressed() == Some(target));
                app.fullscreen.pointer.clear_pressed();
                if activate && let Some(PointerTarget::Modal(target)) = target {
                    return MouseAction::Command(super::modal::activate(app, area, target));
                }
            }
            MouseEventKind::Moved => app.fullscreen.pointer.update_hover(target),
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                if super::modal::layout(area).surface.contains(position) =>
            {
                let key = crossterm::event::KeyEvent::new(
                    if mouse.kind == MouseEventKind::ScrollUp {
                        crossterm::event::KeyCode::Up
                    } else {
                        crossterm::event::KeyCode::Down
                    },
                    crossterm::event::KeyModifiers::NONE,
                );
                return MouseAction::Command(super::modal::handle_key(app, key, area).flatten());
            }
            _ => {}
        }
        return MouseAction::Selection(None);
    }
    let overlay_contains = overlay_contains(app, area, position);
    if (app.overlay().is_some() || app.completion_visible()) && !overlay_contains {
        app.fullscreen.clear();
        return MouseAction::Selection(None);
    }
    match mouse.kind {
        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
            let direction = if mouse.kind == MouseEventKind::ScrollUp {
                TranscriptScrollDirection::Up
            } else {
                TranscriptScrollDirection::Down
            };
            let command = if overlay_contains {
                scroll_pointer_item(app, area, mouse.column, mouse.row, direction)
            } else if !app.fullscreen.home_visible()
                && app.session_manager_view().is_none()
                && app.issue_manager().is_none()
                && super::layout(app, area)
                    .session
                    .transcript
                    .contains(position)
            {
                app.navigate_transcript(direction, area)
            } else {
                None
            };
            return MouseAction::Command(command);
        }
        MouseEventKind::Down(MouseButton::Left) => {
            let target = target_at(app, area, mouse.column, mouse.row);
            app.fullscreen.pointer.update_pressed(target);
            app.fullscreen.selection.begin(position);
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            app.fullscreen.pointer.clear_pressed();
            app.fullscreen.selection.drag(position);
        }
        MouseEventKind::Up(MouseButton::Left) => {
            let outcome = app.fullscreen.selection.finish(position, Instant::now());
            app.fullscreen.pointer.clear_pressed();
            return MouseAction::Selection(outcome);
        }
        MouseEventKind::Moved => update_pointer_hover(app, area, mouse.column, mouse.row),
        _ => {}
    }
    MouseAction::Selection(None)
}

pub(super) fn activate_pointer_item(
    app: &mut App,
    area: ratatui::layout::Rect,
    column: u16,
    row: u16,
) -> Option<AppCommand> {
    let target = target_at(app, area, column, row)?;
    match target {
        PointerTarget::Home => {
            app.open_home();
            None
        }
        PointerTarget::HomeAction(action) => super::home::activate(app, action),
        PointerTarget::Modal(target) => super::modal::activate(app, area, target),
        PointerTarget::TranscriptJumpToBottom => {
            app.follow_latest_transcript();
            None
        }
        PointerTarget::Composer(ChatComposerPointerTarget::CompletionItem(index)) => {
            app.activate_input_completion(index)
        }
    }
}

fn scroll_pointer_item(
    app: &mut App,
    area: ratatui::layout::Rect,
    column: u16,
    row: u16,
    direction: TranscriptScrollDirection,
) -> Option<AppCommand> {
    let position = ratatui::layout::Position::new(column, row);
    if !overlay_contains(app, area, position) {
        return None;
    }
    let key = match direction {
        TranscriptScrollDirection::Up => crossterm::event::KeyCode::Up,
        TranscriptScrollDirection::Down => crossterm::event::KeyCode::Down,
    };
    app.fullscreen.clear();
    app.handle_key_in_area(
        crossterm::event::KeyEvent::new(key, crossterm::event::KeyModifiers::NONE),
        area,
    )
}

pub(super) fn update_pointer_hover(
    app: &mut App,
    area: ratatui::layout::Rect,
    column: u16,
    row: u16,
) {
    let target = target_at(app, area, column, row);
    app.fullscreen.pointer.update_hover(target);
}

pub(in crate::app) fn finish_pointer_gesture(
    app: &mut App,
    terminal: &terminal::TerminalSession,
    outcome: Option<ScreenSelectionOutcome>,
) -> Result<Option<AppCommand>, std::io::Error> {
    let select = |app: &mut App, range| {
        super::selection::apply_screen_selection(
            app,
            range,
            |range| terminal.selected_text(range),
            host::clipboard::write_text,
        );
    };
    match outcome {
        Some(ScreenSelectionOutcome::Click {
            position,
            count: ClickCount::Single,
        }) => {
            let area = terminal.area()?;
            Ok(activate_pointer_item(app, area, position.x, position.y))
        }
        Some(ScreenSelectionOutcome::Click {
            position,
            count: ClickCount::Double,
        }) => {
            if let Some(range) = terminal.token_range_at(position) {
                select(app, range);
            }
            Ok(None)
        }
        Some(ScreenSelectionOutcome::Click {
            position,
            count: ClickCount::Triple,
        }) => {
            if let Some(range) = terminal.line_range_at(position) {
                select(app, range);
            }
            Ok(None)
        }
        Some(ScreenSelectionOutcome::Selection(range)) => {
            select(app, range);
            Ok(None)
        }
        None => Ok(None),
    }
}

#[cfg(test)]
#[path = "pointer_tests.rs"]
mod tests;
