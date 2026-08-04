use glyphon::{
    Attrs, Buffer, Cache, Color as GlyphColor, Metrics, Resolution, Shaping, SwashCache, TextArea,
    TextAtlas, TextBounds, TextRenderer, Viewport, Wrap,
};
use std::ops::Range;
use std::sync::Arc;
use zui::renderer_support::{create_font_system, font_family, font_style, font_weight};

use zui::{Color, Rect, SceneBatch, TextBlock, TextBlockWrap, TextStyle, UiScene};

use self::icon::IconRenderer;
use self::image::ImageRenderer;
use self::rect::RectRenderer;

mod icon;
mod image;
mod rect;

/// Physical render-target extent paired with the logical-to-physical UI scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiViewport {
    width: u32,
    height: u32,
    scale_factor: f32,
}

impl UiViewport {
    pub const fn new(width: u32, height: u32, scale_factor: f32) -> Self {
        Self {
            width,
            height,
            scale_factor,
        }
    }

    pub const fn width(self) -> u32 {
        self.width
    }

    pub const fn height(self) -> u32 {
        self.height
    }

    pub const fn scale_factor(self) -> f32 {
        self.scale_factor
    }
}

struct PreparedArea {
    left: f32,
    top: f32,
    bounds: TextBounds,
    color: GlyphColor,
}

struct TextLayer {
    renderer: TextRenderer,
    buffers: Vec<Arc<Buffer>>,
    areas: Vec<PreparedArea>,
}

struct CachedTextBuffer {
    block: TextBlock,
    scale_factor_bits: u32,
    buffer: Arc<Buffer>,
}

impl CachedTextBuffer {
    fn matches(&self, block: &TextBlock, scale_factor: f32) -> bool {
        self.scale_factor_bits == scale_factor.to_bits()
            && same_text_buffer_layout(&self.block, block)
    }
}

enum PreparedBatch {
    Rects(Range<usize>),
    Icons(Range<usize>),
    Images(Range<usize>),
    Text(usize),
}

impl TextLayer {
    fn new(atlas: &mut TextAtlas, device: &wgpu::Device) -> Self {
        Self {
            renderer: TextRenderer::new(atlas, device, wgpu::MultisampleState::default(), None),
            buffers: Vec::new(),
            areas: Vec::new(),
        }
    }
}

/// Owns the font shaping, glyph cache, atlas, and GPU pipeline for a native UI surface.
pub struct UiRenderer {
    rect_renderer: RectRenderer,
    icon_renderer: IconRenderer,
    image_renderer: ImageRenderer,
    font_system: glyphon::FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    text_batches: Vec<TextLayer>,
    text_buffer_cache: Vec<CachedTextBuffer>,
    prepared_batches: Vec<PreparedBatch>,
    prepared_scene_batches: Vec<SceneBatch>,
    prepared_text_blocks: Vec<TextBlock>,
    prepared_target: Option<UiViewport>,
}

