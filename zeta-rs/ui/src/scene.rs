use crate::{Color, Component, PaintIcon, PaintRect, Point, Rect, Size};

/// The font-family selection requested by a text block.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum FontFamily {
    SansSerif,
    Serif,
    Monospace,
    Named(String),
}

/// The supported semantic font weights.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum FontWeight {
    #[default]
    Normal,
    Bold,
}

/// The supported semantic font styles.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
}

/// Text appearance independent of a concrete shaping or GPU backend.
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    family: FontFamily,
    font_size: f32,
    line_height: f32,
    color: Color,
    weight: FontWeight,
    style: FontStyle,
}

impl TextStyle {
    pub fn new(font_size: f32, color: Color) -> Self {
        Self {
            family: FontFamily::SansSerif,
            font_size,
            line_height: font_size * 1.2,
            color,
            weight: FontWeight::Normal,
            style: FontStyle::Normal,
        }
    }

    pub fn with_family(mut self, family: FontFamily) -> Self {
        self.family = family;
        self
    }

    pub fn with_line_height(mut self, line_height: f32) -> Self {
        self.line_height = line_height;
        self
    }

    pub fn with_weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    pub fn with_style(mut self, style: FontStyle) -> Self {
        self.style = style;
        self
    }

    pub fn family(&self) -> &FontFamily {
        &self.family
    }

    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    pub fn color(&self) -> Color {
        self.color
    }

    pub fn weight(&self) -> FontWeight {
        self.weight
    }

    pub fn style(&self) -> FontStyle {
        self.style
    }
}

/// A shaped-on-demand block of text placed in logical UI coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct TextBlock {
    text: String,
    origin: Point,
    bounds: Size,
    style: TextStyle,
    clip_bounds: Option<Rect>,
}

impl TextBlock {
    pub fn new(text: impl Into<String>, origin: Point, bounds: Size, style: TextStyle) -> Self {
        Self {
            text: text.into(),
            origin,
            bounds,
            style,
            clip_bounds: None,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn origin(&self) -> Point {
        self.origin
    }

    pub fn bounds(&self) -> Size {
        self.bounds
    }

    pub fn style(&self) -> &TextStyle {
        &self.style
    }

    pub(crate) const fn clip_bounds(&self) -> Option<Rect> {
        self.clip_bounds
    }

    fn apply_clip(&mut self, clip_bounds: Rect) {
        self.clip_bounds = Some(match self.clip_bounds {
            Some(current) => current.intersection(clip_bounds),
            None => clip_bounds,
        });
    }
}

/// One immutable frame of native UI drawing input.
#[derive(Clone, Debug, PartialEq)]
pub struct UiScene {
    background: Color,
    rects: Vec<PaintRect>,
    icons: Vec<PaintIcon>,
    text_blocks: Vec<TextBlock>,
    active_clip: Option<Rect>,
}

impl UiScene {
    pub fn new(background: Color) -> Self {
        Self {
            background,
            rects: Vec::new(),
            icons: Vec::new(),
            text_blocks: Vec::new(),
            active_clip: None,
        }
    }

    pub fn draw_rect(&mut self, mut rect: PaintRect) {
        if let Some(clip_bounds) = self.active_clip {
            rect.apply_clip(clip_bounds);
        }
        self.rects.push(rect);
    }

    pub fn draw_icon(&mut self, mut icon: PaintIcon) {
        if let Some(clip_bounds) = self.active_clip {
            icon.apply_clip(clip_bounds);
        }
        self.icons.push(icon);
    }

    pub fn draw_text(&mut self, mut block: TextBlock) {
        if let Some(clip_bounds) = self.active_clip {
            block.apply_clip(clip_bounds);
        }
        self.text_blocks.push(block);
    }

    pub fn draw_component<C: Component + ?Sized>(&mut self, component: &C) {
        component.paint(self);
    }

    pub fn with_clip<R>(&mut self, clip_bounds: Rect, draw: impl FnOnce(&mut Self) -> R) -> R {
        let previous_clip = self.active_clip;
        self.active_clip = Some(match previous_clip {
            Some(current) => current.intersection(clip_bounds),
            None => clip_bounds,
        });
        let result = draw(self);
        self.active_clip = previous_clip;
        result
    }

    pub fn background(&self) -> Color {
        self.background
    }

    pub fn rects(&self) -> &[PaintRect] {
        &self.rects
    }

    pub fn icons(&self) -> &[PaintIcon] {
        &self.icons
    }

    pub fn text_blocks(&self) -> &[TextBlock] {
        &self.text_blocks
    }
}

#[cfg(test)]
#[path = "scene_tests.rs"]
mod tests;
