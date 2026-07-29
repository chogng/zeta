use super::SelectionTab;
use super::SelectionViewState;
use crate::ui::HIGHLIGHT;
use crate::ui::MUTED;
use crate::ui::horizontal_margin;
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
use unicode_width::UnicodeWidthStr;

const TAB_GAP: usize = 2;

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, view: &SelectionViewState) {
    frame.render_widget(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(HIGHLIGHT)),
        area,
    );
    let content = horizontal_margin(
        Rect {
            y: area.y.saturating_add(1),
            height: area.height.saturating_sub(1),
            ..area
        },
        2,
    );
    if content.is_empty() {
        return;
    }

    let tab_lines = tab_lines(view.tabs(), view.active_tab_index(), content.width);
    let tab_height = tab_lines.len().min(u16::MAX as usize) as u16;
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(tab_height),
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(content);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            view.title(),
            Style::default().fg(HIGHLIGHT).add_modifier(Modifier::BOLD),
        ))),
        areas[0],
    );
    frame.render_widget(Paragraph::new(tab_lines), areas[1]);

    let search_text = if view.query().is_empty() {
        Span::styled(view.search_placeholder(), Style::default().fg(MUTED))
    } else {
        Span::raw(view.query())
    };
    frame.render_widget(
        Paragraph::new(Line::from(search_text)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MUTED)),
        ),
        areas[2],
    );

    let visible_items = view.visible_items();
    let rendered_rows = usize::from(areas[3].height).min(visible_items.len());
    let first_row = view.first_rendered_row(rendered_rows);
    let rows = if visible_items.is_empty() {
        vec![Line::from(Span::styled(
            view.empty_message(),
            Style::default().fg(MUTED),
        ))]
    } else {
        visible_items
            .iter()
            .enumerate()
            .skip(first_row)
            .take(rendered_rows)
            .map(|(index, item)| {
                let selected = view.selected_visible_index() == Some(index);
                let label_style = if selected {
                    Style::default().fg(HIGHLIGHT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let marker = if selected { "› " } else { "  " };
                let mut spans = vec![
                    Span::styled(marker, label_style),
                    Span::styled(item.label(), label_style),
                ];
                if let Some(description) = item.description() {
                    spans.push(Span::styled("  ·  ", Style::default().fg(MUTED)));
                    spans.push(Span::styled(description, Style::default().fg(MUTED)));
                }
                Line::from(spans)
            })
            .collect()
    };
    frame.render_widget(Paragraph::new(rows), areas[3]);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            view.footer_hint(),
            Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
        ))),
        areas[4],
    );
}

fn tab_lines(tabs: &[SelectionTab], active: usize, width: u16) -> Vec<Line<'static>> {
    if tabs.is_empty() {
        return Vec::new();
    }
    let available_width = usize::from(width.max(1));
    let mut lines = Vec::new();
    let mut spans = Vec::new();
    let mut row_width = 0usize;

    for (index, tab) in tabs.iter().enumerate() {
        let tab_width = tab.label().width().saturating_add(2);
        let gap = usize::from(!spans.is_empty()) * TAB_GAP;
        if !spans.is_empty()
            && row_width.saturating_add(gap).saturating_add(tab_width) > available_width
        {
            lines.push(Line::from(spans));
            spans = Vec::new();
            row_width = 0;
        }
        if !spans.is_empty() {
            spans.push(Span::raw("  "));
            row_width = row_width.saturating_add(TAB_GAP);
        }
        if index == active {
            spans.push(Span::styled(
                format!(" {} ", tab.label()),
                Style::default()
                    .fg(Color::Black)
                    .bg(HIGHLIGHT)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {} ", tab.label()),
                Style::default().fg(MUTED),
            ));
        }
        row_width = row_width.saturating_add(tab_width);
    }
    if !spans.is_empty() {
        lines.push(Line::from(spans));
    }
    lines
}
