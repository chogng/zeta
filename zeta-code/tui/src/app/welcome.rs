//! Responsive Thread identity header.

mod pet;

use crate::models::ModelSummary;
use crate::models::access_label;
use crate::render::RenderContext;
use crate::render::horizontal_margin;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use std::path::Path;
use zeta_protocol::ModelAccess;

const INFO_ROWS: u16 = 3;
const PET_GAP: u16 = 3;
const MIN_INFO_WIDTH_WITH_PET: u16 = 12;

/// Display-only context for the Thread identity header.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WelcomeModel {
    directory: String,
    model: String,
    access: ModelAccess,
}

impl WelcomeModel {
    pub(crate) fn for_workspace(workspace_root: &Path) -> Self {
        Self {
            directory: format_directory(workspace_root, dirs::home_dir().as_deref()),
            model: "Automatic model".into(),
            access: ModelAccess::Unknown,
        }
    }

    pub(crate) fn apply_model_summary(&mut self, summary: &ModelSummary) {
        let model = summary.model_label();
        if self.model != model {
            self.model = model;
            self.access = summary.access();
        } else if summary.access() != ModelAccess::Unknown {
            self.access = summary.access();
        }
    }

    pub(crate) fn directory(&self) -> &str {
        &self.directory
    }

    fn model_line(&self) -> String {
        format!("{} · {}", self.model, access_label(self.access))
    }
}

pub(crate) fn desired_height(_available_width: u16) -> u16 {
    pet::sprite().height().max(INFO_ROWS)
}

pub(crate) fn history_height(available_height: u16) -> u16 {
    desired_height(0).saturating_add(1).min(available_height)
}

pub(crate) fn history_buffer(
    width: u16,
    height: u16,
    model: &WelcomeModel,
    context: RenderContext<'_>,
) -> Buffer {
    let area = Rect::new(0, 0, width, history_height(height));
    let mut buffer = Buffer::empty(area);
    render(&mut buffer, area, model, context);
    buffer
}

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &WelcomeModel,
    context: RenderContext<'_>,
) {
    render(frame.buffer_mut(), area, model, context);
}

fn render(buffer: &mut Buffer, area: Rect, model: &WelcomeModel, context: RenderContext<'_>) {
    let available = horizontal_margin(area, 2);
    if available.is_empty() {
        return;
    }
    let sprite = pet::sprite();
    let show_pet = available.height >= sprite.height()
        && available.width
            >= sprite
                .width()
                .saturating_add(PET_GAP)
                .saturating_add(MIN_INFO_WIDTH_WITH_PET);
    let content_height = if show_pet { sprite.height() } else { INFO_ROWS };
    let content_y = available
        .y
        .saturating_add(u16::from(available.height > content_height));
    let text_x = if show_pet {
        let pet_area = Rect::new(available.x, content_y, sprite.width(), sprite.height());
        pet::PetWidget::new(sprite).render(pet_area, buffer);
        pet_area.right().saturating_add(PET_GAP)
    } else {
        available.x
    };
    let text_area = Rect::new(
        text_x,
        content_y,
        available.right().saturating_sub(text_x),
        INFO_ROWS.min(available.height),
    );
    Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "Zeta Code",
                Style::default()
                    .fg(context.foreground())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" v{}", env!("CARGO_PKG_VERSION")),
                Style::default().fg(context.muted()),
            ),
        ]),
        Line::from(Span::styled(
            model.model_line(),
            Style::default().fg(context.muted()),
        )),
        Line::from(Span::styled(
            model.directory(),
            Style::default().fg(context.muted()),
        )),
    ])
    .render(text_area, buffer);
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

#[cfg(test)]
#[path = "welcome_model_tests.rs"]
mod model_tests;

#[cfg(test)]
#[path = "welcome_view_tests.rs"]
mod view_tests;
