use super::SelectionViewState;
use super::state::SelectionItem;
use crate::components::search_box;
use crate::components::search_box::SEARCH_BOX_HEIGHT;
use crate::components::tab_list;
use crate::ui::horizontal_margin;
use crate::ui::{highlight, muted};
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use std::rc::Rc;
use unicode_width::UnicodeWidthStr;

const ITEM_MARKER_WIDTH: u16 = 2;
const ITEM_COLUMN_GAP: u16 = 4;

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, view: &SelectionViewState) {
    let presentation_highlight = view.presentation_highlight().unwrap_or_else(highlight);
    frame.render_widget(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(presentation_highlight)),
        area,
    );
    let content = content_area(area);
    if content.is_empty() {
        return;
    }

    let tab_height = if view.show_tabs() {
        tab_list::desired_height(view.tabs(), content.width)
    } else {
        0
    };
    let areas = selection_areas(content, view, tab_height);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            view.title(),
            Style::default()
                .fg(presentation_highlight)
                .add_modifier(Modifier::BOLD),
        ))),
        areas[1],
    );
    if view.show_tabs() {
        tab_list::draw(frame, areas[3], view.tab_list(), presentation_highlight);
    }

    if let Some(search) = view.search() {
        search_box::draw(frame, areas[4], search, presentation_highlight);
    }

    let visible_items = view.visible_items();
    let rendered_rows = usize::from(areas[5].height).min(visible_items.len());
    let first_row = view.first_rendered_row(rendered_rows);
    if visible_items.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                view.empty_message(),
                Style::default().fg(muted()),
            ))),
            areas[5],
        );
    } else {
        let column_layout = ItemColumnLayout::new(areas[5].width, &visible_items);
        for (row, (index, item)) in visible_items
            .iter()
            .enumerate()
            .skip(first_row)
            .take(rendered_rows)
            .enumerate()
        {
            let row_area = Rect::new(
                areas[5].x,
                areas[5].y.saturating_add(row as u16),
                areas[5].width,
                1,
            );
            draw_item(
                frame,
                row_area,
                item,
                view.selected_visible_index() == Some(index),
                column_layout,
            );
        }
    }
    if let Some(item) = view.selected_item()
        && let Some(preview) = item.preview()
    {
        let caption_height = u16::from(preview.caption().is_some());
        let top_margin = preview.top_margin().min(u16::MAX as usize) as u16;
        let line_height = preview.lines().len().min(u16::MAX as usize) as u16;
        let bottom_margin = preview.bottom_margin().min(u16::MAX as usize) as u16;
        let preview_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(top_margin),
                Constraint::Length(1),
                Constraint::Length(line_height),
                Constraint::Length(1),
                Constraint::Length(caption_height),
                Constraint::Length(bottom_margin),
            ])
            .split(areas[6]);
        let separator_color = preview.separator_color().unwrap_or_else(muted);
        frame.render_widget(
            Paragraph::new(dashed_rule(
                preview_areas[1].width,
                Some(preview.title()),
                separator_color,
            )),
            preview_areas[1],
        );
        for (offset, line) in preview
            .lines()
            .iter()
            .take(usize::from(preview_areas[2].height))
            .enumerate()
        {
            let row = Rect::new(
                preview_areas[2].x,
                preview_areas[2].y.saturating_add(offset as u16),
                preview_areas[2].width,
                1,
            );
            frame.render_widget(Block::default().style(line.style), row);
            frame.render_widget(Paragraph::new(line.clone()), row);
        }
        frame.render_widget(
            Paragraph::new(dashed_rule(preview_areas[3].width, None, separator_color)),
            preview_areas[3],
        );
        if let Some(caption) = preview.caption() {
            frame.render_widget(Paragraph::new(caption.clone()), preview_areas[4]);
        }
    }
}

impl SelectionViewState {
    pub(crate) fn tab_index_at(&self, area: Rect, column: u16, row: u16) -> Option<usize> {
        if !self.show_tabs() {
            return None;
        }
        let content = content_area(area);
        if content.is_empty() {
            return None;
        }
        let tab_height = tab_list::desired_height(self.tabs(), content.width);
        let areas = selection_areas(content, self, tab_height);
        self.tab_list().index_at(areas[3], column, row)
    }

    pub(crate) fn item_index_at(&self, area: Rect, column: u16, row: u16) -> Option<usize> {
        let content = content_area(area);
        if content.is_empty() {
            return None;
        }
        let tab_height = if self.show_tabs() {
            tab_list::desired_height(self.tabs(), content.width)
        } else {
            0
        };
        let areas = selection_areas(content, self, tab_height);
        let visible_items = self.visible_items();
        let rendered_rows = usize::from(areas[5].height).min(visible_items.len());
        let first_row = self.first_rendered_row(rendered_rows);
        if column < areas[5].x
            || column >= areas[5].right()
            || row < areas[5].y
            || row >= areas[5].y.saturating_add(rendered_rows as u16)
        {
            return None;
        }
        let index = first_row.saturating_add(usize::from(row - areas[5].y));
        (index < visible_items.len()).then_some(index)
    }
}

