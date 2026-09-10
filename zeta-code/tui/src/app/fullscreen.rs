//! Full-screen page composition and transient pointer state.

mod composer;
mod footer;
mod header;
mod layout;
pub(super) mod navigation;
mod panel;
pub(super) mod pointer;
pub(super) mod selection;

pub(super) use layout::layout;

const JUMP_LABEL: &str = "Jump to bottom (click) ↓";

use crate::app::App;
use crate::render::Renderable;
use crate::sessions;
use crate::thread::composer as chat_composer;
use crate::thread::transcript::ChatHistoryView;
use pointer::PointerInteraction;
use pointer::PointerTarget;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui::widgets::Paragraph;
use selection::ScreenSelection;

/// Owns interaction state that exists only while the full-screen page is active.
#[derive(Debug)]
pub(super) struct Fullscreen {
    pub(super) preview: crate::thread::transcript::viewport::PreviewViewport,
    pub(super) escape: crate::app::escape::ScreenEscapeSequence,
    pub(super) panels: crate::app::command_panel::Panels,
    pub(super) viewports: crate::thread::transcript::viewport::Viewports,
    pub(super) pointer: PointerInteraction<PointerTarget>,
    pub(super) selection: ScreenSelection,
}

impl Fullscreen {
    pub(super) fn new(thread: zeta_protocol::ThreadId) -> Self {
        Self {
            preview: Default::default(),
            escape: Default::default(),
            panels: Default::default(),
            viewports: crate::thread::transcript::viewport::Viewports::new(thread),
            pointer: Default::default(),
            selection: Default::default(),
        }
    }

    pub(super) fn clear(&mut self) {
        self.pointer.clear();
        self.selection.clear();
    }
}

pub(super) fn draw(
    frame: &mut Frame<'_>,
    app: &App,
    links: &std::cell::RefCell<crate::terminal::hyperlinks::FrameLinks>,
) {
    let context = app.render_context().with_hyperlinks(links);
    frame.render_widget(
        Block::default().style(
            Style::default()
                .fg(context.foreground())
                .bg(context.background()),
        ),
        frame.area(),
    );
    let areas = layout(app, frame.area());
    if let Some(preview) = app.session_preview() {
        let messages = preview.messages();
        let header = header::history_buffer(
            areas.session.transcript.width,
            areas.session.transcript.height,
            app.welcome(),
            context,
        );
        ChatHistoryView {
            jump_label: JUMP_LABEL,
            header: Some(&header),
            messages: &messages,
            scroll: &app.fullscreen.preview.scroll,
            render_cache: &app.fullscreen.preview.cache,
            pointer: pointer::transcript_pointer(app),
        }
        .render(frame, areas.session.transcript, context);
        let title = format!("  Preview · {} · read only", preview.title);
        frame.render_widget(
            Paragraph::new(title).style(Style::default().fg(context.muted())),
            areas.session.composer,
        );
        if let Some(notice) = preview.notice() {
            frame.render_widget(
                Paragraph::new(notice).style(Style::default().fg(context.muted())),
                areas.session.top_tip,
            );
        }
        footer::draw(frame, areas.session.bottom, app, context);
        if let Some(overlay) = app.overlay() {
            context.clear_hyperlinks(overlay.surface(areas.transient_area()));
            crate::widgets::overlay::draw(frame, areas.transient_area(), overlay, context);
        }
        app.fullscreen.selection.draw(frame.buffer_mut(), context);
        return;
    }
    let hovered = app.fullscreen.pointer.hovered();
    let pressed = app.fullscreen.pointer.pressed();
    if let Some(manager) = app.issue_manager() {
        manager.draw(frame, areas.session.transcript, context);
    } else if let Some(manager) = app.session_manager_view() {
        let manager_areas = layout::manager_areas(
            areas.session.transcript,
            header::desired_height(areas.session.transcript.width),
        );
        header::draw(frame, manager_areas.welcome, app.welcome(), context);
        sessions::draw_manager(frame, manager_areas.sessions, manager, None, None, context);
    } else {
        let messages = app.visible_transcript_views();
        let header = header::history_buffer(
            areas.session.transcript.width,
            areas.session.transcript.height,
            app.welcome(),
            context,
        );
        ChatHistoryView {
            jump_label: JUMP_LABEL,
            header: Some(&header),
            messages: &messages,
            scroll: app.transcript_scroll(),
            render_cache: app.transcript_render_cache(),
            pointer: pointer::transcript_pointer(app),
        }
        .render(frame, areas.session.transcript, context);
    }
    composer::draw(frame, app, &areas, context);
    if let Some(overlay) = app.overlay() {
        context.clear_hyperlinks(overlay.surface(areas.transient_area()));
        crate::widgets::overlay::draw(frame, areas.transient_area(), overlay, context);
    } else if app.completion_visible() {
        context.clear_hyperlinks(areas.completion_area());
        let input_view = app.chat_composer_view();
        let hovered_composer = match hovered {
            Some(PointerTarget::Composer(target)) => Some(*target),
            _ => None,
        };
        let pressed_composer = match pressed {
            Some(PointerTarget::Composer(target)) => Some(*target),
            _ => None,
        };
        chat_composer::draw_completion_layer(
            frame,
            areas.completion_area(),
            &input_view,
            hovered_composer,
            pressed_composer,
            context,
        );
    }
    app.fullscreen.selection.draw(frame.buffer_mut(), context);
}

#[cfg(test)]
#[path = "fullscreen/frame_tests.rs"]
mod tests;

pub(super) fn process_resource_demand(
    app: &App,
    area: Rect,
) -> zeta_memory_diagnostics::ProcessResourceDemand {
    footer::process_resource_demand(app, &layout(app, area))
}
