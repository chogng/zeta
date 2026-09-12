use crate::app::App;
use crate::render::RenderContext;
use crate::render::interaction_style;
use ratatui::Frame;
use ratatui::layout::Position;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use zeta_memory_diagnostics::ProcessResourceDemand;

pub(super) fn draw(frame: &mut Frame<'_>, area: Rect, app: &App, context: RenderContext<'_>) {
    let home = super::pointer::PointerTarget::Home;
    frame.render_widget(
        Paragraph::new("≡").style(interaction_style(
            context,
            app.fullscreen.pointer.interaction_state(&home),
        )),
        menu_area(area),
    );
    let status = crate::status::header_line(
        app.status_line(),
        usize::from(status_width(area)),
        app.status_line_runtime(),
        context,
    );
    let width = status.width() as u16;
    let gap = if width > 0 { 2 } else { 0 };
    let workspace = Rect::new(
        area.x + 2.min(area.width),
        area.y,
        area.width.saturating_sub(2 + width + gap),
        area.height,
    );
    let mut spans = Vec::new();
    let mut path_width = usize::from(workspace.width);
    if let Some(branch) = app.status_line().branch_label() {
        let branch = crate::render::truncate_with_ellipsis(branch, path_width / 2);
        path_width =
            path_width.saturating_sub(unicode_width::UnicodeWidthStr::width(branch.as_str()) + 1);
        spans.push(Span::styled(
            branch,
            Style::default()
                .fg(context.foreground())
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));
    }
    spans.push(Span::raw(crate::render::truncate_with_ellipsis(
        app.welcome().directory(),
        path_width,
    )));
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().fg(context.muted())),
        workspace,
    );
    frame.render_widget(
        Paragraph::new(status),
        Rect::new(
            area.right().saturating_sub(width),
            area.y,
            width,
            area.height,
        ),
    );
}

fn menu_area(area: Rect) -> Rect {
    Rect {
        width: area.width.min(2),
        ..area
    }
}

pub(super) fn home_at(area: Rect, position: Position) -> bool {
    menu_area(area).contains(position)
}

fn status_width(area: Rect) -> u16 {
    if area.is_empty() {
        0
    } else {
        area.width.saturating_sub(16) / 2
    }
}

pub(super) fn process_resource_demand(app: &App, area: Rect) -> ProcessResourceDemand {
    app.status_line()
        .header_process_resources(usize::from(status_width(area)), app.status_line_runtime())
        .map_or(
            ProcessResourceDemand::Disabled,
            ProcessResourceDemand::Summary,
        )
}

#[cfg(test)]
#[path = "header_tests.rs"]
mod tests;