impl UiRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, surface_format);
        let text_batches = vec![TextLayer::new(&mut atlas, device)];
        Self {
            rect_renderer: RectRenderer::new(device, surface_format),
            icon_renderer: IconRenderer::new(device, surface_format),
            image_renderer: ImageRenderer::new(device, surface_format),
            font_system: create_font_system(),
            swash_cache: SwashCache::new(),
            viewport,
            atlas,
            text_batches,
            text_buffer_cache: Vec::new(),
            prepared_batches: Vec::new(),
            prepared_scene_batches: Vec::new(),
            prepared_text_blocks: Vec::new(),
            prepared_target: None,
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &UiScene,
        target: UiViewport,
    ) -> Result<(), UiRenderError> {
        let scale_factor = target.scale_factor;
        if !scale_factor.is_finite() || scale_factor <= 0.0 {
            return Err(UiRenderError::InvalidScaleFactor(scale_factor));
        }
        let target_changed = self.prepared_target != Some(target);
        if target_changed {
            self.viewport.update(
                queue,
                Resolution {
                    width: target.width,
                    height: target.height,
                },
            );
        }
        self.rect_renderer.prepare(device, queue, scene, target)?;
        self.icon_renderer.prepare(device, queue, scene, target)?;
        self.image_renderer.prepare(device, queue, scene, target)?;
        let scene_batches = scene.batches().collect::<Vec<_>>();
        let text_changed = target_changed
            || self.prepared_scene_batches != scene_batches
            || self.prepared_text_blocks != scene.text_blocks();
        if text_changed {
            self.refresh_text_buffer_cache(scene, scale_factor)?;
            self.prepared_batches.clear();
            let mut text_batch_index = 0;
            for batch in &scene_batches {
                match batch {
                    SceneBatch::Rects { range, .. } => {
                        self.prepared_batches
                            .push(PreparedBatch::Rects(range.clone()));
                    }
                    SceneBatch::Icons { range, .. } => {
                        self.prepared_batches
                            .push(PreparedBatch::Icons(range.clone()));
                    }
                    SceneBatch::Images { range, .. } => {
                        self.prepared_batches
                            .push(PreparedBatch::Images(range.clone()));
                    }
                    SceneBatch::Text { range, .. } => {
                        if text_batch_index == self.text_batches.len() {
                            self.text_batches
                                .push(TextLayer::new(&mut self.atlas, device));
                        }
                        let text_batch = &mut self.text_batches[text_batch_index];
                        text_batch.buffers.clear();
                        text_batch.areas.clear();
                        for index in range.clone() {
                            let block = &scene.text_blocks()[index];
                            text_batch
                                .buffers
                                .push(self.text_buffer_cache[index].buffer.clone());
                            text_batch.areas.push(prepared_area(
                                block,
                                scale_factor,
                                block.style().color(),
                            ));
                        }
                        let text_areas =
                            text_batch.buffers.iter().zip(text_batch.areas.iter()).map(
                                |(buffer, area)| TextArea {
                                    buffer,
                                    left: area.left,
                                    top: area.top,
                                    scale: 1.0,
                                    bounds: area.bounds,
                                    default_color: area.color,
                                    custom_glyphs: &[],
                                },
                            );
                        text_batch.renderer.prepare(
                            device,
                            queue,
                            &mut self.font_system,
                            &mut self.atlas,
                            &self.viewport,
                            text_areas,
                            &mut self.swash_cache,
                        )?;
                        self.prepared_batches
                            .push(PreparedBatch::Text(text_batch_index));
                        text_batch_index += 1;
                    }
                }
            }
            for unused_batch in self.text_batches.iter_mut().skip(text_batch_index) {
                unused_batch.buffers.clear();
                unused_batch.areas.clear();
            }
        }
        self.prepared_scene_batches = scene_batches;
        self.prepared_text_blocks = scene.text_blocks().to_vec();
        self.prepared_target = Some(target);
        Ok(())
    }

    fn refresh_text_buffer_cache(
        &mut self,
        scene: &UiScene,
        scale_factor: f32,
    ) -> Result<(), UiRenderError> {
        for (index, block) in scene.text_blocks().iter().enumerate() {
            validate_text_block(index, block)?;
            if self
                .text_buffer_cache
                .get(index)
                .is_some_and(|cached| cached.matches(block, scale_factor))
            {
                continue;
            }
            let cached = CachedTextBuffer {
                block: block.clone(),
                scale_factor_bits: scale_factor.to_bits(),
                buffer: Arc::new(prepare_text_buffer(
                    &mut self.font_system,
                    block,
                    scale_factor,
                )),
            };
            if let Some(slot) = self.text_buffer_cache.get_mut(index) {
                *slot = cached;
            } else {
                self.text_buffer_cache.push(cached);
            }
        }
        self.text_buffer_cache.truncate(scene.text_blocks().len());
        Ok(())
    }

    pub fn render<'pass>(
        &'pass self,
        render_pass: &mut wgpu::RenderPass<'pass>,
    ) -> Result<(), UiRenderError> {
        for batch in &self.prepared_batches {
            match batch {
                PreparedBatch::Rects(range) => {
                    self.rect_renderer.render_range(render_pass, range.clone());
                }
                PreparedBatch::Icons(range) => {
                    self.icon_renderer.render_range(render_pass, range.clone());
                }
                PreparedBatch::Images(range) => {
                    self.image_renderer.render_range(render_pass, range.clone());
                }
                PreparedBatch::Text(index) => self.text_batches[*index].renderer.render(
                    &self.atlas,
                    &self.viewport,
                    render_pass,
                )?,
            }
        }
        Ok(())
    }

    pub fn trim(&mut self) {
        self.atlas.trim();
    }
}

