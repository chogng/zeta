mod footer;

use crate::app::App;
use crate::components::composer;
use crate::components::key_hint_bar;
use crate::components::pane;
use crate::components::selection;
use crate::components::transcript;
use crate::ui::InteractionLayout;
use crate::ui::background;
use crate::ui::foreground;
use crate::ui::frame_areas;
use crate::ui::highlight;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Block;

pub(crate) fn draw(frame: &mut Frame<'_>, app: &App) {
    frame.render_widget(
        Block::default().style(Style::default().fg(foreground()).bg(background())),
        frame.area(),
    );
    let areas = frame_areas(frame.area(), interaction_layout(app, frame.area()));
    let presentation_highlight = app
        .selection_view()
        .and_then(|view| view.presentation_highlight())
        .unwrap_or_else(highlight);

    transcript::draw(
        frame,
        areas.history,
        app.messages(),
        app.transcript_scroll(),
        app.welcome(),
        presentation_highlight,
    );
    if let Some(view) = app.selection_pane() {
        let pane_areas = pane::areas(areas.interaction);
        selection::draw(frame, pane_areas.body, view.body());
        key_hint_bar::draw(frame, pane_areas.key_hint_bar, view.key_hints());
    } else {
        composer::draw_slash_popup(frame, areas.history, app.slash_popup());
        composer::draw_mention_popup(frame, areas.history, app.mention_popup());
        composer::draw_skill_popup(frame, areas.history, app.skill_popup());
        let cursor = if app.accepts_input() {
            composer::ComposerCursor::Visible
        } else {
            composer::ComposerCursor::Hidden
        };
        composer::draw_composer(
            frame,
            areas.interaction,
            app.input(),
            app.input_cursor_width(),
            app.input_cursor_line(),
            cursor,
        );
        footer::draw(frame, areas.footer, app);
    }
}

pub(crate) fn mention_index_at(
    app: &App,
    terminal_area: Rect,
    column: u16,
    row: u16,
) -> Option<usize> {
    let areas = frame_areas(terminal_area, interaction_layout(app, terminal_area));
    composer::mention_index_at(areas.history, app.mention_popup(), column, row)
}

pub(crate) fn skill_index_at(
    app: &App,
    terminal_area: Rect,
    column: u16,
    row: u16,
) -> Option<usize> {
    let areas = frame_areas(terminal_area, interaction_layout(app, terminal_area));
    composer::skill_index_at(areas.history, app.skill_popup(), column, row)
}

pub(crate) fn slash_command_index_at(
    app: &App,
    terminal_area: Rect,
    column: u16,
    row: u16,
) -> Option<usize> {
    let areas = frame_areas(terminal_area, interaction_layout(app, terminal_area));
    composer::command_index_at(areas.history, app.slash_popup(), column, row)
}

fn interaction_layout(app: &App, terminal_area: Rect) -> InteractionLayout {
    app.selection_pane()
        .map(|view| InteractionLayout::Expanded {
            desired_height: pane::desired_height(view.body().desired_height(terminal_area.width)),
        })
        .unwrap_or(InteractionLayout::Composer {
            desired_height: app.composer_desired_height(terminal_area.width),
        })
}

#[cfg(test)]
#[path = "frame/frame_tests.rs"]
mod tests;