fn content_area(area: Rect) -> Rect {
    horizontal_margin(
        Rect {
            y: area.y.saturating_add(1),
            height: area.height.saturating_sub(1),
            ..area
        },
        2,
    )
}

fn selection_areas(content: Rect, view: &SelectionViewState, tab_height: u16) -> Rc<[Rect]> {
    let search_height = view.search().map(|_| SEARCH_BOX_HEIGHT).unwrap_or(0);
    let preview_height = view
        .selected_item()
        .and_then(|item| item.preview())
        .map(|preview| preview.desired_height())
        .unwrap_or_default()
        .min(u16::MAX as usize) as u16;
    let title_top_margin = view.title_top_margin().min(u16::MAX as usize) as u16;
    let title_bottom_margin = view.title_bottom_margin().min(u16::MAX as usize) as u16;
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(title_top_margin),
            Constraint::Length(1),
            Constraint::Length(title_bottom_margin),
            Constraint::Length(tab_height),
            Constraint::Length(search_height),
            Constraint::Min(1),
            Constraint::Length(preview_height),
        ])
        .split(content)
}

#[derive(Clone, Copy)]
struct ItemColumnLayout {
    leading_width: u16,
    trailing_width: u16,
    gap: u16,
}

impl ItemColumnLayout {
    fn new(width: u16, items: &[&SelectionItem]) -> Self {
        let desired_leading = items
            .iter()
            .filter_map(|item| item.columns())
            .map(|columns| columns.leading.width().min(u16::MAX as usize) as u16)
            .max()
            .unwrap_or_default();
        let trailing_width = items
            .iter()
            .filter_map(|item| item.columns())
            .map(|columns| columns.trailing.width().min(u16::MAX as usize) as u16)
            .max()
            .unwrap_or_default()
            .min(width.saturating_sub(ITEM_MARKER_WIDTH));
        let width_before_trailing = width
            .saturating_sub(ITEM_MARKER_WIDTH)
            .saturating_sub(trailing_width);
        let gap = ITEM_COLUMN_GAP.min(width_before_trailing / 2);
        let leading_width = desired_leading.min(
            width_before_trailing
                .saturating_sub(gap.saturating_mul(2))
                .saturating_sub(1),
        );
        Self {
            leading_width,
            trailing_width,
            gap,
        }
    }
}

fn draw_item(
    frame: &mut Frame<'_>,
    area: Rect,
    item: &SelectionItem,
    selected: bool,
    column_layout: ItemColumnLayout,
) {
    let label_style = if selected {
        Style::default()
            .fg(item.selection_foreground().unwrap_or_else(highlight))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let marker = if selected { "❯ " } else { "  " };
    let Some(columns) = item.columns() else {
        let mut spans = vec![
            Span::styled(marker, label_style),
            Span::styled(item.label(), label_style),
        ];
        if let Some(description) = item.description() {
            spans.push(Span::styled("  ·  ", Style::default().fg(muted())));
            spans.push(Span::styled(description, Style::default().fg(muted())));
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
        return;
    };

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(marker, label_style),
            Span::styled(columns.leading.as_str(), label_style),
        ])),
        Rect::new(
            area.x,
            area.y,
            ITEM_MARKER_WIDTH.saturating_add(column_layout.leading_width),
            1,
        ),
    );

    let middle_x = area
        .x
        .saturating_add(ITEM_MARKER_WIDTH)
        .saturating_add(column_layout.leading_width)
        .saturating_add(column_layout.gap);
    let trailing_x = area
        .x
        .saturating_add(area.width.saturating_sub(column_layout.trailing_width));
    let middle_width = trailing_x
        .saturating_sub(column_layout.gap)
        .saturating_sub(middle_x);
    let detail_style = Style::default().fg(muted());
    frame.render_widget(
        Paragraph::new(Span::styled(columns.middle.as_str(), detail_style)),
        Rect::new(middle_x, area.y, middle_width, 1),
    );
    frame.render_widget(
        Paragraph::new(Span::styled(columns.trailing.as_str(), detail_style)),
        Rect::new(trailing_x, area.y, column_layout.trailing_width, 1),
    );
}

fn dashed_rule(width: u16, title: Option<&str>, color: Color) -> Line<'static> {
    let width = usize::from(width);
    let prefix = title
        .map(|title| format!("╌╌ {title} "))
        .unwrap_or_default();
    let prefix = prefix.chars().take(width).collect::<String>();
    let remaining = width.saturating_sub(prefix.width());
    Line::from(Span::styled(
        format!("{prefix}{}", "╌".repeat(remaining)),
        Style::default().fg(color),
    ))
}
