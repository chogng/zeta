use zeta_renderer::{Renderer, RendererError};
use zeta_wgpu::WgpuRenderer;
use zeta_winit::NativeWindow;

/// Creates the graphics backend selected by the native product composition root.
///
/// Backend choice is isolated here so product state, components, and the event loop depend only on
/// [`Renderer`]. Replacing wgpu changes this adapter and crate wiring, not scene construction.
pub(crate) fn create(window: NativeWindow) -> Result<Box<dyn Renderer>, RendererError> {
    WgpuRenderer::initialize(window)
        .map(|renderer| Box::new(renderer) as Box<dyn Renderer>)
        .map_err(RendererError::backend)
}
