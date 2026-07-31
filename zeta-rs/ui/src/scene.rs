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

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
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

/// One owned text run with a uniform style inside a rich [`TextBlock`].
///
/// Callers should split spans only where presentation changes. The renderer shapes every span in
/// the same paragraph buffer so wrapping, bidirectional text, and fallback remain coordinated.
#[derive(Clone, Debug, PartialEq)]
pub struct TextSpan {
    text: String,
    style: TextStyle,
}

impl TextSpan {
    pub fn new(text: impl Into<String>, style: TextStyle) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub const fn style(&self) -> &TextStyle {
        &self.style
    }
}

/// Horizontal overflow behavior for one shaped [`TextBlock`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TextBlockWrap {
    /// Wrap at word or glyph boundaries when text exceeds the block width.
    #[default]
    WordOrGlyph,
    /// Keep every source line unwrapped and rely on the scene clip for overflow.
    None,
}

/// A shaped-on-demand block of text placed in logical UI coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct TextBlock {
    text: String,
    spans: Vec<TextSpan>,
    origin: Point,
    bounds: Size,
    style: TextStyle,
    wrap: TextBlockWrap,
    clip_bounds: Option<Rect>,
}

impl TextBlock {
    pub fn new(text: impl Into<String>, origin: Point, bounds: Size, style: TextStyle) -> Self {
        Self {
            text: text.into(),
            spans: Vec::new(),
            origin,
            bounds,
            style,
            wrap: TextBlockWrap::WordOrGlyph,
            clip_bounds: None,
        }
    }

    /// Creates one paragraph from differently styled spans.
    ///
    /// `style` supplies the paragraph's default metrics and fallback attributes. Individual spans
    /// may override font metrics, family, weight, style, and color.
    pub fn from_spans(
        spans: impl IntoIterator<Item = TextSpan>,
        origin: Point,
        bounds: Size,
        style: TextStyle,
    ) -> Self {
        let spans = spans.into_iter().collect::<Vec<_>>();
        let text = spans.iter().map(TextSpan::text).collect::<String>();
        Self {
            text,
            spans,
            origin,
            bounds,
            style,
            wrap: TextBlockWrap::WordOrGlyph,
            clip_bounds: None,
        }
    }

    pub const fn with_wrap(mut self, wrap: TextBlockWrap) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn spans(&self) -> &[TextSpan] {
        &self.spans
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

    pub const fn wrap(&self) -> TextBlockWrap {
        self.wrap
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
    rect_layers: Vec<usize>,
    icons: Vec<PaintIcon>,
    icon_layers: Vec<usize>,
    images: Vec<crate::PaintImage>,
    image_layers: Vec<usize>,
    text_blocks: Vec<TextBlock>,
    text_layers: Vec<usize>,
    active_clip: Option<Rect>,
    active_layer: usize,
    layer_count: usize,
}

impl UiScene {
    pub fn new(background: Color) -> Self {
        Self {
            background,
            rects: Vec::new(),
            rect_layers: Vec::new(),
            icons: Vec::new(),
            icon_layers: Vec::new(),
            images: Vec::new(),
            image_layers: Vec::new(),
            text_blocks: Vec::new(),
            text_layers: Vec::new(),
            active_clip: None,
            active_layer: 0,
            layer_count: 1,
        }
    }

    pub fn draw_rect(&mut self, mut rect: PaintRect) {
        if let Some(clip_bounds) = self.active_clip {
            rect.apply_clip(clip_bounds);
        }
        self.rects.push(rect);
        self.rect_layers.push(self.active_layer);
    }

    pub fn draw_icon(&mut self, mut icon: PaintIcon) {
        if let Some(clip_bounds) = self.active_clip {
            icon.apply_clip(clip_bounds);
        }
        self.icons.push(icon);
        self.icon_layers.push(self.active_layer);
    }

    pub fn draw_image(&mut self, mut image: crate::PaintImage) {
        if let Some(clip_bounds) = self.active_clip {
            image.apply_clip(clip_bounds);
        }
        self.images.push(image);
        self.image_layers.push(self.active_layer);
    }

    pub fn draw_text(&mut self, mut block: TextBlock) {
        if let Some(clip_bounds) = self.active_clip {
            block.apply_clip(clip_bounds);
        }
        self.text_blocks.push(block);
        self.text_layers.push(self.active_layer);
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

    /// Draws into a new layer composited above every previously created scene layer.
    ///
    /// The new layer does not inherit the caller's active clip, allowing anchored views to escape
    /// their host component. Nested overlays receive their own later layer. After the closure
    /// returns, drawing resumes with the caller's layer and clip.
    pub fn with_overlay<R>(&mut self, draw: impl FnOnce(&mut Self) -> R) -> R {
        let previous_layer = self.active_layer;
        let previous_clip = self.active_clip;
        self.active_layer = self.layer_count;
        self.active_clip = None;
        self.layer_count += 1;
        let result = draw(self);
        self.active_layer = previous_layer;
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

    pub fn images(&self) -> &[crate::PaintImage] {
        &self.images
    }

    pub fn text_blocks(&self) -> &[TextBlock] {
        &self.text_blocks
    }

    pub(crate) const fn layer_count(&self) -> usize {
        self.layer_count
    }

    pub(crate) fn rect_layers(&self) -> &[usize] {
        &self.rect_layers
    }

    pub(crate) fn icon_layers(&self) -> &[usize] {
        &self.icon_layers
    }

    pub(crate) fn image_layers(&self) -> &[usize] {
        &self.image_layers
    }

    pub(crate) fn text_layers(&self) -> &[usize] {
        &self.text_layers
    }
}

#[cfg(test)]
#[path = "scene_tests.rs"]
mod tests;
