//! Backend-neutral rendering boundary between UI scenes and graphics implementations.

use std::error::Error;

use thiserror::Error;
use zui::UiScene;

/// Physical dimensions of the renderer's current presentation target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderTargetSize {
    width: u32,
    height: u32,
}

impl RenderTargetSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub const fn width(self) -> u32 {
        self.width
    }

    pub const fn height(self) -> u32 {
        self.height
    }
}

/// Result of attempting to present one renderer frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderOutcome {
    Presented,
    Skipped,
    Retry,
}

/// A graphics-backend failure crossing the backend-neutral renderer boundary.
#[derive(Debug, Error)]
#[error("render backend failed: {source}")]
pub struct RendererError {
    #[source]
    source: Box<dyn Error + Send + Sync>,
}

impl RendererError {
    /// Preserves a concrete backend error as the source of a renderer failure.
    pub fn backend(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}

/// Executes backend-neutral [`UiScene`] frames on one presentation target.
///
/// Implementations own graphics resources and surface lifecycle. Product components only emit
/// scene primitives; they must never receive a concrete graphics device, queue, encoder, or render
/// pass. Hosts may replace an implementation without changing component or scene construction.
pub trait Renderer {
    /// Reconfigures the physical presentation target.
    fn resize(&mut self, size: RenderTargetSize);

    /// Records the logical-to-physical scale used for subsequent frames.
    fn set_scale_factor(&mut self, scale_factor: f64);

    /// Clears and presents one frame without UI scene content.
    fn render(&mut self) -> Result<RenderOutcome, RendererError>;

    /// Renders and presents one immutable backend-neutral UI scene.
    fn render_scene(&mut self, scene: &UiScene) -> Result<RenderOutcome, RendererError>;
}

#[cfg(test)]
#[path = "renderer_tests.rs"]
mod tests;
