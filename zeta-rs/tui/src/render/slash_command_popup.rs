use super::theme::HIGHLIGHT;
use super::theme::MUTED;
use crate::app::App;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;

const COMMAND_COLUMN_WIDTH: usize = 26;

#[derive(Clone, Copy)]
struct PopupLayout {
    area: Rect,
    first_command: usize,
    visible_rows: usize,
}

pub(super) fn draw(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let Some(popup) = app.slash_popup() else {
        return;
    };
    let Some(layout) = popup_layout(area, popup.commands.len(), popup.selected) else {
        return;
    };
    let lines = if popup.commands.is_empty() {
        vec![Line::from(Span::styled(
            "No matching commands",
            Style::default().fg(MUTED),
        ))]
    } else {
        popup
            .commands
            .iter()
            .enumerate()
            .skip(layout.first_command)
            .take(layout.visible_rows)
            .map(|(index, command)| {
                let selected = index == popup.selected;
                let command_style = if selected {
                    Style::default().fg(HIGHLIGHT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(MUTED)
                };
                Line::from(vec![
                    Span::styled(
                        format!(
                            "/{:<width$}",
                            command.command(),
                            width = COMMAND_COLUMN_WIDTH
                        ),
                        command_style,
                    ),
                    Span::styled(command.description(), command_style),
                ])
            })
            .collect()
    };
    frame.render_widget(Clear, layout.area);
    frame.render_widget(Paragraph::new(lines), layout.area);
}

pub(super) fn command_index_at(area: Rect, app: &App, column: u16, row: u16) -> Option<usize> {
    let popup = app.slash_popup()?;
    if popup.commands.is_empty() {
        return None;
    }
    let layout = popup_layout(area, popup.commands.len(), popup.selected)?;
    if column < layout.area.x
        || column >= layout.area.right()
        || row < layout.area.y
        || row >= layout.area.bottom()
    {
        return None;
    }
    let index = layout.first_command + usize::from(row - layout.area.y);
    (index < popup.commands.len()).then_some(index)
}

fn popup_layout(area: Rect, command_count: usize, selected: usize) -> Option<PopupLayout> {
    let max_rows = area.height.saturating_sub(2).min(6) as usize;
    if max_rows == 0 {
        return None;
    }
    let visible_rows = command_count.clamp(1, max_rows);
    let first_command = selected
        .saturating_add(1)
        .saturating_sub(max_rows)
        .min(command_count.saturating_sub(visible_rows));
    Some(PopupLayout {
        area: Rect {
            x: area.x.saturating_add(2),
            y: area.bottom().saturating_sub(visible_rows as u16),
            width: area.width.saturating_sub(4),
            height: visible_rows as u16,
        },
        first_command,
        visible_rows,
    })
}
