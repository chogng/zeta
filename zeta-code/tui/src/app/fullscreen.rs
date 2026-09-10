//! Full-screen page composition and transient pointer state.

mod composer;
mod conversation;
mod footer;
mod header;
mod home;
mod layout;
mod modal;
pub(super) mod navigation;
pub(super) mod pointer;
pub(super) mod selection;

pub(super) use layout::layout;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::app) enum Page {
    Home,
    #[default]
    Conversation,
}

const JUMP_LABEL: &str = "Jump to bottom (click) ↓";

use crate::app::App;
use crate::thread::composer as chat_composer;
use pointer::PointerInteraction;
use pointer::PointerTarget;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Block;
use selection::ScreenSelection;

/// Owns interaction state that exists only while the full-screen page is active.
#[derive(Debug)]
pub(super) struct Fullscreen {
    pub(super) sessions: crate::sessions::SessionNavigation,
    pub(super) issues: crate::issues::Manager,
    pub(super) agent_thread_switcher: crate::thread::AgentThreadSwitcher,
    page: Page,
    home: home::Home,
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
            sessions: Default::default(),
            issues: Default::default(),
            agent_thread_switcher: Default::default(),
            page: Page::Conversation,
            home: home::Home::default(),
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

    pub(in crate::app) fn home_visible(&self) -> bool {
        self.page == Page::Home
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
    header::draw(frame, areas.header, app, context);
    let hovered = app.fullscreen.pointer.hovered();
    let pressed = app.fullscreen.pointer.pressed();
    if app.fullscreen.home_visible() {
        home::draw(frame, areas.session.transcript, app, context);
    } else {
        conversation::draw(frame, &areas, app, context);
    }
    if app.session_preview().is_none() {
        composer::draw(frame, app, &areas, context);
    }
    if modal::is_open(app) {
        modal::draw(frame, app, context);
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
    if modal::is_open(app) {
        return if modal::process_resources_visible(app, area) {
            zeta_memory_diagnostics::ProcessResourceDemand::Detailed
        } else {
            zeta_memory_diagnostics::ProcessResourceDemand::Disabled
        };
    }
    footer::process_resource_demand(app, &layout(app, area))
}
