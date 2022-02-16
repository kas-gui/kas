// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Images pipeline

use guillotiere::{AllocId, Allocation, AtlasAllocator};
use std::mem::size_of;
use std::num::NonZeroU64;
use std::ops::Range;
use thiserror::Error;

use kas::cast::{Cast, Conv};
use kas::draw::{ImageError, PassId};
use kas::geom::{Quad, Size, Vec2};
use kas::macros::autoimpl;

fn to_vec2(p: guillotiere::Point) -> Vec2 {
    Vec2(p.x.cast(), p.y.cast())
}

// TODO
/// Allocation failed: too large or zero sized
#[derive(Error, Debug)]
#[error("failed to allocate: size too large or zero-sized")]
pub struct AllocError;

impl From<AllocError> for ImageError {
    fn from(_: AllocError) -> ImageError {
        ImageError::Allocation
    }
}

pub struct Atlas {
    alloc: AtlasAllocator,
    tex: wgpu::Texture,
    bg: wgpu::BindGroup,
}

impl Atlas {
    /// Construct a new allocator
    pub fn new_alloc(size: (i32, i32)) -> AtlasAllocator {
        AtlasAllocator::new(size.into())
    }

    /// Construct from an allocator
    pub fn new(
        alloc: AtlasAllocator,
        device: &wgpu::Device,
        bg_tex_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        format: wgpu::TextureFormat,
    ) -> Self {
        let size = alloc.size();
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("loaded image"),
            size: wgpu::Extent3d {
                width: size.width.cast(),
                height: size.height.cast(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        });

        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());

        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atlas texture bind group"),
            layout: bg_tex_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });

        Atlas { alloc, tex, bg }
    }
}

/// A pipeline for rendering from image atlases
pub struct Pipeline<I: bytemuck::Pod> {
    tex_size: i32,
    tex_format: wgpu::TextureFormat,
    bg_tex_layout: wgpu::BindGroupLayout,
    render_pipeline: wgpu::RenderPipeline,
    atlases: Vec<Atlas>,
    new_aa: Vec<AtlasAllocator>,
    sampler: wgpu::Sampler,
    _pd: std::marker::PhantomData<I>,
}

