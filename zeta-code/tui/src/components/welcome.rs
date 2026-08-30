//! Responsive empty-Thread welcome banner presentation.

use std::path::Path;

/// Display-only context for the empty-Thread welcome banner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WelcomeModel {
    directory: String,
}

impl WelcomeModel {
    pub(crate) fn for_workspace(workspace_root: &Path) -> Self {
        Self {
            directory: format_directory(workspace_root, dirs::home_dir().as_deref()),
        }
    }

    pub(crate) fn directory(&self) -> &str {
        &self.directory
    }
}

fn format_directory(directory: &Path, home: Option<&Path>) -> String {
    if let Some(home) = home
        && let Ok(relative) = directory.strip_prefix(home)
    {
        return if relative.as_os_str().is_empty() {
            "~".into()
        } else {
            format!("~{}{}", std::path::MAIN_SEPARATOR, relative.display())
        };
    }
    directory.display().to_string()
}

use crate::render::RenderContext;
use crate::render::horizontal_margin;
use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
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

pub(crate) fn desired_height(available_width: u16) -> u16 {
    if horizontal_margin(Rect::new(0, 0, available_width, u16::MAX), 2).width >= EXPANDED_MIN_WIDTH
    {
        EXPANDED_HEIGHT
    } else {
        COMPACT_HEIGHT
    }
}

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &WelcomeModel,
    context: RenderContext<'_>,
) {
    let available = horizontal_margin(area, 2);
    if available.is_empty() {
        return;
    }

    let expanded = available.width >= EXPANDED_MIN_WIDTH;
    let desired_height = desired_height(area.width);
    let banner_area = Rect {
        y: available
            .y
            .saturating_add(u16::from(available.height > desired_height)),
        height: desired_height.min(available.height),
        ..available
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(context.accent()));
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
    draw_title(frame, title_area, context);

    if expanded {
        draw_expanded(frame, content, model, context);
    } else {
        draw_compact(frame, content, model, context);
    }
}

fn draw_expanded(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &WelcomeModel,
    context: RenderContext<'_>,
) {
    let columns = expanded_columns(area);
    frame.render_widget(
        Block::default()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(context.accent())),
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
            Line::from(Span::styled(
                "╭─────╮",
                Style::default().fg(context.accent()),
            )),
            Line::from(vec![
                Span::styled("│  ", Style::default().fg(context.accent())),
                Span::styled(
                    "ζ",
                    Style::default()
                        .fg(context.accent())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  │", Style::default().fg(context.accent())),
            ]),
            Line::from(Span::styled(
                "╰─┬─┬─╯",
                Style::default().fg(context.accent()),
            )),
            Line::default(),
            Line::from(Span::styled(
                "Ready when you are",
                Style::default().fg(context.muted()),
            )),
            Line::from(Span::styled(
                model.directory(),
                Style::default().fg(context.muted()),
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
            heading("Tips for getting started", context),
            Line::from("Use @ to mention workspace files and / to discover commands."),
        ])
        .wrap(Wrap { trim: true }),
        sections[0],
    );
    let prompts = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(context.accent()));
    let prompts_content = prompts.inner(sections[1]);
    frame.render_widget(prompts, sections[1]);
    frame.render_widget(
        Paragraph::new(vec![
            heading("Try asking", context),
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

fn draw_title(frame: &mut Frame<'_>, area: Rect, context: RenderContext<'_>) {
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled("Zeta Code", Style::default().fg(context.accent())),
            Span::styled(
                format!(" v{}", env!("CARGO_PKG_VERSION")),
                Style::default().fg(context.chat_input_chrome()),
            ),
            Span::raw(" "),
        ]))
        .alignment(Alignment::Center),
        area,
    );
}

fn draw_compact(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &WelcomeModel,
    context: RenderContext<'_>,
) {
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
                Span::styled(
                    "  ·  Ready when you are",
                    Style::default().fg(context.muted()),
                ),
            ]),
            Line::from(Span::styled(
                model.directory(),
                Style::default().fg(context.muted()),
            )),
            Line::default(),
            heading("Tips for getting started", context),
            Line::from("Use @ for workspace files and / for commands."),
            Line::default(),
            heading("Try asking", context),
            Line::from("“Explain this workspace, then help me make a change.”"),
        ])
        .wrap(Wrap { trim: true }),
        content,
    );
}

fn heading(text: &'static str, context: RenderContext<'_>) -> Line<'static> {
    Line::from(Span::styled(
        text,
        Style::default()
            .fg(context.accent())
            .add_modifier(Modifier::BOLD),
    ))
}

#[cfg(test)]
#[path = "welcome_model_tests.rs"]
mod model_tests;

#[cfg(test)]
#[path = "welcome_view_tests.rs"]
mod view_tests;
