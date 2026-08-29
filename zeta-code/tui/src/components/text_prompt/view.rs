use super::TextPrompt;
use crate::components::search_box;
use crate::ui::highlight;
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, prompt: &TextPrompt) {
    let content = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(3)])
        .split(area);
    frame.render_widget(
        Paragraph::new(prompt.explanation()).block(
            Block::default()
                .title(prompt.title())
                .borders(Borders::TOP)
                .border_style(Style::default().fg(highlight())),
        ),
        content[0],
    );
    search_box::draw(frame, content[1], prompt.input(), highlight());
}
