use super::ListSelectionState;
use super::state::ListSelectionItem;
use crate::render::RenderContext;
use crate::render::line_to_borrowed;
use crate::render::selection_marker;
use crate::widgets::search_box;
use crate::widgets::search_box::SEARCH_BOX_HEIGHT;
use crate::widgets::tab_list;
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
use ratatui::widgets::Paragraph;
use std::rc::Rc;
use unicode_width::UnicodeWidthStr;

const ITEM_STATE_COLUMN_WIDTH: u16 = 2;
const ITEM_COLUMN_GAP: u16 = 4;

pub(crate) fn draw_tabs(
    frame: &mut Frame<'_>,
    area: Rect,
    view: &ListSelectionState,
    hovered_tab: Option<usize>,
    pressed_tab: Option<usize>,
    context: RenderContext<'_>,
) {
    if !view.show_tabs() {
        return;
    }
    tab_list::draw(
        frame,
        area,
        view.tab_list(),
        view.tabs_focused(),
        hovered_tab,
        pressed_tab,
        context,
    );
}

pub(crate) fn draw_body_with_pointer(
    frame: &mut Frame<'_>,
    area: Rect,
    view: &ListSelectionState,
    hovered_search: bool,
    pressed_search: bool,
    hovered_item: Option<usize>,
    pressed_item: Option<usize>,
    context: RenderContext<'_>,
) {
    if area.is_empty() {
        return;
    }
    let areas = body_areas(area, view);

    if let Some(search) = view.search() {
        search_box::draw(
            frame,
            areas[0],
            search,
            hovered_search,
            pressed_search,
            context,
        );
    }

    let visible_items = view.visible_items();
    let rendered_rows = usize::from(areas[1].height).min(visible_items.len());
    let first_row = view.first_rendered_row(rendered_rows);
    if visible_items.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                view.empty_message(),
                Style::default().fg(context.muted()),
            ))),
            areas[1],
        );
    } else {
        let list_area = with_state_column(areas[1]);
        let column_layout = ItemColumnLayout::new(list_area.width, &visible_items);
        for (row, (index, item)) in visible_items
            .iter()
            .enumerate()
            .skip(first_row)
            .take(rendered_rows)
            .enumerate()
        {
            let row_area = Rect::new(
                list_area.x,
                areas[1].y.saturating_add(row as u16),
                list_area.width,
                1,
            );
            draw_item(
                frame,
                row_area,
                item,
                view.selected_visible_index() == Some(index),
                hovered_item == Some(index),
                pressed_item == Some(index),
                column_layout,
                context,
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
            .split(areas[2]);
        let separator_color = preview.separator_color().unwrap_or_else(|| context.muted());
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
            frame.render_widget(Paragraph::new(line_to_borrowed(line)), row);
        }
        frame.render_widget(
            Paragraph::new(dashed_rule(preview_areas[3].width, None, separator_color)),
            preview_areas[3],
        );
        if let Some(caption) = preview.caption() {
            frame.render_widget(Paragraph::new(line_to_borrowed(caption)), preview_areas[4]);
        }
    }
}

impl ListSelectionState {
    pub(crate) fn tab_index_in(&self, area: Rect, column: u16, row: u16) -> Option<usize> {
        if !self.show_tabs() {
            return None;
        }
        self.tab_list().index_at(area, column, row)
    }

    pub(crate) fn item_index_in(&self, area: Rect, column: u16, row: u16) -> Option<usize> {
        if area.is_empty() {
            return None;
        }
        let areas = body_areas(area, self);
        let list_area = with_state_column(areas[1]);
        let visible_items = self.visible_items();
        let rendered_rows = usize::from(areas[1].height).min(visible_items.len());
        let first_row = self.first_rendered_row(rendered_rows);
        if column < list_area.x
            || column >= list_area.right()
            || row < areas[1].y
            || row >= areas[1].y.saturating_add(rendered_rows as u16)
        {
            return None;
        }
        let index = first_row.saturating_add(usize::from(row - areas[1].y));
        (index < visible_items.len()).then_some(index)
    }

