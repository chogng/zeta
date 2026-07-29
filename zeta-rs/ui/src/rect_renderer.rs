use std::mem;

use bytemuck::{Pod, Zeroable};

use crate::{Color, PaintRect, Rect, UiRenderError, UiScene, UiViewport};

const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![
    0 => Float32x4,
    1 => Float32x4,
    2 => Float32x4,
    3 => Float32x4,
    4 => Float32x4,
    5 => Float32x4,
    6 => Float32x4
];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct RectInstance {
    bounds: [f32; 4],
    fill: [f32; 4],
    border_color: [f32; 4],
    border_widths: [f32; 4],
    corner_radii: [f32; 4],
    clip_bounds: [f32; 4],
    viewport: [f32; 4],
}

pub(crate) struct RectRenderer {
    pipeline: wgpu::RenderPipeline,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    instance_count: u32,
}

impl RectRenderer {
    pub(crate) fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("zeta-ui rect shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("rect.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("zeta-ui rect pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: mem::size_of::<RectInstance>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &INSTANCE_ATTRIBUTES,
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let instance_capacity = 1;
        let instance_buffer = create_instance_buffer(device, instance_capacity);
        Self {
            pipeline,
            instance_buffer,
            instance_capacity,
            instance_count: 0,
        }
    }

    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &UiScene,
        target: UiViewport,
    ) -> Result<(), UiRenderError> {
        let instances = prepare_instances(scene, target)?;
        self.instance_count = instances.len() as u32;
        if instances.is_empty() {
            return Ok(());
        }
        if instances.len() > self.instance_capacity {
            self.instance_capacity = instances.len().next_power_of_two();
            self.instance_buffer = create_instance_buffer(device, self.instance_capacity);
        }
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));
        Ok(())
    }

    pub(crate) fn render<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
        if self.instance_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
        pass.draw(0..6, 0..self.instance_count);
    }
}

fn create_instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("zeta-ui rect instances"),
        size: (capacity * mem::size_of::<RectInstance>()) as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn prepare_instances(
    scene: &UiScene,
    target: UiViewport,
) -> Result<Vec<RectInstance>, UiRenderError> {
    let scale_factor = target.scale_factor();
    let viewport = [target.width() as f32, target.height() as f32, 0.0, 0.0];
    let logical_viewport = Rect::from_xywh(
        0.0,
        0.0,
        target.width() as f32 / scale_factor,
        target.height() as f32 / scale_factor,
    );
    let mut instances = Vec::with_capacity(scene.rects().len());
    for (index, rect) in scene.rects().iter().copied().enumerate() {
        validate_paint_rect(index, rect)?;
        let clip_bounds = rect
            .clip_bounds()
            .map(|clip| clip.intersection(logical_viewport))
            .unwrap_or(logical_viewport);
        if rect.bounds().is_empty() || clip_bounds.is_empty() {
            continue;
        }
        let bounds = rect.bounds();
        let border = rect.border();
        let widths = border.widths();
        let radii = rect.corner_radii();
        instances.push(RectInstance {
            bounds: scaled_rect(bounds, scale_factor),
            fill: linear_color(rect.fill()),
            border_color: linear_color(border.color()),
            border_widths: [
                widths.top * scale_factor,
                widths.right * scale_factor,
                widths.bottom * scale_factor,
                widths.left * scale_factor,
            ],
            corner_radii: [
                radii.top_left * scale_factor,
                radii.top_right * scale_factor,
                radii.bottom_right * scale_factor,
                radii.bottom_left * scale_factor,
            ],
            clip_bounds: scaled_rect(clip_bounds, scale_factor),
            viewport,
        });
    }
    Ok(instances)
}

fn validate_paint_rect(index: usize, rect: PaintRect) -> Result<(), UiRenderError> {
    let bounds = rect.bounds();
    let widths = rect.border().widths();
    let requested_radii = rect.requested_corner_radii();
    let values = [
        bounds.origin.x,
        bounds.origin.y,
        bounds.size.width,
        bounds.size.height,
        widths.top,
        widths.right,
        widths.bottom,
        widths.left,
        requested_radii.top_left,
        requested_radii.top_right,
        requested_radii.bottom_right,
        requested_radii.bottom_left,
    ];
    if values.into_iter().any(|value| !value.is_finite()) {
        return Err(UiRenderError::InvalidPaintRect {
            index,
            reason: "coordinates and visual metrics must be finite",
        });
    }
    if bounds.size.width < 0.0 || bounds.size.height < 0.0 {
        return Err(UiRenderError::InvalidPaintRect {
            index,
            reason: "bounds must not be negative",
        });
    }
    if [
        widths.top,
        widths.right,
        widths.bottom,
        widths.left,
        requested_radii.top_left,
        requested_radii.top_right,
        requested_radii.bottom_right,
        requested_radii.bottom_left,
    ]
    .into_iter()
    .any(|value| value < 0.0)
    {
        return Err(UiRenderError::InvalidPaintRect {
            index,
            reason: "border widths and corner radii must not be negative",
        });
    }
    if let Some(clip) = rect.clip_bounds() {
        let values = [
            clip.origin.x,
            clip.origin.y,
            clip.size.width,
            clip.size.height,
        ];
        if values.into_iter().any(|value| !value.is_finite()) {
            return Err(UiRenderError::InvalidPaintRect {
                index,
                reason: "clip bounds must be finite",
            });
        }
        if clip.size.width < 0.0 || clip.size.height < 0.0 {
            return Err(UiRenderError::InvalidPaintRect {
                index,
                reason: "clip bounds must not be negative",
            });
        }
    }
    Ok(())
}

fn scaled_rect(rect: Rect, scale_factor: f32) -> [f32; 4] {
    [
        rect.origin.x * scale_factor,
        rect.origin.y * scale_factor,
        rect.size.width * scale_factor,
        rect.size.height * scale_factor,
    ]
}

pub(crate) fn linear_color(color: Color) -> [f32; 4] {
    let [red, green, blue, alpha] = color.components();
    [
        linear_channel(red),
        linear_channel(green),
        linear_channel(blue),
        f32::from(alpha) / 255.0,
    ]
}

fn linear_channel(channel: u8) -> f32 {
    let encoded = f32::from(channel) / 255.0;
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
#[path = "rect_renderer_tests.rs"]
mod tests;
