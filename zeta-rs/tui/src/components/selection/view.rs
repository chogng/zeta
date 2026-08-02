use super::SelectionTab;
use super::SelectionViewState;
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
use unicode_width::UnicodeWidthStr;

const TAB_GAP: usize = 2;

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, view: &SelectionViewState) {
    let presentation_highlight = view.presentation_highlight().unwrap_or_else(highlight);
    frame.render_widget(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(presentation_highlight)),
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

    let tab_lines = if view.show_tabs() {
        tab_lines(
            view.tabs(),
            view.active_tab_index(),
            content.width,
            presentation_highlight,
        )
    } else {
        Vec::new()
    };
    let tab_height = tab_lines.len().min(u16::MAX as usize) as u16;
    let search_height = if view.search_active() { 3 } else { 0 };
    let preview_height = view
        .selected_item()
        .and_then(|item| item.preview())
        .map(|preview| preview.desired_height())
        .unwrap_or_default()
        .min(u16::MAX as usize) as u16;
    let title_top_margin = view.title_top_margin().min(u16::MAX as usize) as u16;
    let title_bottom_margin = view.title_bottom_margin().min(u16::MAX as usize) as u16;
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(title_top_margin),
            Constraint::Length(1),
            Constraint::Length(title_bottom_margin),
            Constraint::Length(tab_height),
            Constraint::Length(search_height),
            Constraint::Min(1),
            Constraint::Length(preview_height),
            Constraint::Length(1),
        ])
        .split(content);

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
        frame.render_widget(Paragraph::new(tab_lines), areas[3]);
    }

    if view.search_active() {
        let search_text = if view.query().is_empty() {
            Span::styled(view.search_placeholder(), Style::default().fg(muted()))
        } else {
            Span::raw(view.query())
        };
        frame.render_widget(
            Paragraph::new(Line::from(search_text)).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(muted())),
            ),
            areas[4],
        );
    }

    let visible_items = view.visible_items();
    let rendered_rows = usize::from(areas[5].height).min(visible_items.len());
    let first_row = view.first_rendered_row(rendered_rows);
    let rows = if visible_items.is_empty() {
        vec![Line::from(Span::styled(
            view.empty_message(),
            Style::default().fg(muted()),
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
                    Style::default()
                        .fg(item.selection_foreground().unwrap_or_else(highlight))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let marker = if selected { "› " } else { "  " };
                let mut spans = vec![
                    Span::styled(marker, label_style),
                    Span::styled(item.label(), label_style),
                ];
                if let Some(description) = item.description() {
                    spans.push(Span::styled("  ·  ", Style::default().fg(muted())));
                    spans.push(Span::styled(description, Style::default().fg(muted())));
                }
                Line::from(spans)
            })
            .collect()
    };
    frame.render_widget(Paragraph::new(rows), areas[5]);
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
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            view.footer_hint(),
            Style::default().fg(muted()).add_modifier(Modifier::ITALIC),
        ))),
        areas[7],
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

fn tab_lines(
    tabs: &[SelectionTab],
    active: usize,
    width: u16,
    presentation_highlight: Color,
) -> Vec<Line<'static>> {
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
                    .bg(presentation_highlight)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {} ", tab.label()),
                Style::default().fg(muted()),
            ));
        }
        row_width = row_width.saturating_add(tab_width);
    }
    if !spans.is_empty() {
        lines.push(Line::from(spans));
    }
    lines
}