impl<I: bytemuck::Pod> Pipeline<I> {
    /// Construct
    ///
    /// Configuration:
    ///
    /// -   `tex_size`: side length of square texture atlases
    /// -   `tex_format`: texture format
    pub fn new(
        device: &wgpu::Device,
        bg_common: &wgpu::BindGroupLayout,
        tex_size: i32,
        tex_format: wgpu::TextureFormat,
        vertex: wgpu::VertexState,
        fragment: wgpu::FragmentState,
    ) -> Self {
        let bg_tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("atlas texture bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        filtering: false,
                        comparison: false,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("atlas pipeline layout"),
            bind_group_layouts: &[bg_common, &bg_tex_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("atlas render pipeline"),
            layout: Some(&pipeline_layout),
            vertex,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: Some(wgpu::Face::Back), // not required
                clamp_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(fragment),
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("image sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Pipeline {
            tex_size,
            tex_format,
            bg_tex_layout,
            render_pipeline,
            atlases: vec![],
            new_aa: vec![],
            sampler,
            _pd: Default::default(),
        }
    }

    fn allocate_space(
        &mut self,
        size: (i32, i32),
    ) -> Result<(u32, Allocation, (i32, i32)), AllocError> {
        let mut tex_size = (self.tex_size, self.tex_size);
        let size2d = size.into();
        let mut atlas = 0;
        while atlas < self.atlases.len() {
            if let Some(alloc) = self.atlases[atlas].alloc.allocate(size2d) {
                return Ok((atlas.cast(), alloc, tex_size));
            }
            atlas += 1;
        }

        // New_aa are atlas allocators which haven't been assigned textures yet
        for new_aa in &mut self.new_aa {
            if let Some(alloc) = new_aa.allocate(size2d) {
                return Ok((atlas.cast(), alloc, tex_size));
            }
            atlas += 1;
        }

        if size.0 > self.tex_size || size.1 > self.tex_size {
            // Too large to fit in a regular atlas: use a special allocation
            let max_supported = i32::conv(wgpu::Limits::default().max_texture_dimension_2d);
            if size.0 > max_supported || size.1 > max_supported {
                return Err(AllocError);
            }
            tex_size = size;
        }

        self.new_aa.push(Atlas::new_alloc(tex_size));
        match self.new_aa.last_mut().unwrap().allocate(size2d) {
            Some(alloc) => Ok((atlas.cast(), alloc, tex_size)),
            None => unreachable!(),
        }
    }

    /// Allocate space within a texture atlas
    ///
    /// Fails if `size` is zero in any dimension.
    ///
    /// On success, returns:
    ///
    /// -   `atlas` number
    /// -   allocation identifier within the atlas
    /// -   `origin` within texture (integer coordinates, for use when uploading)
    /// -   texture coordinates (for use when drawing)
    pub fn allocate(
        &mut self,
        size: (u32, u32),
    ) -> Result<(u32, AllocId, (u32, u32), Quad), AllocError> {
        if size.0 == 0 || size.1 == 0 {
            return Err(AllocError);
        }

        let (atlas, alloc, tex_size) = self.allocate_space((size.0.cast(), size.1.cast()))?;

        let origin = (alloc.rectangle.min.x.cast(), alloc.rectangle.min.y.cast());

        let tex_size = Vec2::conv(Size::from(tex_size));
        let a = to_vec2(alloc.rectangle.min) / tex_size;
        let b = to_vec2(alloc.rectangle.max) / tex_size;
        debug_assert!(Vec2::ZERO.le(a) && a.le(b) && b.le(Vec2::splat(1.0)));
        let tex_quad = Quad { a, b };

        Ok((atlas, alloc.id, origin, tex_quad))
    }

    pub fn deallocate(&mut self, atlas: u32, alloc: AllocId) {
        self.atlases[usize::conv(atlas)].alloc.deallocate(alloc);
    }

    /// Prepare textures
    pub fn prepare(&mut self, device: &wgpu::Device) {
        for alloc in self.new_aa.drain(..) {
            let atlas = Atlas::new(
                alloc,
                device,
                &self.bg_tex_layout,
                &self.sampler,
                self.tex_format,
            );
            self.atlases.push(atlas);
        }
    }

    pub fn get_texture(&self, atlas: u32) -> &wgpu::Texture {
        &self.atlases[usize::conv(atlas)].tex
    }

    /// Enqueue render commands
    pub fn render<'a>(
        &'a self,
        window: &'a Window<I>,
        pass: usize,
        rpass: &mut wgpu::RenderPass<'a>,
        bg_common: &'a wgpu::BindGroup,
    ) {
        if let Some(buffer) = window.buffer.as_ref() {
            if let Some(pass) = window.passes.get(pass) {
                if pass.data_range.is_empty() {
                    return;
                }
                rpass.set_pipeline(&self.render_pipeline);
                rpass.set_bind_group(0, bg_common, &[]);
                rpass.set_vertex_buffer(0, buffer.slice(pass.data_range.clone()));
                for (a, atlas) in pass.atlases.iter().enumerate() {
                    if !atlas.range.is_empty() {
                        rpass.set_bind_group(1, &self.atlases[a].bg, &[]);
                        rpass.draw(0..4, atlas.range.clone());
                    }
                }
            }
        }
    }
}

#[autoimpl(Default)]
#[derive(Clone, Debug)]
struct AtlasPassData<I: bytemuck::Pod> {
    instances: Vec<I>,
    range: Range<u32>,
}

#[autoimpl(Default)]
#[derive(Clone, Debug)]
struct PassData<I: bytemuck::Pod> {
    atlases: Vec<AtlasPassData<I>>,
    data_range: Range<u64>,
}

/// Per-window state
#[autoimpl(Default)]
#[derive(Debug)]
pub struct Window<I: bytemuck::Pod> {
    passes: Vec<PassData<I>>,
    buffer: Option<wgpu::Buffer>,
    buffer_size: u64,
}

impl<I: bytemuck::Pod> Window<I> {
    /// Prepare vertex buffers
    pub fn write_buffers(
        &mut self,
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let mut req_len = 0;
        for pass in self.passes.iter() {
            for atlas in pass.atlases.iter() {
                req_len += u64::conv(atlas.instances.len() * size_of::<I>());
            }
        }
        let byte_len = match NonZeroU64::new(req_len) {
            Some(nz) => nz,
            None => {
                for pass in self.passes.iter_mut() {
                    for atlas in pass.atlases.iter_mut() {
                        atlas.range = 0..0;
                    }
                }
                return;
            }
        };

        if req_len <= self.buffer_size {
            let buffer = self.buffer.as_ref().unwrap();
            let mut slice = staging_belt.write_buffer(encoder, buffer, 0, byte_len, device);
            copy_to_slice(&mut self.passes, &mut slice);
        } else {
            // Size must be a multiple of alignment
            let mask = wgpu::COPY_BUFFER_ALIGNMENT - 1;
            let buffer_size = (byte_len.get() + mask) & !mask;
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("atlases vertex buffer"),
                size: buffer_size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: true,
            });

            let mut slice = buffer.slice(..byte_len.get()).get_mapped_range_mut();
            copy_to_slice(&mut self.passes, &mut slice);
            drop(slice);

            buffer.unmap();
            self.buffer = Some(buffer);
            self.buffer_size = buffer_size;
        }

        fn copy_to_slice<I: bytemuck::Pod>(passes: &mut [PassData<I>], slice: &mut [u8]) {
            let mut byte_offset = 0;
            for pass in passes.iter_mut() {
                let byte_start = byte_offset;
                let mut index = 0;
                for atlas in pass.atlases.iter_mut() {
                    let len = u32::conv(atlas.instances.len());
                    let byte_len = u64::from(len) * u64::conv(size_of::<I>());
                    let byte_end = byte_offset + byte_len;

                    slice[usize::conv(byte_offset)..usize::conv(byte_end)]
                        .copy_from_slice(bytemuck::cast_slice(&atlas.instances));

                    byte_offset = byte_end;
                    atlas.instances.clear();
                    let end = index + len;
                    atlas.range = index..end;
                    index = end;
                }
                pass.data_range = byte_start..byte_offset;
            }
        }
    }

    /// Add a rectangle to the buffer
    pub fn rect(&mut self, pass: PassId, atlas: u32, instance: I) {
        let pass = pass.pass();
        if self.passes.len() <= pass {
            // We only need one more, but no harm in adding extra
            self.passes.resize(pass + 8, Default::default());
        }
        let pass = &mut self.passes[pass];

        let atlas = usize::conv(atlas);
        if pass.atlases.len() <= atlas {
            // Warning: length must not excced number of atlases
            pass.atlases.resize(atlas + 1, Default::default());
        }

        pass.atlases[atlas].instances.push(instance);
    }
}
