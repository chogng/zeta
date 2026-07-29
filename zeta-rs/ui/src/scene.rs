/// An sRGB color with straight alpha.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

impl Color {
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);

    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self::rgba(red, green, blue, 255)
    }

    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    pub const fn components(self) -> [u8; 4] {
        [self.red, self.green, self.blue, self.alpha]
    }
}

/// A point in logical UI pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A size in logical UI pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

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
}

impl TextBlock {
    pub fn new(text: impl Into<String>, origin: Point, bounds: Size, style: TextStyle) -> Self {
        Self {
            text: text.into(),
            origin,
            bounds,
            style,
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
}

/// One immutable frame of native UI drawing input.
#[derive(Clone, Debug, PartialEq)]
pub struct UiScene {
    background: Color,
    text_blocks: Vec<TextBlock>,
}

impl UiScene {
    pub fn new(background: Color) -> Self {
        Self {
            background,
            text_blocks: Vec::new(),
        }
    }

    pub fn draw_text(&mut self, block: TextBlock) {
        self.text_blocks.push(block);
    }

    pub fn background(&self) -> Color {
        self.background
    }

    pub fn text_blocks(&self) -> &[TextBlock] {
        &self.text_blocks
    }
}

#[cfg(test)]
#[path = "scene_tests.rs"]
mod tests;
