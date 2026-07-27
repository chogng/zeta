use crate::app::App;
use crate::app::MessageRole;
use crate::app::Status;
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;
use unicode_width::UnicodeWidthStr;

pub(crate) fn draw(frame: &mut Frame<'_>, app: &App) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(4),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let history_width = areas[0].width.saturating_sub(2) as usize;
    let history_height = areas[0].height.saturating_sub(2) as usize;
    let history_rows = app
        .messages()
        .iter()
        .map(|message| {
            let label_width = match message.role {
                MessageRole::User => "You: ".width(),
                MessageRole::Agent => "Zeta: ".width(),
                MessageRole::Notice => "Note: ".width(),
                MessageRole::Error => "Error: ".width(),
            };
            estimated_wrapped_rows(label_width, &message.text, history_width)
        })
        .sum::<usize>();
    let messages = app
        .messages()
        .iter()
        .map(|message| {
            let (label, color) = match message.role {
                MessageRole::User => ("You", Color::Cyan),
                MessageRole::Agent => ("Zeta", Color::Green),
                MessageRole::Notice => ("Note", Color::Yellow),
                MessageRole::Error => ("Error", Color::Red),
            };
            Line::from(vec![
                Span::styled(
                    format!("{label}: "),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(&message.text),
            ])
        })
        .collect::<Vec<_>>();
    let history = Paragraph::new(messages)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Zeta ")
                .title_style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .wrap(Wrap { trim: false });
    let scroll = history_rows
        .saturating_sub(history_height)
        .min(u16::MAX as usize) as u16;
    let history = history.scroll((scroll, 0));
    frame.render_widget(history, areas[0]);

    let input = Paragraph::new(app.input())
        .block(Block::default().borders(Borders::ALL).title(" Message "));
    frame.render_widget(input, areas[1]);

    let (status, style) = match app.status() {
        Status::Ready => ("Enter send  •  Esc quit".to_owned(), Style::default()),
        Status::Working => (
            "Working… Ctrl-C interrupts this turn".to_owned(),
            Style::default().fg(Color::Yellow),
        ),
        Status::WaitingForApproval => (
            "Waiting for approval… this TUI cannot resolve it yet; Ctrl-C interrupts".to_owned(),
            Style::default().fg(Color::Yellow),
        ),
        Status::WaitingForUserInput => (
            "Waiting for user input… this TUI cannot resolve it yet; Ctrl-C interrupts".to_owned(),
            Style::default().fg(Color::Yellow),
        ),
        Status::WaitingForCapability => (
            "Waiting for a capability… this TUI cannot resolve it yet; Ctrl-C interrupts"
                .to_owned(),
            Style::default().fg(Color::Yellow),
        ),
        Status::Cancelling => (
            "Interrupting turn… waiting for its terminal state".to_owned(),
            Style::default().fg(Color::Yellow),
        ),
        Status::Error(message) => (
            format!("Error: {message}  •  type another message or Esc quit"),
            Style::default().fg(Color::Red),
        ),
    };
    frame.render_widget(Paragraph::new(status).style(style), areas[2]);

    let input_width = app
        .input()
        .width()
        .min(areas[1].width.saturating_sub(2) as usize) as u16;
    frame.set_cursor_position((areas[1].x + 1 + input_width, areas[1].y + 1));
}

fn estimated_wrapped_rows(label_width: usize, text: &str, available_width: usize) -> usize {
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

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
