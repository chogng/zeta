use super::SteerView;
use crate::ui::background;
use crate::ui::highlight;
use crate::ui::muted;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;

const MAX_VISIBLE_ITEMS: usize = 3;

pub(crate) fn desired_height(view: &SteerView<'_>) -> u16 {
    u16::try_from(view.items.len().min(MAX_VISIBLE_ITEMS).saturating_add(2)).unwrap_or(u16::MAX)
}

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, view: &SteerView<'_>) {
    let lines = view
        .items
        .iter()
        .take(MAX_VISIBLE_ITEMS)
        .map(|item| Line::from(format!("↳ {item}")))
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(format!("Steer  {} sending", view.items.len()))
                .title_style(
                    Style::default()
                        .fg(highlight())
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(muted()))
                .style(Style::default().bg(background())),
        ),
        area,
    );
}
