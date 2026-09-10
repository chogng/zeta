use crate::app::App;
use crate::render::RenderContext;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;

pub(super) fn draw(frame: &mut Frame<'_>, area: Rect, app: &App, context: RenderContext<'_>) {
    let branch = app.status_line().branch_label();
    let mut spans = vec![Span::styled(
        "Home",
        Style::default().fg(context.foreground()),
    )];
    if let Some(branch) = branch {
        spans.push(Span::raw(format!("  {branch}")));
    }
    spans.push(Span::raw(format!("  {}", app.welcome().directory())));
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().fg(context.muted())),
        area,
    );
}

#[cfg(test)]
#[path = "header_tests.rs"]
mod tests;
