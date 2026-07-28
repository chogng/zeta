mod composer;
mod footer;
mod header;
mod history;
mod layout;
mod mention_popup;
mod slash_command_popup;
mod theme;

use crate::app::App;
use ratatui::Frame;
use ratatui::layout::Rect;

pub(crate) fn draw(frame: &mut Frame<'_>, app: &App) {
    let areas = layout::frame_areas(frame.area());

    header::draw(frame, areas.header, app.status());
    history::draw(frame, areas.history, app);
    slash_command_popup::draw(frame, areas.history, app);
    mention_popup::draw(frame, areas.history, app);
    composer::draw(frame, areas.composer, app);
    footer::draw(frame, areas.footer, app.status());
}

pub(crate) fn mention_index_at(
    app: &App,
    terminal_area: Rect,
    column: u16,
    row: u16,
) -> Option<usize> {
    let areas = layout::frame_areas(terminal_area);
    mention_popup::mention_index_at(areas.history, app, column, row)
}

pub(crate) fn slash_command_index_at(
    app: &App,
    terminal_area: Rect,
    column: u16,
    row: u16,
) -> Option<usize> {
    let areas = layout::frame_areas(terminal_area);
    slash_command_popup::command_index_at(areas.history, app, column, row)
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
