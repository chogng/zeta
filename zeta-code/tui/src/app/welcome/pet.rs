//! Embedded terminal pet and its Ratatui widget.

mod asset {
    include!(concat!(env!("OUT_DIR"), "/welcome_pet.rs"));
}

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;
use zeta_sprite::Rgb;
use zeta_sprite::TerminalSprite;

pub(super) struct PetWidget<'a> {
    sprite: &'a TerminalSprite<'static>,
}

impl<'a> PetWidget<'a> {
    pub(super) const fn new(sprite: &'a TerminalSprite<'static>) -> Self {
        Self { sprite }
    }
}

impl Widget for PetWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let width = self.sprite.width().min(area.width);
        let height = self.sprite.height().min(area.height);
        for y in 0..height {
            for x in 0..width {
                let source = self.sprite.cells()
                    [usize::from(y) * usize::from(self.sprite.width()) + usize::from(x)];
                if source.symbol() == ' ' {
                    continue;
                }
                let cell = &mut buffer[(area.x + x, area.y + y)];
                cell.set_char(source.symbol());
                if let Some(color) = source.foreground() {
                    cell.set_fg(ratatui_color(color));
                }
                if let Some(color) = source.background() {
                    cell.set_bg(ratatui_color(color));
                }
            }
        }
    }
}

fn ratatui_color(color: Rgb) -> Color {
    let [red, green, blue] = color.components();
    Color::Rgb(red, green, blue)
}

pub(super) fn sprite() -> &'static TerminalSprite<'static> {
    &asset::PET
}