fn same_text_buffer_layout(left: &TextBlock, right: &TextBlock) -> bool {
    left.text() == right.text()
        && left.spans() == right.spans()
        && left.bounds() == right.bounds()
        && left.wrap() == right.wrap()
        && same_shaping_style(left.style(), right.style())
}

fn same_shaping_style(left: &TextStyle, right: &TextStyle) -> bool {
    left.family() == right.family()
        && left.font_size().to_bits() == right.font_size().to_bits()
        && left.line_height().to_bits() == right.line_height().to_bits()
        && left.weight() == right.weight()
        && left.style() == right.style()
}

fn validate_text_block(index: usize, block: &TextBlock) -> Result<(), UiRenderError> {
    let origin = block.origin();
    let bounds = block.bounds();
    let style = block.style();
    let values = [
        origin.x,
        origin.y,
        bounds.width,
        bounds.height,
        style.font_size(),
        style.line_height(),
    ];
    if values.into_iter().any(|value| !value.is_finite()) {
        return Err(UiRenderError::InvalidTextBlock {
            index,
            reason: "coordinates and metrics must be finite",
        });
    }
    if bounds.width <= 0.0 || bounds.height <= 0.0 {
        return Err(UiRenderError::InvalidTextBlock {
            index,
            reason: "bounds must be positive",
        });
    }
    if style.font_size() <= 0.0 || style.line_height() <= 0.0 {
        return Err(UiRenderError::InvalidTextBlock {
            index,
            reason: "font size and line height must be positive",
        });
    }
    for span in block.spans() {
        let style = span.style();
        if !style.font_size().is_finite()
            || !style.line_height().is_finite()
            || style.font_size() <= 0.0
            || style.line_height() <= 0.0
        {
            return Err(UiRenderError::InvalidTextBlock {
                index,
                reason: "span font size and line height must be finite and positive",
            });
        }
    }
    if let Some(clip) = block.clip_bounds() {
        let values = [
            clip.origin.x,
            clip.origin.y,
            clip.size.width,
            clip.size.height,
        ];
        if values.into_iter().any(|value| !value.is_finite()) {
            return Err(UiRenderError::InvalidTextBlock {
                index,
                reason: "clip bounds must be finite",
            });
        }
        if clip.size.width < 0.0 || clip.size.height < 0.0 {
            return Err(UiRenderError::InvalidTextBlock {
                index,
                reason: "clip bounds must not be negative",
            });
        }
    }
    Ok(())
}

fn attrs_for_style(style: &TextStyle, scale_factor: f32) -> Attrs<'_> {
    Attrs::new()
        .family(font_family(style.family()))
        .weight(font_weight(style.weight()))
        .style(font_style(style.style()))
        .color(glyphon_color(style.color()))
        .metrics(Metrics::new(
            style.font_size() * scale_factor,
            style.line_height() * scale_factor,
        ))
}

