//! Hosts feature editors and details above any full-screen page.

use crate::app::App;
use crate::app::AppCommand;
use crate::app::command_panel::CommandPanel;
use crate::keymap::bindings;
use crate::render::RenderContext;
use crate::widgets::modal::ModalLayout;
use crate::widgets::navigation::Navigation;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use ratatui::Frame;
use ratatui::layout::Rect;

pub(super) fn is_open(app: &App) -> bool {
    app.overlay().is_some() || app.command_panel().is_some()
}

pub(super) fn layout(available: Rect) -> ModalLayout {
    ModalLayout::new(
        available,
        (available.width.saturating_mul(3) / 4).clamp(64, 100),
        (available.height.saturating_mul(4) / 5).clamp(12, 32),
    )
}

pub(super) fn body_area(panel: &CommandPanel, content: Rect) -> Rect {
    let rows = panel.body().tab_rows(content.width).min(content.height);
    let gap = u16::from(rows > 0).min(content.height.saturating_sub(rows));
    Rect::new(
        content.x,
        content.y + rows + gap,
        content.width,
        content.height.saturating_sub(rows + gap),
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::app) enum Target {
    Close,
    Tab(usize),
    List(crate::widgets::list_selection::ListSelectionPointerTarget),
}

pub(super) fn target_at(
    app: &App,
    available: Rect,
    position: ratatui::layout::Position,
) -> Option<Target> {
    let layout = layout(available);
    if layout.close.contains(position) {
        return Some(Target::Close);
    }
    if app.overlay().is_some() {
        return None;
    }
    let panel = app.command_panel()?;
    let body = body_area(panel, layout.content);
    let tabs = Rect {
        height: panel
            .body()
            .tab_rows(layout.content.width)
            .min(layout.content.height),
        ..layout.content
    };
    match panel.list_selection() {
        Some(selection) => {
            crate::widgets::list_selection::pointer_target_at(selection, tabs, body, position)
                .map(Target::List)
        }
        None => panel.tab_at(tabs, position).map(Target::Tab),
    }
}

pub(super) fn activate(app: &mut App, available: Rect, target: Target) -> Option<AppCommand> {
    match target {
        Target::Tab(index) => {
            app.fullscreen.panels.command_mut()?.select_tab(index);
            None
        }
        Target::Close => {
            close(app);
            None
        }
        Target::List(target) => {
            let panel = app.fullscreen.panels.command_mut()?;
            let body = body_area(panel, layout(available).content);
            let outcome = panel.focus_pointer(&target, body);
            app.handle_command_panel_outcome(outcome)
        }
    }
}

pub(super) fn draw(frame: &mut Frame<'_>, app: &App, context: RenderContext<'_>) {
    if !is_open(app) {
        return;
    }
    let layout = layout(frame.area());
    context.clear_hyperlinks(layout.surface);
    if let Some(detail) = app.overlay() {
        let hints = crate::widgets::key_hint::KeyHints::new()
            .with_compact_action("↑/↓", "scroll")
            .with_action("Esc", "close");
        crate::widgets::modal::draw(
            frame,
            layout,
            detail.title(),
            &hints,
            app.key_hint_style(),
            context,
        );
        detail.draw_body(frame, layout.content, context);
    } else if let Some(panel) = app.command_panel() {
        let hovered = match app.fullscreen.pointer.hovered() {
            Some(super::pointer::PointerTarget::Modal(target)) => Some(target),
            _ => None,
        };
        let pressed = match app.fullscreen.pointer.pressed() {
            Some(super::pointer::PointerTarget::Modal(target)) => Some(target),
            _ => None,
        };
        draw_panel(
            frame,
            panel,
            layout,
            hovered,
            pressed,
            app.key_hint_style(),
            context,
        );
    }
}

pub(super) fn draw_panel(
    frame: &mut Frame<'_>,
    panel: &CommandPanel,
    layout: ModalLayout,
    hovered: Option<&Target>,
    pressed: Option<&Target>,
    hint_style: crate::config::KeyHintStyle,
    context: RenderContext<'_>,
) {
    let body = panel.body();
    crate::widgets::modal::draw(
        frame,
        layout,
        body.title(),
        panel.key_hints(),
        hint_style,
        context,
    );
    let tabs = Rect {
        height: body
            .tab_rows(layout.content.width)
            .min(layout.content.height),
        ..layout.content
    };
    let tab = |target: Option<&Target>| match target {
        Some(Target::Tab(index))
        | Some(Target::List(crate::widgets::list_selection::ListSelectionPointerTarget::Tab(
            index,
        ))) => Some(*index),
        _ => None,
    };
    let hovered_list = match hovered {
        Some(Target::List(target)) => Some(target),
        _ => None,
    };
    let pressed_list = match pressed {
        Some(Target::List(target)) => Some(target),
        _ => None,
    };
    body.draw_tabs(frame, tabs, tab(hovered), tab(pressed), context);
    body.draw_body(
        frame,
        body_area(panel, layout.content),
        hovered_list,
        pressed_list,
        context,
    );
}

/// An open modal consumes every key; unhandled content input never reaches the page below.
pub(super) fn handle_key(
    app: &mut App,
    key: KeyEvent,
    available: Rect,
) -> Option<Option<AppCommand>> {
    if !is_open(app) {
        return None;
    }
    let layout = layout(available);
    if let Some(detail) = app.overlay_mut() {
        if key.kind == KeyEventKind::Press && bindings::CLOSE.matches(key) {
            super::navigation::close_overlay(app);
        } else if let Some(navigation) = Navigation::from_key(key) {
            detail.scroll_body(navigation, layout.content);
        }
        return Some(None);
    }
    let panel = app.fullscreen.panels.command_mut()?;
    let area = body_area(panel, layout.content);
    let outcome = panel.handle_key(key, area);
    Some(app.handle_command_panel_outcome(outcome))
}

pub(super) fn close(app: &mut App) {
    if app.overlay().is_some() {
        super::navigation::close_overlay(app);
    } else {
        super::navigation::close_command_panel(app);
    }
}

pub(super) fn process_resources_visible(app: &App, available: Rect) -> bool {
    app.overlay().is_none()
        && app.command_panel().is_some_and(|panel| {
            panel.process_resources_visible(body_area(panel, layout(available).content))
        })
}

#[cfg(test)]
#[path = "modal_tests.rs"]
mod tests;
