use super::super::SkillSelectorView;
use crate::ui::bottom_anchored_area;
use crate::ui::highlight;
use crate::ui::horizontal_margin;
use crate::ui::muted;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;

const MAX_VISIBLE_ROWS: usize = 6;

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, popup: Option<SkillSelectorView<'_>>) {
    let Some(popup) = popup else {
        return;
    };
    let Some((popup_area, first_item, visible_rows)) = popup_layout(area, popup) else {
        return;
    };
    let lines = if popup.items.is_empty() {
        vec![Line::from(Span::styled(
            "No matching Skills",
            Style::default().fg(muted()),
        ))]
    } else {
        popup
            .items
            .iter()
            .enumerate()
            .skip(first_item)
            .take(visible_rows)
            .map(|(index, item)| {
                let style = if index == popup.selected {
                    Style::default()
                        .fg(highlight())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(muted())
                };
                Line::from(vec![
                    Span::styled(format!("${}", item.name()), style),
                    Span::styled(
                        format!("  {}", item.description()),
                        Style::default().fg(muted()),
                    ),
                ])
            })
            .collect()
    };
    frame.render_widget(Clear, popup_area);
    frame.render_widget(Paragraph::new(lines), popup_area);
}

pub(crate) fn skill_index_at(
    area: Rect,
    popup: Option<SkillSelectorView<'_>>,
    column: u16,
    row: u16,
) -> Option<usize> {
    let popup = popup?;
    if popup.items.is_empty() {
        return None;
    }
    let (popup_area, first_item, _) = popup_layout(area, popup)?;
    if column < popup_area.x
        || column >= popup_area.right()
        || row < popup_area.y
        || row >= popup_area.bottom()
    {
        return None;
    }
    let index = first_item + usize::from(row - popup_area.y);
    (index < popup.items.len()).then_some(index)
}

fn popup_layout(area: Rect, popup: SkillSelectorView<'_>) -> Option<(Rect, usize, usize)> {
    let max_rows = area.height.saturating_sub(2).min(MAX_VISIBLE_ROWS as u16) as usize;
    if max_rows == 0 {
        return None;
    }
    let visible_rows = popup.items.len().clamp(1, max_rows);
    let first_item = popup
        .selected
        .saturating_add(1)
        .saturating_sub(max_rows)
        .min(popup.items.len().saturating_sub(visible_rows));
    Some((
        horizontal_margin(bottom_anchored_area(area, visible_rows as u16), 2),
        first_item,
        visible_rows,
    ))
}
