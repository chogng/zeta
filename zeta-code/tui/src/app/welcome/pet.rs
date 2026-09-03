//! Terminal pet data model and Ratatui widget.

mod asset;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SpriteCell {
    symbol: &'static str,
    foreground: Option<Color>,
    background: Option<Color>,
}

impl SpriteCell {
    pub(super) const fn new(
        symbol: &'static str,
        foreground: Option<Color>,
        background: Option<Color>,
    ) -> Self {
        Self {
            symbol,
            foreground,
            background,
        }
    }

    pub(super) const fn transparent() -> Self {
        Self::new("", None, None)
    }

    pub(super) const fn symbol(self) -> &'static str {
        self.symbol
    }

    pub(super) const fn foreground(self) -> Option<Color> {
        self.foreground
    }

    pub(super) const fn background(self) -> Option<Color> {
        self.background
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct PetSprite {
    width: u16,
    height: u16,
    cells: &'static [SpriteCell],
}

impl PetSprite {
    pub(super) const fn new(width: u16, height: u16, cells: &'static [SpriteCell]) -> Self {
        assert!(cells.len() == width as usize * height as usize);
        Self {
            width,
            height,
            cells,
        }
    }

    pub(super) const fn width(&self) -> u16 {
        self.width
    }

    pub(super) const fn height(&self) -> u16 {
        self.height
    }

    pub(super) const fn cells(&self) -> &'static [SpriteCell] {
        self.cells
    }
}

pub(super) struct PetWidget<'a> {
    sprite: &'a PetSprite,
}

impl<'a> PetWidget<'a> {
    pub(super) const fn new(sprite: &'a PetSprite) -> Self {
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
                if source.symbol().is_empty() {
                    continue;
                }
                let cell = &mut buffer[(area.x + x, area.y + y)];
                cell.set_symbol(source.symbol());
                if let Some(color) = source.foreground() {
                    cell.set_fg(color);
                }
                if let Some(color) = source.background() {
                    cell.set_bg(color);
                }
            }
        }
    }
}

pub(super) fn sprite() -> &'static PetSprite {
    &asset::PET
}
