use zeta_renderer::{RenderOutcome, RenderTargetSize, Renderer, RendererError};
use zeta_winit::{NativeWindow, PhysicalExtent};
use zui::{Color, UiScene};

use crate::ui_renderer::{UiRenderer, UiViewport};
use crate::viewport::{SurfaceExtent, Viewport};

const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.035,
    g: 0.043,
    b: 0.055,
    a: 1.0,
};

#[derive(Debug, thiserror::Error)]
pub enum WgpuRendererError {
    #[error("failed to create the presentation surface: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
    #[error("no compatible graphics adapter is available: {0}")]
    RequestAdapter(#[from] wgpu::RequestAdapterError),
    #[error("failed to create the graphics device: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
    #[error("the selected adapter cannot present to this window")]
    UnsupportedSurface,
    #[error("wgpu rejected a surface frame")]
    SurfaceValidation,
    #[error(transparent)]
    Ui(#[from] crate::ui_renderer::UiRenderError),
}

/// Owns one window's `wgpu` presentation resources.
///
/// The caller owns application state and event routing. This renderer owns only GPU/surface
/// lifecycle and never performs product I/O.
pub struct WgpuRenderer {
    window: NativeWindow,
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    viewport: Viewport,
    ui_renderer: UiRenderer,
}

impl Renderer for WgpuRenderer {
    fn resize(&mut self, size: RenderTargetSize) {
        self.resize(PhysicalExtent::new(size.width(), size.height()));
    }

    fn set_scale_factor(&mut self, scale_factor: f64) {
        self.set_scale_factor(scale_factor);
    }

    fn render(&mut self) -> Result<RenderOutcome, RendererError> {
        self.render().map_err(RendererError::backend)
    }

    fn render_scene(&mut self, scene: &UiScene) -> Result<RenderOutcome, RendererError> {
        self.render_scene(scene).map_err(RendererError::backend)
    }
}

impl WgpuRenderer {
    /// Initializes presentation resources for a native window.
    pub fn initialize(window: NativeWindow) -> Result<Self, WgpuRendererError> {
        pollster::block_on(Self::initialize_async(window))
    }

    async fn initialize_async(window: NativeWindow) -> Result<Self, WgpuRendererError> {
        let descriptor = wgpu::InstanceDescriptor::new_with_display_handle_from_env(Box::new(
            window.display_handle(),
        ));
        let instance = wgpu::Instance::new(descriptor);
        let surface = instance.create_surface(window.surface_target())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("zeta-wgpu device"),
                ..Default::default()
            })
            .await?;
        let extent = window.inner_extent();
        let viewport = Viewport::new(extent.width, extent.height, window.scale_factor());
        let extent = viewport.surface_extent().unwrap_or(SurfaceExtent {
            width: 1,
            height: 1,
        });
        let config = surface
            .get_default_config(&adapter, extent.width, extent.height)
            .ok_or(WgpuRendererError::UnsupportedSurface)?;
        surface.configure(&device, &config);
        let ui_renderer = UiRenderer::new(&device, &queue, config.format);

        Ok(Self {
            window,
            instance,
            surface,
            device,
            queue,
            config,
            viewport,
            ui_renderer,
        })
    }

    /// Reconfigures the surface for a new physical window size.
    pub fn resize(&mut self, extent: PhysicalExtent) {
        self.viewport.resize(extent.width, extent.height);
        self.configure_for_viewport();
    }

    /// Records the window's current logical-to-physical scale factor.
    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        self.viewport.set_scale_factor(scale_factor);
    }

    /// Schedules a redraw through the renderer's owned native window.
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    /// Renders one invalidated frame.
    pub fn render(&mut self) -> Result<RenderOutcome, WgpuRendererError> {
        self.render_frame(None)
    }

    /// Renders one immutable UI scene into the next presented frame.
    pub fn render_scene(&mut self, scene: &UiScene) -> Result<RenderOutcome, WgpuRendererError> {
        self.render_frame(Some(scene))
    }

    fn render_frame(
        &mut self,
        scene: Option<&UiScene>,
    ) -> Result<RenderOutcome, WgpuRendererError> {
        if self.viewport.surface_extent().is_none() {
            return Ok(RenderOutcome::Skipped);
        }

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(RenderOutcome::Skipped);
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.configure_for_viewport();
                return Ok(RenderOutcome::Retry);
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                drop(frame);
                self.configure_for_viewport();
                return Ok(RenderOutcome::Retry);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.recreate_surface()?;
                return Ok(RenderOutcome::Retry);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(WgpuRendererError::SurfaceValidation);
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        if let Some(scene) = scene {
            self.ui_renderer.prepare(
                &self.device,
                &self.queue,
                scene,
                UiViewport::new(
                    self.config.width,
                    self.config.height,
                    self.viewport.scale_factor() as f32,
                ),
            )?;
        }
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("zeta-wgpu frame encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("zeta-wgpu clear pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(
                            scene
                                .map(|scene| wgpu_color(scene.background()))
                                .unwrap_or(CLEAR_COLOR),
                        ),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if scene.is_some() {
                self.ui_renderer.render(&mut pass)?;
            }
        }
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        self.queue.present(frame);
        if scene.is_some() {
            self.ui_renderer.trim();
        }

        Ok(RenderOutcome::Presented)
    }

    fn recreate_surface(&mut self) -> Result<(), WgpuRendererError> {
        self.surface = self.instance.create_surface(self.window.surface_target())?;
        self.configure_for_viewport();
        Ok(())
    }

    fn configure_for_viewport(&mut self) {
        let Some(extent) = self.viewport.surface_extent() else {
            return;
        };
        self.config.width = extent.width;
        self.config.height = extent.height;
        self.surface.configure(&self.device, &self.config);
    }
}

fn wgpu_color(color: Color) -> wgpu::Color {
    let [red, green, blue, alpha] = color.components();
    wgpu::Color {
        r: srgb_channel(red),
        g: srgb_channel(green),
        b: srgb_channel(blue),
        a: f64::from(alpha) / 255.0,
    }
}

fn srgb_channel(channel: u8) -> f64 {
    let encoded = f64::from(channel) / 255.0;
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
#[path = "gpu_tests.rs"]
mod tests;
