// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Images pipeline

use guillotiere::{AllocId, Allocation, AtlasAllocator};
use image::RgbaImage;
use std::collections::HashMap;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use thiserror::Error;
use wgpu::util::DeviceExt;

use crate::draw::ShaderManager;
use kas::cast::{Cast, Conv};
use kas::draw::{ImageId, Pass};
use kas::geom::{Quad, Size, Vec2, Vec3};

const TEXTURE_SIZE: (u32, u32) = (2048, 2048);

fn to_vec2(p: guillotiere::Point) -> Vec2 {
    Vec2(p.x.cast(), p.y.cast())
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Vertex(Vec3, Vec2);
unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

/// Image loading errors
#[derive(Error, Debug)]
pub enum ImageError {
    #[error("IO error")]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    Image(#[from] image::ImageError),
    #[error("allocation failed: insufficient space in image atlas")]
    Allocation,
}

pub struct Atlas {
    alloc: AtlasAllocator,
    tex: wgpu::Texture,
    bg: wgpu::BindGroup,
}

impl Atlas {
    /// Are image dimensions too large to fit in an Atlas?
    pub fn is_too_big(size: (u32, u32)) -> bool {
        size.0 > TEXTURE_SIZE.0 || size.1 > TEXTURE_SIZE.1
    }

    /// Construct a new allocator
    pub fn new_alloc() -> AtlasAllocator {
        let size_i32: (i32, i32) = (TEXTURE_SIZE.0.cast(), TEXTURE_SIZE.1.cast());
        AtlasAllocator::new(size_i32.into())
    }

    /// Construct from an allocator
    pub fn new(
        alloc: AtlasAllocator,
        device: &wgpu::Device,
        bg_tex_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
    ) -> Self {
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

        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image atlas bind group"),
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

#[derive(Debug)]
pub struct Image {
    image: RgbaImage,
    atlas: u32,
    alloc: AllocId,
    tex_quad: Quad,
}

impl Image {
    /// Load an image
    pub fn load_path(pipe: &mut Pipeline, path: &Path) -> Result<(Self, (u32, u32)), ImageError> {
        let image = image::io::Reader::open(path)?.decode()?;
        // TODO(opt): we convert to RGBA8 since this is the only format common
        // to both the image crate and WGPU. It may not be optimal however.
        // It also assumes that the image colour space is sRGB.
        let image = image.into_rgba8();
        let size = image.dimensions();

        let (atlas, alloc) = pipe.allocate(size)?;
        let atlas = u32::conv(atlas);

        let tex_size = Vec2::from(Size::from(TEXTURE_SIZE));
        let a = to_vec2(alloc.rectangle.min) / tex_size;
        let b = to_vec2(alloc.rectangle.max) / tex_size;
        debug_assert!(Vec2::ZERO.le(a) && a.le(b) && b.le(Vec2::splat(1.0)));
        let tex_quad = Quad { a, b };

        let origin = (alloc.rectangle.min.x.cast(), alloc.rectangle.min.y.cast());
        let alloc = alloc.id;
        let image = Image {
            image,
            atlas,
            alloc,
            tex_quad,
        };
        log::debug!(
            "Image: atlas={}, alloc={:?}, tex_quad={:?}",
            image.atlas,
            image.alloc,
            image.tex_quad
        );
        Ok((image, origin))
    }

    /// Query image size
    pub fn size(&self) -> (u32, u32) {
        self.image.dimensions()
    }

    /// Prepare textures
    pub fn write_to_tex(&mut self, atlases: &[Atlas], origin: (u32, u32), queue: &wgpu::Queue) {
        let size = self.image.dimensions();
        queue.write_texture(
            wgpu::TextureCopyView {
                texture: &atlases[usize::conv(self.atlas)].tex,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: origin.0,
                    y: origin.1,
                    z: 0,
                },
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

    pub fn tex_quad(&self) -> Quad {
        self.tex_quad
    }
}

/// A pipeline for rendering images
pub struct Pipeline {
    bg_tex_layout: wgpu::BindGroupLayout,
    render_pipeline: wgpu::RenderPipeline,
    atlases: Vec<Atlas>,
    new_aa: Vec<AtlasAllocator>,
    sampler: wgpu::Sampler,
    last_image_n: u32,
    paths: HashMap<PathBuf, (ImageId, u32)>,
    images: HashMap<ImageId, Image>,
    prepare: Vec<(ImageId, (u32, u32))>,
}

/// Buffer used during render pass
///
/// This buffer must not be dropped before the render pass.
pub struct RenderBuffer<'a> {
    pipe: &'a Pipeline,
    buffers: Vec<(u32, wgpu::Buffer)>,
}

impl<'a> RenderBuffer<'a> {
    /// Do the render
    pub fn render(&'a self, rpass: &mut wgpu::RenderPass<'a>, bg_common: &'a wgpu::BindGroup) {
        for (atlas, (count, buffer)) in self.buffers.iter().enumerate() {
            rpass.set_pipeline(&self.pipe.render_pipeline);
            rpass.set_bind_group(0, bg_common, &[]);
            rpass.set_bind_group(1, &self.pipe.atlases[atlas].bg, &[]);
            rpass.set_vertex_buffer(0, buffer.slice(..));
            rpass.draw(0..*count, 0..1);
        }
    }
}

impl Pipeline {
    /// Construct
    pub fn new(
        device: &wgpu::Device,
        shaders: &ShaderManager,
        bg_common: &wgpu::BindGroupLayout,
    ) -> Self {
        let bg_tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("images texture bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
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
            label: Some("images pipeline layout"),
            bind_group_layouts: &[bg_common, &bg_tex_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("images render pipeline"),
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

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("image sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Pipeline {
            bg_tex_layout,
            render_pipeline,
            atlases: vec![],
            new_aa: vec![],
            sampler,
            last_image_n: 0,
            paths: Default::default(),
            images: Default::default(),
            prepare: vec![],
        }
    }

    /// Construct per-window state
    pub fn new_window(&self) -> Window {
        Window { passes: vec![] }
    }

    fn allocate(&mut self, size: (u32, u32)) -> Result<(usize, Allocation), ImageError> {
        if Atlas::is_too_big(size) {
            return Err(ImageError::Allocation);
        }
        let size = (i32::conv(size.0), i32::conv(size.1)).into();

        let mut atlas = 0;
        while atlas < self.atlases.len() {
            if let Some(alloc) = self.atlases[atlas].alloc.allocate(size) {
                return Ok((atlas, alloc));
            }
            atlas += 1;
        }

        // New_aa are atlas allocators which haven't been assigned textures yet
        for new_aa in &mut self.new_aa {
            if let Some(alloc) = new_aa.allocate(size) {
                return Ok((atlas, alloc));
            }
            atlas += 1;
        }

        self.new_aa.push(Atlas::new_alloc());
        match self.new_aa.last_mut().unwrap().allocate(size) {
            Some(alloc) => return Ok((atlas, alloc)),
            None => unreachable!(),
        }
    }

    fn next_image_id(&mut self) -> ImageId {
        let n = self.last_image_n.wrapping_add(1);
        self.last_image_n = n;
        ImageId::try_new(n).expect("exhausted image IDs")
    }

    /// Load an image
    pub fn load_path(
        &mut self,
        path: &Path,
    ) -> Result<ImageId, Box<dyn std::error::Error + 'static>> {
        if let Some((id, _)) = self.paths.get(path) {
            return Ok(*id);
        }

        let id = self.next_image_id();
        let (image, origin) = Image::load_path(self, path)?;
        self.images.insert(id, image);
        self.prepare.push((id, origin));
        self.paths.insert(path.to_owned(), (id, 1));

        Ok(id)
    }

    /// Free an image
    pub fn remove(&mut self, id: ImageId) {
        // We don't have a map from id to path, hence have to iterate. We can
        // however do a fast check that id is used.
        if !self.images.contains_key(&id) {
            return;
        }

        let atlases = &mut self.atlases;
        let images = &mut self.images;
        self.paths.retain(|_, obj| {
            if obj.0 == id {
                obj.1 -= 1;
                if obj.1 == 0 {
                    if let Some(im) = images.remove(&id) {
                        atlases[usize::conv(im.atlas)].alloc.deallocate(im.alloc);
                    }
                    return false;
                }
            }
            true
        })
    }

    /// Query image size
    pub fn image_size(&self, id: ImageId) -> Option<Size> {
        self.images.get(&id).map(|im| im.size().into())
    }

    /// Prepare textures
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        for alloc in self.new_aa.drain(..) {
            let atlas = Atlas::new(alloc, device, &self.bg_tex_layout, &self.sampler);
            self.atlases.push(atlas);
        }

        for (id, origin) in self.prepare.drain(..) {
            if let Some(image) = self.images.get_mut(&id) {
                image.write_to_tex(&mut self.atlases, origin, queue);
            }
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

        let buffers = window.passes[pass]
            .iter()
            .map(|vertices| {
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("image atlas render buffer"),
                    contents: bytemuck::cast_slice(vertices),
                    usage: wgpu::BufferUsage::VERTEX,
                });
                (vertices.len().cast(), buffer)
            })
            .collect();

        Some(RenderBuffer {
            pipe: &self,
            buffers,
        })
    }
}

/// Per-window state
#[derive(Clone, Debug)]
pub struct Window {
    // per pass, per atlas, a list of queued vertices
    passes: Vec<Vec<Vec<Vertex>>>,
}

impl Window {
    /// Used after rendering to clear queued vertices
    pub fn clear_vertices(&mut self) {
        for pass_queue in self.passes.iter_mut() {
            for atlas_queue in pass_queue.iter_mut() {
                atlas_queue.clear();
            }
        }
    }

    /// Add a rectangle to the buffer
    pub fn rect(&mut self, pipe: &Pipeline, pass: Pass, id: ImageId, rect: Quad) {
        let aa = rect.a;
        let bb = rect.b;

        if !aa.lt(bb) {
            // zero / negative size: nothing to draw
            return;
        }

        let (atlas, t) = match pipe.images.get(&id) {
            Some(im) => (im.atlas, im.tex_quad()),
            None => return,
        };

        let depth = pass.depth();
        let ab = Vec3(aa.0, bb.1, depth);
        let ba = Vec3(bb.0, aa.1, depth);
        let aa = Vec3::from2(aa, depth);
        let bb = Vec3::from2(bb, depth);

        let tab = Vec2(t.a.0, t.b.1);
        let tba = Vec2(t.b.0, t.a.1);

        let num_atlases = pipe.atlases.len() + pipe.new_aa.len();
        #[rustfmt::skip]
        self.add_vertices(pass.pass(), num_atlases, atlas, &[
            Vertex(aa, t.a), Vertex(ba, tba), Vertex(ab, tab),
            Vertex(ab, tab), Vertex(ba, tba), Vertex(bb, t.b),
        ]);
    }

    fn add_vertices(&mut self, pass: usize, num_atlases: usize, atlas: u32, slice: &[Vertex]) {
        if self.passes.len() <= pass {
            // We only need one more, but no harm in adding extra
            self.passes.resize(pass + 8, vec![]);
        }
        let pass = &mut self.passes[pass];

        let atlas = usize::conv(atlas);
        assert!(atlas < num_atlases);
        pass.resize(num_atlases, vec![]);

        pass[atlas].extend_from_slice(slice);
    }
}
