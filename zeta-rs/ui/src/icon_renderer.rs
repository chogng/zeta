use std::collections::HashMap;
use std::mem;
use std::ops::Range;

use bytemuck::{Pod, Zeroable};
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg;
use zeta_icons::{Icon, IconRendering};

use crate::rect_renderer::linear_color;
use crate::{PaintIcon, Rect, UiRenderError, UiScene, UiViewport};

const ATLAS_SIZE: u32 = 2_048;
const ATLAS_PADDING: u32 = 1;
const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
    0 => Float32x4,
    1 => Float32x4,
    2 => Float32x4,
    3 => Float32x4,
    4 => Float32x4
];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct IconInstance {
    bounds: [f32; 4],
    uv_bounds: [f32; 4],
    color: [f32; 4],
    clip_bounds: [f32; 4],
    viewport: [f32; 4],
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct RasterKey {
    icon: Icon,
    width: u32,
    height: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AtlasRegion {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

struct ShelfAllocator {
    next_x: u32,
    next_y: u32,
    row_height: u32,
}

impl ShelfAllocator {
    fn new() -> Self {
        Self {
            next_x: ATLAS_PADDING,
            next_y: ATLAS_PADDING,
            row_height: 0,
        }
    }

    fn allocate(&mut self, width: u32, height: u32) -> Option<AtlasRegion> {
        if width + ATLAS_PADDING * 2 > ATLAS_SIZE || height + ATLAS_PADDING * 2 > ATLAS_SIZE {
            return None;
        }
        if self.next_x + width + ATLAS_PADDING > ATLAS_SIZE {
            self.next_x = ATLAS_PADDING;
            self.next_y += self.row_height + ATLAS_PADDING;
            self.row_height = 0;
        }
        if self.next_y + height + ATLAS_PADDING > ATLAS_SIZE {
            return None;
        }
        let region = AtlasRegion {
            x: self.next_x,
            y: self.next_y,
            width,
            height,
        };
        self.next_x += width + ATLAS_PADDING;
        self.row_height = self.row_height.max(height);
        Some(region)
    }
}

pub(crate) struct IconRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    mask_atlas: wgpu::Texture,
    color_atlas: wgpu::Texture,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    layer_ranges: Vec<Range<u32>>,
    allocator: ShelfAllocator,
    regions: HashMap<RasterKey, AtlasRegion>,
}

impl IconRenderer {
    pub(crate) fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let mask_atlas = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("zeta-ui icon symbolic-mask atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let color_atlas = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("zeta-ui icon fixed-color atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let mask_atlas_view = mask_atlas.create_view(&wgpu::TextureViewDescriptor::default());
        let color_atlas_view = color_atlas.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("zeta-ui icon sampler"),
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("zeta-ui icon bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("zeta-ui icon bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&mask_atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&color_atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("zeta-ui icon shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("icon.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("zeta-ui icon pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("zeta-ui icon pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: mem::size_of::<IconInstance>() as wgpu::BufferAddress,
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
        Self {
            pipeline,
            bind_group,
            mask_atlas,
            color_atlas,
            instance_buffer: create_instance_buffer(device, instance_capacity),
            instance_capacity,
            layer_ranges: Vec::new(),
            allocator: ShelfAllocator::new(),
            regions: HashMap::new(),
        }
    }

    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &UiScene,
        target: UiViewport,
    ) -> Result<(), UiRenderError> {
        let prepared = self.prepare_instances(queue, scene, target)?;
        let instances = prepared.instances;
        self.layer_ranges = prepared.layer_ranges;
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

    fn prepare_instances(
        &mut self,
        queue: &wgpu::Queue,
        scene: &UiScene,
        target: UiViewport,
    ) -> Result<PreparedIconInstances, UiRenderError> {
        let scale_factor = target.scale_factor();
        let viewport = [target.width() as f32, target.height() as f32, 0.0, 0.0];
        let logical_viewport = Rect::from_xywh(
            0.0,
            0.0,
            target.width() as f32 / scale_factor,
            target.height() as f32 / scale_factor,
        );
        let mut instances = Vec::with_capacity(scene.icons().len());
        let mut layer_ranges = Vec::with_capacity(scene.layer_count());
        for layer in 0..scene.layer_count() {
            let start = instances.len() as u32;
            for (index, icon) in scene.icons().iter().copied().enumerate() {
                if scene.icon_layers()[index] != layer {
                    continue;
                }
                validate_paint_icon(index, icon)?;
                let bounds = icon.bounds();
                let clip_bounds = icon
                    .clip_bounds()
                    .map(|clip| clip.intersection(logical_viewport))
                    .unwrap_or(logical_viewport);
                if bounds.is_empty() || bounds.intersection(clip_bounds).is_empty() {
                    continue;
                }
                let width = (bounds.size.width * scale_factor).ceil() as u32;
                let height = (bounds.size.height * scale_factor).ceil() as u32;
                let region = self.region_for(queue, icon.icon(), width, height)?;
                instances.push(IconInstance {
                    bounds: scaled_rect(bounds, scale_factor),
                    uv_bounds: region.uv_bounds(),
                    color: linear_color(icon.color()),
                    clip_bounds: scaled_rect(clip_bounds, scale_factor),
                    viewport,
                });
            }
            layer_ranges.push(start..instances.len() as u32);
        }
        Ok(PreparedIconInstances {
            instances,
            layer_ranges,
        })
    }

    fn region_for(
        &mut self,
        queue: &wgpu::Queue,
        icon: Icon,
        width: u32,
        height: u32,
    ) -> Result<AtlasRegion, UiRenderError> {
        let key = RasterKey {
            icon,
            width,
            height,
        };
        if let Some(region) = self.regions.get(&key) {
            return Ok(*region);
        }
        let raster = rasterize_icon(icon, width, height)?;
        let Some(region) = self.allocator.allocate(width, height) else {
            return Err(UiRenderError::IconAtlasFull {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
            });
        };
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.mask_atlas,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: region.x,
                    y: region.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &raster.mask,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.color_atlas,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: region.x,
                    y: region.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &raster.color,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.regions.insert(key, region);
        Ok(region)
    }

    pub(crate) fn render_layer<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        layer: usize,
    ) {
        let Some(range) = self.layer_ranges.get(layer) else {
            return;
        };
        if range.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
        pass.draw(0..6, range.clone());
    }
}

struct PreparedIconInstances {
    instances: Vec<IconInstance>,
    layer_ranges: Vec<Range<u32>>,
}

impl AtlasRegion {
    fn uv_bounds(self) -> [f32; 4] {
        let atlas_size = ATLAS_SIZE as f32;
        [
            self.x as f32 / atlas_size,
            self.y as f32 / atlas_size,
            self.width as f32 / atlas_size,
            self.height as f32 / atlas_size,
        ]
    }
}

fn create_instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("zeta-ui symbolic icon instances"),
        size: (capacity * mem::size_of::<IconInstance>()) as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn validate_paint_icon(index: usize, icon: PaintIcon) -> Result<(), UiRenderError> {
    let bounds = icon.bounds();
    let values = [
        bounds.origin.x,
        bounds.origin.y,
        bounds.size.width,
        bounds.size.height,
    ];
    if values.into_iter().any(|value| !value.is_finite()) {
        return Err(UiRenderError::InvalidPaintIcon {
            index,
            reason: "coordinates must be finite",
        });
    }
    if bounds.size.width < 0.0 || bounds.size.height < 0.0 {
        return Err(UiRenderError::InvalidPaintIcon {
            index,
            reason: "bounds must not be negative",
        });
    }
    if let Some(clip) = icon.clip_bounds() {
        let values = [
            clip.origin.x,
            clip.origin.y,
            clip.size.width,
            clip.size.height,
        ];
        if values.into_iter().any(|value| !value.is_finite()) {
            return Err(UiRenderError::InvalidPaintIcon {
                index,
                reason: "clip bounds must be finite",
            });
        }
        if clip.size.width < 0.0 || clip.size.height < 0.0 {
            return Err(UiRenderError::InvalidPaintIcon {
                index,
                reason: "clip bounds must not be negative",
            });
        }
    }
    Ok(())
}

struct RasterizedIcon {
    mask: Vec<u8>,
    color: Vec<u8>,
}

fn rasterize_icon(icon: Icon, width: u32, height: u32) -> Result<RasterizedIcon, UiRenderError> {
    let tree = usvg::Tree::from_data(icon.definition().svg(), &usvg::Options::default()).map_err(
        |error| UiRenderError::InvalidSvgIcon {
            name: icon.id().as_str(),
            reason: error.to_string(),
        },
    )?;
    let mut pixmap = Pixmap::new(width, height).ok_or(UiRenderError::IconRasterTooLarge {
        name: icon.id().as_str(),
        width,
        height,
    })?;
    let source_size = tree.size();
    let scale = (width as f32 / source_size.width()).min(height as f32 / source_size.height());
    let offset_x = (width as f32 - source_size.width() * scale) * 0.5;
    let offset_y = (height as f32 - source_size.height() * scale) * 0.5;
    let transform = Transform::from_row(scale, 0.0, 0.0, scale, offset_x, offset_y);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let mut mask = Vec::with_capacity((width * height) as usize);
    let mut color = Vec::with_capacity((width * height * 4) as usize);
    for pixel in pixmap.pixels() {
        let pixel = pixel.demultiply();
        let is_symbolic = icon.definition().rendering() == IconRendering::Symbolic
            || (pixel.red() == 0 && pixel.green() == 0 && pixel.blue() == 0);
        mask.push(if is_symbolic { pixel.alpha() } else { 0 });
        if is_symbolic {
            color.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            color.extend_from_slice(&[pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()]);
        }
    }
    Ok(RasterizedIcon { mask, color })
}

fn scaled_rect(rect: Rect, scale_factor: f32) -> [f32; 4] {
    [
        rect.origin.x * scale_factor,
        rect.origin.y * scale_factor,
        rect.size.width * scale_factor,
        rect.size.height * scale_factor,
    ]
}

#[cfg(test)]
#[path = "icon_renderer_tests.rs"]
mod tests;
