//! Responsive empty-Thread welcome banner presentation.

use super::WelcomeModel;
use crate::ui::horizontal_margin;
use crate::ui::{accent, composer_chrome, muted};
use ratatui::Frame;
use ratatui::layout::Alignment;
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
use ratatui::widgets::Wrap;

const EXPANDED_MIN_WIDTH: u16 = 70;
const EXPANDED_HEIGHT: u16 = 11;
const COMPACT_HEIGHT: u16 = 12;

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &WelcomeModel,
    presentation_highlight: Color,
) {
    let available = horizontal_margin(area, 2);
    if available.is_empty() {
        return;
    }

    let expanded = available.width >= EXPANDED_MIN_WIDTH;
    let desired_height = if expanded {
        EXPANDED_HEIGHT
    } else {
        COMPACT_HEIGHT
    };
    let banner_area = Rect {
        y: available
            .y
            .saturating_add(u16::from(available.height > desired_height)),
        height: desired_height.min(available.height),
        ..available
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(presentation_highlight));
    let content = block.inner(banner_area);
    frame.render_widget(block, banner_area);
    if content.is_empty() {
        return;
    }

    let title_area = if expanded {
        let columns = expanded_columns(content);
        Rect {
            y: banner_area.y,
            height: 1,
            ..columns[0]
        }
    } else {
        Rect {
            y: banner_area.y,
            height: 1,
            ..content
        }
    };
    draw_title(frame, title_area);

    if expanded {
        draw_expanded(frame, content, model, presentation_highlight);
    } else {
        draw_compact(frame, content, model);
    }
}

fn draw_expanded(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &WelcomeModel,
    presentation_highlight: Color,
) {
    let columns = expanded_columns(area);
    frame.render_widget(
        Block::default()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(presentation_highlight)),
        columns[0],
    );
    let welcome_area = Rect {
        y: columns[0].y.saturating_add(1),
        height: columns[0].height.saturating_sub(1),
        ..columns[0]
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "Welcome back!",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::default(),
            Line::from(Span::styled("╭─────╮", Style::default().fg(accent()))),
            Line::from(vec![
                Span::styled("│  ", Style::default().fg(accent())),
                Span::styled(
                    "ζ",
                    Style::default().fg(accent()).add_modifier(Modifier::BOLD),
                ),
                Span::styled("  │", Style::default().fg(accent())),
            ]),
            Line::from(Span::styled("╰─┬─┬─╯", Style::default().fg(accent()))),
            Line::default(),
            Line::from(Span::styled(
                "Ready when you are",
                Style::default().fg(muted()),
            )),
            Line::from(Span::styled(
                model.directory(),
                Style::default().fg(muted()),
            )),
        ])
        .alignment(Alignment::Center),
        welcome_area,
    );

    let guide = horizontal_margin(columns[1], 2);
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .split(guide);
    frame.render_widget(
        Paragraph::new(vec![
            heading("Tips for getting started"),
            Line::from("Use @ to mention workspace files and / to discover commands."),
        ])
        .wrap(Wrap { trim: true }),
        sections[0],
    );
    let prompts = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(presentation_highlight));
    let prompts_content = prompts.inner(sections[1]);
    frame.render_widget(prompts, sections[1]);
    frame.render_widget(
        Paragraph::new(vec![
            heading("Try asking"),
            Line::from("“Explain how this workspace is structured.”"),
            Line::from("“Implement the change and run the relevant tests.”"),
        ])
        .wrap(Wrap { trim: true }),
        prompts_content,
    );
}

fn expanded_columns(area: Rect) -> [Rect; 2] {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);
    [columns[0], columns[1]]
}

fn draw_title(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled("Zeta Code", Style::default().fg(accent())),
            Span::styled(
                format!(" v{}", env!("CARGO_PKG_VERSION")),
                Style::default().fg(composer_chrome()),
            ),
            Span::raw(" "),
        ]))
        .alignment(Alignment::Center),
        area,
    );
}

fn draw_compact(frame: &mut Frame<'_>, area: Rect, model: &WelcomeModel) {
    let content = horizontal_margin(area, 1);
    let content = Rect {
        y: content.y.saturating_add(1),
        height: content.height.saturating_sub(1),
        ..content
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    "Welcome back!",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled("  ·  Ready when you are", Style::default().fg(muted())),
            ]),
            Line::from(Span::styled(
                model.directory(),
                Style::default().fg(muted()),
            )),
            Line::default(),
            heading("Tips for getting started"),
            Line::from("Use @ for workspace files and / for commands."),
            Line::default(),
            heading("Try asking"),
            Line::from("“Explain this workspace, then help me make a change.”"),
        ])
        .wrap(Wrap { trim: true }),
        content,
    );
}

fn heading(text: &'static str) -> Line<'static> {
    Line::from(Span::styled(
        text,
        Style::default().fg(accent()).add_modifier(Modifier::BOLD),
    ))
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