    pub(crate) fn search_contains_in(&self, area: Rect, column: u16, row: u16) -> bool {
        if self.search().is_none() {
            return false;
        }
        if area.is_empty() {
            return false;
        }
        body_areas(area, self)[0].contains(ratatui::layout::Position::new(column, row))
    }
}

fn with_state_column(area: Rect) -> Rect {
    let state_column_width = ITEM_STATE_COLUMN_WIDTH.min(area.x);
    Rect {
        x: area.x.saturating_sub(state_column_width),
        width: area.width.saturating_add(state_column_width),
        ..area
    }
}

fn body_areas(content: Rect, view: &ListSelectionState) -> Rc<[Rect]> {
    let search_height = view.search().map(|_| SEARCH_BOX_HEIGHT).unwrap_or(0);
    let preview_height = view
        .selected_item()
        .and_then(|item| item.preview())
        .map(|preview| preview.desired_height())
        .unwrap_or_default()
        .min(u16::MAX as usize) as u16;
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
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
    fn new(width: u16, items: &[&ListSelectionItem]) -> Self {
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
            .min(width.saturating_sub(ITEM_STATE_COLUMN_WIDTH));
        let width_before_trailing = width
            .saturating_sub(ITEM_STATE_COLUMN_WIDTH)
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
    item: &ListSelectionItem,
    selected: bool,
    hovered: bool,
    pressed: bool,
    column_layout: ItemColumnLayout,
    context: RenderContext<'_>,
) {
    let row_style = item_style(context, selected, hovered, pressed);
    frame.render_widget(Block::default().style(row_style), area);
    let label_style = if selected && !pressed {
        item.selection_foreground()
            .map(|foreground| row_style.fg(foreground))
            .unwrap_or(row_style)
    } else if selected || hovered || pressed {
        row_style
    } else {
        row_style
    };
    let marker = selection_marker(selected);
    let marker_style = if selected {
        Style::default()
            .fg(context.foreground())
            .add_modifier(Modifier::BOLD)
    } else {
        label_style
    };
    let detail_style = if !selected && !hovered && !pressed {
        Style::default().fg(context.muted())
    } else {
        row_style
    };
    let Some(columns) = item.columns() else {
        let spans = item_spans(item, marker, marker_style, label_style, detail_style);
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
        return;
    };

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(marker, marker_style),
            Span::styled(columns.leading.as_str(), label_style),
        ])),
        Rect::new(
            area.x,
            area.y,
            ITEM_STATE_COLUMN_WIDTH.saturating_add(column_layout.leading_width),
            1,
        ),
    );

    let middle_x = area
        .x
        .saturating_add(ITEM_STATE_COLUMN_WIDTH)
        .saturating_add(column_layout.leading_width)
        .saturating_add(column_layout.gap);
    let trailing_x = area
        .x
        .saturating_add(area.width.saturating_sub(column_layout.trailing_width));
    let middle_width = trailing_x
        .saturating_sub(column_layout.gap)
        .saturating_sub(middle_x);
    frame.render_widget(
        Paragraph::new(Span::styled(columns.middle.as_str(), detail_style)),
        Rect::new(middle_x, area.y, middle_width, 1),
    );
    frame.render_widget(
        Paragraph::new(Span::styled(columns.trailing.as_str(), detail_style)),
        Rect::new(trailing_x, area.y, column_layout.trailing_width, 1),
    );
}

fn item_spans<'a>(
    item: &'a ListSelectionItem,
    marker: &'static str,
    marker_style: Style,
    label_style: Style,
    detail_style: Style,
) -> Vec<Span<'a>> {
    let Some(description) = item.description() else {
        return vec![
            Span::styled(marker, marker_style),
            Span::styled(item.label(), label_style),
        ];
    };
    vec![
        Span::styled(marker, marker_style),
        Span::styled(item.label(), label_style),
        Span::styled("  ·  ", detail_style),
        Span::styled(description, detail_style),
    ]
}

fn item_style(context: RenderContext<'_>, selected: bool, hovered: bool, pressed: bool) -> Style {
    let mut style = Style::default().fg(if pressed {
        context.pressed_foreground()
    } else if selected || hovered {
        context.foreground()
    } else {
        context.muted()
    });
    if selected || pressed {
        style = style.add_modifier(Modifier::BOLD);
    }
    style
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

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
