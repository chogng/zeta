use super::BOTTOM_ROWS;
use super::FrameLayout;
use super::Transcript;
use crate::app::App;
use crate::render::Renderable;
use crate::thread::composer as chat_input;
use crate::thread::composer::ChatComposerSurface;
use crate::thread::goal;
use crate::thread::interaction::approval;
use crate::thread::interaction::query;
use crate::thread::plan;
use crate::thread::queue;
use ratatui::Frame;
use ratatui::layout::Rect;

pub(super) fn layout(app: &App, terminal_area: Rect, min_transcript_rows: u16) -> FrameLayout {
    if app.session_preview().is_some() {
        let session = super::super::layout::session_areas(
            terminal_area,
            0,
            0,
            0,
            0,
            1,
            1,
            0,
            0,
            min_transcript_rows,
        );
        return FrameLayout {
            input: Rect::default(),
            session,
        };
    }
    if let Some(panel) = app.command_panel() {
        return FrameLayout {
            session: super::super::layout::command_panel_areas(
                terminal_area,
                panel.desired_height(terminal_area.width),
                BOTTOM_ROWS,
            ),
            input: Rect::default(),
        };
    }
    let input_view = app.chat_composer_view();
    let input_rows = ChatComposerSurface {
        view: &input_view,
        cursor: chat_input::ChatInputCursor::Hidden,
    }
    .desired_height(terminal_area.width, app.render_context());
    let approval_rows = app
        .approval_view()
        .map(approval::desired_height)
        .unwrap_or_default();
    let query_rows = app
        .query_view()
        .map(query::desired_height)
        .unwrap_or_default();
    let composer_rows = if approval_rows > 0 {
        approval_rows
    } else {
        input_rows
    };
    let queue_rows = if app.session_manager_view().is_some() {
        0
    } else {
        let queue_view = app.queue_view();
        queue::desired_height(&queue_view, queue::DEFAULT_MAX_VISIBLE_ITEMS)
    };
    let session = super::super::layout::session_areas(
        terminal_area,
        if app.session_manager_view().is_some() {
            0
        } else {
            goal::desired_height(app.goal_view())
        },
        if app.session_manager_view().is_some() {
            0
        } else {
            plan::desired_height(app.plan_view())
        },
        queue_rows,
        query_rows,
        composer_rows,
        BOTTOM_ROWS,
        app.agent_thread_switcher_rows(),
        u16::from(app.status_indicator().is_some()),
        min_transcript_rows,
    );
    let input = if approval_rows > 0 {
        Rect {
            y: session.composer.bottom(),
            height: 0,
            ..session.composer
        }
    } else {
        let height = input_rows.min(session.composer.height);
        Rect {
            y: session.composer.bottom().saturating_sub(height),
            height,
            ..session.composer
        }
    };
    FrameLayout { session, input }
}

pub(super) fn draw(
    frame: &mut Frame<'_>,
    app: &App,
    links: &std::cell::RefCell<crate::terminal::hyperlinks::FrameLinks>,
) {
    super::draw_content(frame, app, links, Transcript::Full);
}
