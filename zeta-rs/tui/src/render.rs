use crate::app::App;
use crate::app::MessageRole;
use crate::app::Status;
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
use ratatui::widgets::BorderType;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;
use unicode_width::UnicodeWidthStr;

const ACCENT: Color = Color::Rgb(105, 170, 255);
const MUTED: Color = Color::DarkGray;
const SUCCESS: Color = Color::Rgb(95, 210, 140);
const WARNING: Color = Color::Rgb(245, 190, 80);
const DANGER: Color = Color::Rgb(245, 105, 105);

pub(crate) fn draw(frame: &mut Frame<'_>, app: &App) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(4),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    draw_header(frame, areas[0], app.status());
    draw_history(frame, areas[1], app);
    draw_composer(frame, areas[2], app);
    draw_footer(frame, areas[3], app.status());
}

fn draw_header(frame: &mut Frame<'_>, area: Rect, status: &Status) {
    let status_text = status_label(status);
    let status_width = status_text.width().min(u16::MAX as usize) as u16;
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(12),
            Constraint::Length(status_width.saturating_add(3)),
        ])
        .split(area);
    let divider = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(MUTED));
    frame.render_widget(divider, area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "  Zeta",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  workspace assistant", Style::default().fg(MUTED)),
        ])),
        columns[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("● ", Style::default().fg(status_color(status))),
            Span::styled(status_text, Style::default().fg(MUTED)),
        ]))
        .alignment(Alignment::Right),
        columns[1],
    );
}

fn draw_history(frame: &mut Frame<'_>, area: Rect, app: &App) {
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
    let history = history.scroll((scroll, 0));
    frame.render_widget(history, content_area);
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

fn draw_composer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let area = horizontal_margin(area, 2);
    let border_color = if app.accepts_input() { ACCENT } else { MUTED };
    let composer = Paragraph::new(Line::from(vec![
        Span::styled(
            "› ",
            Style::default()
                .fg(border_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(app.input()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color)),
    );
    frame.render_widget(composer, area);

    if app.accepts_input() {
        let input_width = app
            .input()
            .width()
            .min(area.width.saturating_sub(5) as usize) as u16;
        frame.set_cursor_position((area.x + 3 + input_width, area.y + 1));
    }
}

fn draw_footer(frame: &mut Frame<'_>, area: Rect, status: &Status) {
    let (text, style) = match status {
        Status::Ready => ("enter send  ·  esc quit", Style::default().fg(MUTED)),
        Status::Working => (
            "working…  ·  ctrl-c interrupt",
            Style::default().fg(WARNING),
        ),
        Status::WaitingForApproval => (
            "approval required  ·  ctrl-c interrupt",
            Style::default().fg(WARNING),
        ),
        Status::WaitingForUserInput => (
            "input required  ·  ctrl-c interrupt",
            Style::default().fg(WARNING),
        ),
        Status::WaitingForCapability => (
            "capability required  ·  ctrl-c interrupt",
            Style::default().fg(WARNING),
        ),
        Status::Cancelling => ("interrupting…", Style::default().fg(WARNING)),
        Status::Error => ("ready to retry  ·  esc quit", Style::default().fg(DANGER)),
    };
    frame.render_widget(
        Paragraph::new(text)
            .style(style)
            .alignment(Alignment::Center),
        area,
    );
}

fn status_label(status: &Status) -> &'static str {
    match status {
        Status::Ready => "ready",
        Status::Working => "working",
        Status::WaitingForApproval => "approval",
        Status::WaitingForUserInput => "waiting",
        Status::WaitingForCapability => "capability",
        Status::Cancelling => "stopping",
        Status::Error => "attention",
    }
}

fn status_color(status: &Status) -> Color {
    match status {
        Status::Ready => SUCCESS,
        Status::Working
        | Status::WaitingForApproval
        | Status::WaitingForUserInput
        | Status::WaitingForCapability
        | Status::Cancelling => WARNING,
        Status::Error => DANGER,
    }
}

fn horizontal_margin(area: Rect, margin: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(margin),
        width: area.width.saturating_sub(margin.saturating_mul(2)),
        ..area
    }
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
