use super::layout::Layout;
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

pub(super) fn draw(
    frame: &mut Frame<'_>,
    app: &App,
    areas: &Layout,
    context: crate::render::RenderContext<'_>,
) {
    let cursor = if app.accepts_input() && app.chat_input_focused() {
        chat_input::ChatInputCursor::Visible
    } else {
        chat_input::ChatInputCursor::Hidden
    };
    let input_view = app.chat_composer_view();
    if let Some(approval) = app.approval_view() {
        approval::draw(frame, areas.session.composer, approval, None, None, context);
    } else if let Some(panel) = app.command_panel() {
        super::panel::draw(panel, frame, areas.session.composer, context);
    } else {
        ChatComposerSurface {
            view: &input_view,
            cursor,
        }
        .render(frame, areas.input, context);
    }
    if let Some(query) = app.query_view() {
        query::draw(frame, areas.session.request, query, None, None, context);
    }
    if app.session_manager_view().is_none() {
        goal::draw(frame, areas.session.goal, app.goal_view(), context);
        plan::draw(frame, areas.session.plan, app.plan_view(), context);
        let queue_view = app.queue_view();
        queue::draw(
            frame,
            areas.session.queue,
            &queue_view,
            queue::DEFAULT_MAX_VISIBLE_ITEMS,
            None,
            None,
            context,
        );
    }
    super::footer::draw(frame, areas.session.bottom, app, context);
    if let Some(agent_thread_switcher) = app.agent_thread_switcher_view() {
        crate::thread::draw_agent_thread_switcher(
            frame,
            chat_input::content_area(areas.session.agent_thread_switcher),
            agent_thread_switcher,
            context,
        );
    }
    if let Some(indicator) = app.status_indicator() {
        indicator.draw(frame, areas.session.status_indicator, context);
    }
    super::footer::draw_tip(frame, areas.session.top_tip, app, context);
}
