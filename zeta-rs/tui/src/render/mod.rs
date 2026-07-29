mod composer;
mod footer;
mod header;
mod history;
mod layout;
mod mention_popup;
mod selection_view;
mod slash_command_popup;
mod status_line;
mod theme;

use crate::app::App;
use ratatui::Frame;
use ratatui::layout::Rect;

pub(crate) fn draw(frame: &mut Frame<'_>, app: &App) {
    let areas = layout::frame_areas(frame.area(), interaction_layout(app, frame.area()));

    header::draw(frame, areas.header, app.status());
    history::draw(frame, areas.history, app);
    if let Some(view) = app.selection_view() {
        selection_view::draw(frame, areas.interaction, view);
    } else {
        slash_command_popup::draw(frame, areas.history, app);
        mention_popup::draw(frame, areas.history, app);
        status_line::draw(frame, areas.status_line, app.status_line());
        composer::draw(frame, areas.interaction, app);
        footer::draw(frame, areas.footer, app.status());
    }
}

pub(crate) fn mention_index_at(
    app: &App,
    terminal_area: Rect,
    column: u16,
    row: u16,
) -> Option<usize> {
    let areas = layout::frame_areas(terminal_area, interaction_layout(app, terminal_area));
    mention_popup::mention_index_at(areas.history, app, column, row)
}

pub(crate) fn slash_command_index_at(
    app: &App,
    terminal_area: Rect,
    column: u16,
    row: u16,
) -> Option<usize> {
    let areas = layout::frame_areas(terminal_area, interaction_layout(app, terminal_area));
    slash_command_popup::command_index_at(areas.history, app, column, row)
}

fn interaction_layout(app: &App, terminal_area: Rect) -> layout::InteractionLayout {
    app.selection_view()
        .map(|view| layout::InteractionLayout::Expanded {
            desired_height: view.desired_height(terminal_area.width),
        })
        .unwrap_or(layout::InteractionLayout::Composer)
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