fn prepare_text_buffer(
    font_system: &mut glyphon::FontSystem,
    block: &TextBlock,
    scale_factor: f32,
) -> Buffer {
    let style = block.style();
    let metrics = Metrics::new(
        style.font_size() * scale_factor,
        style.line_height() * scale_factor,
    );
    let mut buffer = Buffer::new(font_system, metrics);
    buffer.set_wrap(glyphon_wrap(block.wrap()));
    let bounds = block.bounds();
    buffer.set_size(
        Some(bounds.width * scale_factor),
        Some(bounds.height * scale_factor),
    );
    let attrs = Attrs::new()
        .family(font_family(style.family()))
        .weight(font_weight(style.weight()))
        .style(font_style(style.style()));
    if block.spans().is_empty() {
        buffer.set_text(block.text(), &attrs, Shaping::Advanced, None);
    } else {
        buffer.set_rich_text(
            block
                .spans()
                .iter()
                .map(|span| (span.text(), attrs_for_style(span.style(), scale_factor))),
            &attrs,
            Shaping::Advanced,
            None,
        );
    }
    buffer.shape_until_scroll(font_system, false);
    buffer
}

fn prepared_area(block: &TextBlock, scale_factor: f32, color: Color) -> PreparedArea {
    let origin = block.origin();
    let size = block.bounds();
    let left = origin.x * scale_factor;
    let top = origin.y * scale_factor;
    let block_bounds = Rect::new(origin, size);
    let clip_bounds = block
        .clip_bounds()
        .map(|clip| clip.intersection(block_bounds))
        .unwrap_or(block_bounds);
    PreparedArea {
        left,
        top,
        bounds: TextBounds {
            left: (clip_bounds.origin.x * scale_factor).floor() as i32,
            top: (clip_bounds.origin.y * scale_factor).floor() as i32,
            right: (clip_bounds.right() * scale_factor).ceil() as i32,
            bottom: (clip_bounds.bottom() * scale_factor).ceil() as i32,
        },
        color: glyphon_color(color),
    }
}

const fn glyphon_wrap(wrap: TextBlockWrap) -> Wrap {
    match wrap {
        TextBlockWrap::WordOrGlyph => Wrap::WordOrGlyph,
        TextBlockWrap::None => Wrap::None,
    }
}

fn glyphon_color(color: Color) -> GlyphColor {
    let [red, green, blue, alpha] = color.components();
    GlyphColor::rgba(red, green, blue, alpha)
}

#[derive(Debug, thiserror::Error)]
pub enum UiRenderError {
    #[error("UI scale factor must be finite and positive, got {0}")]
    InvalidScaleFactor(f32),
    #[error("paint rect {index} is invalid: {reason}")]
    InvalidPaintRect { index: usize, reason: &'static str },
    #[error("paint icon {index} is invalid: {reason}")]
    InvalidPaintIcon { index: usize, reason: &'static str },
    #[error("paint image {index} is invalid: {reason}")]
    InvalidPaintImage { index: usize, reason: &'static str },
    #[error("SVG icon {name} is invalid: {reason}")]
    InvalidSvgIcon { name: &'static str, reason: String },
    #[error("SVG icon {name} cannot be rasterized at {width}x{height}")]
    IconRasterTooLarge {
        name: &'static str,
        width: u32,
        height: u32,
    },
    #[error("icon atlas is full at {width}x{height}")]
    IconAtlasFull { width: u32, height: u32 },
    #[error("image atlas is full at {width}x{height}")]
    ImageAtlasFull { width: u32, height: u32 },
    #[error("text block {index} is invalid: {reason}")]
    InvalidTextBlock { index: usize, reason: &'static str },
    #[error("failed to prepare UI text: {0}")]
    Prepare(#[from] glyphon::PrepareError),
    #[error("failed to render UI text: {0}")]
    Render(#[from] glyphon::RenderError),
}

#[cfg(test)]
#[path = "ui_renderer_tests.rs"]
mod tests;
