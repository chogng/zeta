use super::super::MentionPopupView;
use crate::ui::bottom_anchored_area;
use crate::ui::horizontal_margin;
use crate::ui::{highlight, muted};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;

#[derive(Clone, Copy)]
struct PopupLayout {
    area: Rect,
    first_path: usize,
    visible_rows: usize,
}

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, popup: Option<MentionPopupView<'_>>) {
    let Some(popup) = popup else {
        return;
    };
    let Some(layout) = popup_layout(area, popup.matches.len(), popup.selected) else {
        return;
    };
    let lines = if popup.matches.is_empty() {
        vec![Line::from(Span::styled(
            if popup.searching {
                "Searching workspace files…"
            } else {
                "No matching workspace files"
            },
            Style::default().fg(muted()),
        ))]
    } else {
        popup
            .matches
            .iter()
            .enumerate()
            .skip(layout.first_path)
            .take(layout.visible_rows)
            .map(|(index, mention_match)| {
                let base_style = if index == popup.selected {
                    Style::default()
                        .fg(highlight())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(muted())
                };
                let mut matched_indices = mention_match.indices.iter().peekable();
                Line::from(
                    mention_match
                        .path
                        .chars()
                        .enumerate()
                        .map(|(char_index, character)| {
                            let mut style = base_style;
                            if matched_indices
                                .peek()
                                .is_some_and(|matched| **matched == char_index)
                            {
                                matched_indices.next();
                                style = style.add_modifier(Modifier::BOLD);
                            }
                            Span::styled(character.to_string(), style)
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect()
    };
    frame.render_widget(Clear, layout.area);
    frame.render_widget(Paragraph::new(lines), layout.area);
}

pub(crate) fn mention_index_at(
    area: Rect,
    popup: Option<MentionPopupView<'_>>,
    column: u16,
    row: u16,
) -> Option<usize> {
    let popup = popup?;
    if popup.matches.is_empty() {
        return None;
    }
    let layout = popup_layout(area, popup.matches.len(), popup.selected)?;
    if column < layout.area.x
        || column >= layout.area.right()
        || row < layout.area.y
        || row >= layout.area.bottom()
    {
        return None;
    }
    let index = layout.first_path + usize::from(row - layout.area.y);
    (index < popup.matches.len()).then_some(index)
}

fn popup_layout(area: Rect, path_count: usize, selected: usize) -> Option<PopupLayout> {
    let max_rows = area.height.saturating_sub(2).min(6) as usize;
    if max_rows == 0 {
        return None;
    }
    let visible_rows = path_count.clamp(1, max_rows);
    let first_path = selected
        .saturating_add(1)
        .saturating_sub(max_rows)
        .min(path_count.saturating_sub(visible_rows));
    Some(PopupLayout {
        area: horizontal_margin(bottom_anchored_area(area, visible_rows as u16), 2),
        first_path,
        visible_rows,
    })
}
