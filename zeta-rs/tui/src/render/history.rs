use super::layout::horizontal_margin;
use super::theme::ACCENT;
use super::theme::DANGER;
use super::theme::MUTED;
use super::theme::SUCCESS;
use super::theme::WARNING;
use crate::app::App;
use crate::app::MessageRole;
use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;
use unicode_width::UnicodeWidthStr;

pub(super) fn draw(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let content_area = horizontal_margin(area, 2);
    if app.messages().is_empty() {
        let welcome_area = Rect {
            y: content_area.y.saturating_add(content_area.height / 3),
            height: content_area.height.saturating_sub(content_area.height / 3),
            ..content_area
        };
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    "Zeta",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    "Ask anything about your workspace.",
                    Style::default().fg(MUTED),
                )),
            ])
            .alignment(Alignment::Center),
            welcome_area,
        );
        return;
    }

    let history_width = content_area.width as usize;
    let history_height = content_area.height as usize;
    let history_rows = app
        .messages()
        .iter()
        .map(|message| estimated_wrapped_rows(3, &message.text, history_width).saturating_add(1))
        .sum::<usize>();
    let messages = message_lines(app);
    let history = Paragraph::new(messages).wrap(Wrap { trim: false });
    let scroll = history_rows
        .saturating_sub(history_height)
        .min(u16::MAX as usize) as u16;
    frame.render_widget(history.scroll((scroll, 0)), content_area);
}

fn message_lines(app: &App) -> Vec<Line<'_>> {
    app.messages()
        .iter()
        .flat_map(|message| {
            let (marker, color) = match message.role {
                MessageRole::User => ("›", ACCENT),
                MessageRole::Agent => ("◆", SUCCESS),
                MessageRole::Notice => ("•", WARNING),
                MessageRole::Error => ("×", DANGER),
            };
            [
                Line::from(vec![
                    Span::styled(
                        format!("{marker}  "),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(&message.text),
                ]),
                Line::default(),
            ]
        })
        .collect()
}

pub(super) fn estimated_wrapped_rows(
    label_width: usize,
    text: &str,
    available_width: usize,
) -> usize {
    if available_width == 0 {
        return 0;
    }
    text.lines()
        .enumerate()
        .map(|(index, line)| {
            let prefix_width = if index == 0 { label_width } else { 0 };
            (prefix_width + line.width())
                .div_ceil(available_width)
                .max(1)
        })
        .sum::<usize>()
        .max(1)
}
