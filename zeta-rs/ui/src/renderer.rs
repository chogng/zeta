use glyphon::{
    Attrs, Buffer, Cache, Color as GlyphColor, Metrics, Resolution, Shaping, SwashCache, TextArea,
    TextAtlas, TextBounds, TextRenderer, Viewport,
};

use crate::font::mapping::{glyphon_family, glyphon_style, glyphon_weight};
use crate::icon_renderer::IconRenderer;
use crate::rect_renderer::RectRenderer;
use crate::{Rect, TextBlock, UiScene};

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

/// Owns the font shaping, glyph cache, atlas, and GPU pipeline for a native UI surface.
pub struct UiRenderer {
    rect_renderer: RectRenderer,
    icon_renderer: IconRenderer,
    font_system: glyphon::FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    buffers: Vec<Buffer>,
    areas: Vec<PreparedArea>,
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
        let text_renderer =
            TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);
        Self {
            rect_renderer: RectRenderer::new(device, surface_format),
            icon_renderer: IconRenderer::new(device, surface_format),
            font_system: glyphon::FontSystem::new(),
            swash_cache: SwashCache::new(),
            viewport,
            atlas,
            text_renderer,
            buffers: Vec::new(),
            areas: Vec::new(),
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
        self.viewport.update(
            queue,
            Resolution {
                width: target.width,
                height: target.height,
            },
        );
        self.buffers.clear();
        self.areas.clear();
        self.rect_renderer.prepare(device, queue, scene, target)?;
        self.icon_renderer.prepare(device, queue, scene, target)?;

        for (index, block) in scene.text_blocks().iter().enumerate() {
            validate_text_block(index, block)?;
            let style = block.style();
            let metrics = Metrics::new(
                style.font_size() * scale_factor,
                style.line_height() * scale_factor,
            );
            let mut buffer = Buffer::new(&mut self.font_system, metrics);
            let bounds = block.bounds();
            buffer.set_size(
                Some(bounds.width * scale_factor),
                Some(bounds.height * scale_factor),
            );
            let attrs = Attrs::new()
                .family(glyphon_family(style.family()))
                .weight(glyphon_weight(style.weight()))
                .style(glyphon_style(style.style()));
            buffer.set_text(block.text(), &attrs, Shaping::Advanced, None);
            buffer.shape_until_scroll(&mut self.font_system, false);
            self.buffers.push(buffer);
            self.areas
                .push(prepared_area(block, scale_factor, style.color()));
        }

        let Self {
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
            buffers,
            areas,
            ..
        } = self;
        let text_areas = buffers
            .iter()
            .zip(areas.iter())
            .map(|(buffer, area)| TextArea {
                buffer,
                left: area.left,
                top: area.top,
                scale: 1.0,
                bounds: area.bounds,
                default_color: area.color,
                custom_glyphs: &[],
            });
        text_renderer.prepare(
            device,
            queue,
            font_system,
            atlas,
            viewport,
            text_areas,
            swash_cache,
        )?;
        Ok(())
    }

    pub fn render<'pass>(
        &'pass self,
        render_pass: &mut wgpu::RenderPass<'pass>,
    ) -> Result<(), UiRenderError> {
        self.rect_renderer.render(render_pass);
        self.icon_renderer.render(render_pass);
        self.text_renderer
            .render(&self.atlas, &self.viewport, render_pass)?;
        Ok(())
    }

    pub fn trim(&mut self) {
        self.atlas.trim();
    }
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

fn prepared_area(block: &TextBlock, scale_factor: f32, color: crate::Color) -> PreparedArea {
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

fn glyphon_color(color: crate::Color) -> GlyphColor {
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
    #[error("SVG icon {name} is invalid: {reason}")]
    InvalidSvgIcon { name: &'static str, reason: String },
    #[error("multicolor icon {name} is not supported by the symbolic icon atlas")]
    UnsupportedMulticolorIcon { name: &'static str },
    #[error("SVG icon {name} cannot be rasterized at {width}x{height}")]
    IconRasterTooLarge {
        name: &'static str,
        width: u32,
        height: u32,
    },
    #[error("symbolic icon atlas is full at {width}x{height}")]
    IconAtlasFull { width: u32, height: u32 },
    #[error("text block {index} is invalid: {reason}")]
    InvalidTextBlock { index: usize, reason: &'static str },
    #[error("failed to prepare UI text: {0}")]
    Prepare(#[from] glyphon::PrepareError),
    #[error("failed to render UI text: {0}")]
    Render(#[from] glyphon::RenderError),
}

#[cfg(test)]
#[path = "renderer_tests.rs"]
mod tests;
