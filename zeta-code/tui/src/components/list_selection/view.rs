use super::ListSelectionState;
use super::state::ListSelectionItem;
use crate::components::search_box;
use crate::components::search_box::SEARCH_BOX_HEIGHT;
use crate::components::tab_list;
use crate::render::Insets;
use crate::render::InteractionAttention;
use crate::render::InteractionState;
use crate::render::InteractionTarget;
use crate::render::RectExt;
use crate::render::RenderContext;
use crate::render::focus_style;
use crate::render::interaction_style;
use crate::render::line_to_borrowed;
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Paragraph;
use std::rc::Rc;
use unicode_width::UnicodeWidthStr;

const ITEM_STATE_COLUMN_WIDTH: u16 = 2;
const ITEM_COLUMN_GAP: u16 = 4;

#[cfg(test)]
pub(crate) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    view: &ListSelectionState,
    context: RenderContext<'_>,
) {
    draw_with_pointer(
        frame, area, view, None, None, false, false, None, None, context,
    );
}

pub(crate) fn draw_with_pointer(
    frame: &mut Frame<'_>,
    area: Rect,
    view: &ListSelectionState,
    hovered_tab: Option<usize>,
    pressed_tab: Option<usize>,
    hovered_search: bool,
    pressed_search: bool,
    hovered_item: Option<usize>,
    pressed_item: Option<usize>,
    context: RenderContext<'_>,
) {
    let content = content_area(area);
    if content.is_empty() {
        return;
    }

    let tab_height = if view.show_tabs() {
        tab_list::desired_height(
            view.tabs(),
            content.width.saturating_sub(ITEM_STATE_COLUMN_WIDTH),
        )
    } else {
        0
    };
    let areas = selection_areas(content, view, tab_height);

    if view.show_tabs() {
        let tab_area = content_after_state_column(areas[1]);
        tab_list::draw(
            frame,
            tab_area,
            view.tab_list(),
            view.tabs_focused(),
            hovered_tab,
            pressed_tab,
            context,
        );
        if view.tabs_focused() {
            draw_focus_marker(
                frame,
                Rect::new(
                    areas[1].x,
                    areas[1]
                        .y
                        .saturating_add(view.tab_list().active_row(tab_area.width)),
                    ITEM_STATE_COLUMN_WIDTH.min(areas[1].width),
                    1,
                ),
                context,
            );
        }
    }

    if let Some(search) = view.search() {
        search_box::draw(
            frame,
            content_after_state_column(areas[2]),
            search,
            search_attention(hovered_search, pressed_search),
            context,
        );
        if view.search_focused() {
            draw_focus_marker(
                frame,
                Rect::new(
                    areas[2].x,
                    areas[2].y.saturating_add(areas[2].height / 2),
                    ITEM_STATE_COLUMN_WIDTH.min(areas[2].width),
                    1,
                ),
                context,
            );
        }
    }

    let visible_items = view.visible_items();
    let rendered_rows = usize::from(areas[3].height).min(visible_items.len());
    let first_row = view.first_rendered_row(rendered_rows);
    if visible_items.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                view.empty_message(),
                Style::default().fg(context.muted()),
            ))),
            content_after_state_column(areas[3]),
        );
    } else {
        let column_layout = ItemColumnLayout::new(areas[3].width, &visible_items);
        for (row, (index, item)) in visible_items
            .iter()
            .enumerate()
            .skip(first_row)
            .take(rendered_rows)
            .enumerate()
        {
            let row_area = Rect::new(
                areas[3].x,
                areas[3].y.saturating_add(row as u16),
                areas[3].width,
                1,
            );
            draw_item(
                frame,
                row_area,
                item,
                item_attention(
                    view.items_focused() && view.selected_visible_index() == Some(index),
                    hovered_item == Some(index),
                    pressed_item == Some(index),
                ),
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
            .split(content_after_state_column(areas[4]));
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
    pub(crate) fn tab_index_at(&self, area: Rect, column: u16, row: u16) -> Option<usize> {
        if !self.show_tabs() {
            return None;
        }
        let content = content_area(area);
        if content.is_empty() {
            return None;
        }
        let tab_area = content_after_state_column(content);
        let tab_height = tab_list::desired_height(self.tabs(), tab_area.width);
        let areas = selection_areas(content, self, tab_height);
        self.tab_list()
            .index_at(content_after_state_column(areas[1]), column, row)
    }

    pub(crate) fn item_index_at(&self, area: Rect, column: u16, row: u16) -> Option<usize> {
        let content = content_area(area);
        if content.is_empty() {
            return None;
        }
        let tab_height = if self.show_tabs() {
            tab_list::desired_height(
                self.tabs(),
                content.width.saturating_sub(ITEM_STATE_COLUMN_WIDTH),
            )
        } else {
            0
        };
        let areas = selection_areas(content, self, tab_height);
        let visible_items = self.visible_items();
        let rendered_rows = usize::from(areas[3].height).min(visible_items.len());
        let first_row = self.first_rendered_row(rendered_rows);
        if column < areas[3].x
            || column >= areas[3].right()
            || row < areas[3].y
            || row >= areas[3].y.saturating_add(rendered_rows as u16)
        {
            return None;
        }
        let index = first_row.saturating_add(usize::from(row - areas[3].y));
        (index < visible_items.len()).then_some(index)
    }

    pub(crate) fn search_contains(&self, area: Rect, column: u16, row: u16) -> bool {
        if self.search().is_none() {
            return false;
        }
        let content = content_area(area);
        if content.is_empty() {
            return false;
        }
        let tab_height = if self.show_tabs() {
            tab_list::desired_height(
                self.tabs(),
                content.width.saturating_sub(ITEM_STATE_COLUMN_WIDTH),
            )
        } else {
            0
        };
        content_after_state_column(selection_areas(content, self, tab_height)[2])
            .contains(ratatui::layout::Position::new(column, row))
    }
}

fn content_area(area: Rect) -> Rect {
    area.inset(Insets::tlbr(0, 0, 0, 2))
}

fn content_after_state_column(area: Rect) -> Rect {
    Rect {
        x: area
            .x
            .saturating_add(ITEM_STATE_COLUMN_WIDTH.min(area.width)),
        width: area.width.saturating_sub(ITEM_STATE_COLUMN_WIDTH),
        ..area
    }
}

fn draw_focus_marker(frame: &mut Frame<'_>, area: Rect, context: RenderContext<'_>) {
    frame.render_widget(
        Paragraph::new(Span::styled("❯ ", focus_style(context))),
        area,
    );
}

fn item_attention(selected: bool, hovered: bool, pressed: bool) -> InteractionAttention {
    if selected {
        InteractionAttention::Keyboard
    } else if pressed {
        InteractionAttention::Pressed
    } else if hovered {
        InteractionAttention::Hovered
    } else {
        InteractionAttention::None
    }
}

fn search_attention(hovered: bool, pressed: bool) -> InteractionAttention {
    if pressed {
        InteractionAttention::Pressed
    } else if hovered {
        InteractionAttention::Hovered
    } else {
        InteractionAttention::None
    }
}

fn selection_areas(content: Rect, view: &ListSelectionState, tab_height: u16) -> Rc<[Rect]> {
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
            Constraint::Length(1),
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
    attention: InteractionAttention,
    column_layout: ItemColumnLayout,
    context: RenderContext<'_>,
) {
    let row_style = interaction_style(
        context,
        InteractionState {
            target: InteractionTarget::Rest,
            attention,
        },
    );
    frame.render_widget(Block::default().style(row_style), area);
    let label_style = if attention == InteractionAttention::Keyboard {
        item.selection_foreground()
            .map(|foreground| row_style.fg(foreground))
            .unwrap_or(row_style)
    } else if attention != InteractionAttention::None {
        row_style
    } else {
        Style::default()
    };
    let marker = if attention == InteractionAttention::Keyboard {
        "❯ "
    } else {
        "  "
    };
    let detail_style = if attention == InteractionAttention::None {
        Style::default().fg(context.muted())
    } else {
        row_style
    };
    let Some(columns) = item.columns() else {
        let spans = item_spans(item, marker, label_style, detail_style);
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
    label_style: Style,
    detail_style: Style,
) -> Vec<Span<'a>> {
    let Some(description) = item.description() else {
        return vec![
            Span::styled(marker, label_style),
            Span::styled(item.label(), label_style),
        ];
    };
    vec![
        Span::styled(marker, label_style),
        Span::styled(item.label(), label_style),
        Span::styled("  ·  ", detail_style),
        Span::styled(description, detail_style),
    ]
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
