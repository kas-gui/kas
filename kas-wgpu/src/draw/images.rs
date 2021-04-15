// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Images pipeline

use image::RgbaImage;
use log::warn;
use std::mem::size_of;
use std::path::Path;
use wgpu::util::DeviceExt;

use crate::draw::ShaderManager;
use kas::cast::Cast;
use kas::draw::Pass;
use kas::geom::{Quad, Size, Vec2, Vec3};

const TEXTURE_SIZE: (u32, u32) = (2048, 2048);

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Vertex(Vec3, Vec2);
unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

/// A pipeline for rendering images
pub struct Pipeline {
    bind_group_layout: wgpu::BindGroupLayout,
    render_pipeline: wgpu::RenderPipeline,
    tex: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    image: RgbaImage,
    need_write: bool,
}

/// Per-window state
pub struct Window {
    bind_group: wgpu::BindGroup,
    passes: Vec<Vec<Vertex>>,
}

/// Buffer used during render pass
///
/// This buffer must not be dropped before the render pass.
pub struct RenderBuffer<'a> {
    pipe: &'a wgpu::RenderPipeline,
    vertices: &'a mut Vec<Vertex>,
    bind_group: &'a wgpu::BindGroup,
    buffer: wgpu::Buffer,
}

impl<'a> RenderBuffer<'a> {
    /// Do the render
    pub fn render(&'a self, rpass: &mut wgpu::RenderPass<'a>) {
        let count = self.vertices.len().cast();
        rpass.set_pipeline(self.pipe);
        rpass.set_bind_group(0, self.bind_group, &[]);
        rpass.set_vertex_buffer(0, self.buffer.slice(..));
        rpass.draw(0..count, 0..1);
    }
}

impl<'a> Drop for RenderBuffer<'a> {
    fn drop(&mut self) {
        self.vertices.clear();
    }
}

impl Pipeline {
    /// Construct
    pub fn new(device: &wgpu::Device, shaders: &ShaderManager) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("images bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None, // TODO
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        filtering: false,
                        comparison: false,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("images pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("images render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shaders.vert_2,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float3, 1 => Float2],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: wgpu::CullMode::Back,
                polygon_mode: wgpu::PolygonMode::Fill,
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shaders.frag_image,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    alpha_blend: wgpu::BlendState::REPLACE,
                    color_blend: wgpu::BlendState {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
        });

        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("loaded image"),
            size: wgpu::Extent3d {
                width: TEXTURE_SIZE.0,
                height: TEXTURE_SIZE.1,
                depth: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        });

        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("image sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Pipeline {
            bind_group_layout,
            render_pipeline,
            tex,
            view,
            sampler,
            image: image::ImageBuffer::from_raw(0, 0, Default::default()).unwrap(),
            need_write: false,
        }
    }

    /// Construct per-window state
    pub fn new_window(&self, device: &wgpu::Device, scale_buf: &wgpu::Buffer) -> Window {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("images bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &scale_buf,
                        offset: 0,
                        size: None,
                    },
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        Window {
            bind_group,
            passes: vec![],
        }
    }

    /// Load an image
    pub fn load(&mut self, path: &Path) {
        // TODO(opt): we convert to RGBA8 since this is the only format common
        // to both the image crate and WGPU. It may not be optimal however.
        // It also assumes that the image colour space is sRGB.
        match image::io::Reader::open(path)
            .map_err(|e| image::error::ImageError::IoError(e))
            .and_then(|r| r.decode())
        {
            Ok(image) => {
                self.image = image.into_rgba8();
                self.need_write = true;
            }
            Err(error) => {
                warn!("Loading image \"{}\" failed:", path.display());
                crate::warn_about_error("Cause", &error);
            }
        }
    }

    /// Query image size
    pub fn image_size(&self) -> (u32, u32) {
        self.image.dimensions()
    }

    /// Prepare textures
    pub fn prepare(&mut self, _: &wgpu::Device, queue: &wgpu::Queue) {
        if self.need_write {
            self.need_write = false;

            let size = self.image.dimensions();
            assert!(size.0 <= TEXTURE_SIZE.0 && size.1 <= TEXTURE_SIZE.1);
            queue.write_texture(
                wgpu::TextureCopyView {
                    texture: &self.tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                &self.image,
                wgpu::TextureDataLayout {
                    offset: 0,
                    bytes_per_row: 4 * size.0,
                    rows_per_image: size.1,
                },
                wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth: 1,
                },
            );
        }
    }

    /// Construct a render buffer
    pub fn render_buf<'a>(
        &'a self,
        window: &'a mut Window,
        device: &wgpu::Device,
        pass: usize,
    ) -> Option<RenderBuffer<'a>> {
        if pass >= window.passes.len() || window.passes[pass].len() == 0 {
            return None;
        }

        let vertices = &mut window.passes[pass];
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("images render_buf"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsage::VERTEX,
        });

        Some(RenderBuffer {
            pipe: &self.render_pipeline,
            vertices,
            bind_group: &window.bind_group,
            buffer,
        })
    }
}

impl Window {
    /// Add a rectangle to the buffer
    pub fn rect(&mut self, pipe: &Pipeline, pass: Pass, rect: Quad) {
        let aa = rect.a;
        let bb = rect.b;

        if !aa.lt(bb) {
            // zero / negative size: nothing to draw
            return;
        }

        let depth = pass.depth();
        let ab = Vec3(aa.0, bb.1, depth);
        let ba = Vec3(bb.0, aa.1, depth);
        let aa = Vec3::from2(aa, depth);
        let bb = Vec3::from2(bb, depth);

        let taa = Vec2::ZERO;
        let tbb = Vec2::from(Size::from(pipe.image_size())) / Vec2::from(Size::from(TEXTURE_SIZE));
        let tab = Vec2(taa.0, tbb.1);
        let tba = Vec2(tbb.0, taa.1);

        #[rustfmt::skip]
        self.add_vertices(pass.pass(), &[
            Vertex(aa, taa), Vertex(ba, tba), Vertex(ab, tab),
            Vertex(ab, tab), Vertex(ba, tba), Vertex(bb, tbb),
        ]);
    }

    fn add_vertices(&mut self, pass: usize, slice: &[Vertex]) {
        if self.passes.len() <= pass {
            // We only need one more, but no harm in adding extra
            self.passes.resize(pass + 8, vec![]);
        }

        self.passes[pass].extend_from_slice(slice);
    }
}
