// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Images pipeline

use guillotiere::AllocId;
use std::collections::HashMap;
use std::mem::size_of;

use super::{atlases, ShaderManager};
use kas::cast::Conv;
use kas::draw::{AllocError, ImageFormat, ImageId, PassId};
use kas::geom::{Quad, Vec2};

#[derive(Debug)]
struct Image {
    atlas: u32,
    alloc: AllocId,
    size: (u32, u32),
    origin: (u32, u32),
    tex_quad: Quad,
}

impl Image {
    fn upload(
        &mut self,
        atlas_rgba: &atlases::Pipeline<InstanceRgba>,
        queue: &wgpu::Queue,
        data: &[u8],
    ) {
        // TODO(opt): use StagingBelt for upload (when supported)? Or our own equivalent.
        let size = self.size;
        assert!(!data.is_empty());
        assert_eq!(data.len(), 4 * usize::conv(size.0) * usize::conv(size.1));
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: atlas_rgba.get_texture(self.atlas),
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: self.origin.0,
                    y: self.origin.1,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * size.0),
                rows_per_image: Some(size.1),
            },
            wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
        );
    }
}

/// Screen and texture coordinates
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct InstanceRgba {
    a: Vec2,
    b: Vec2,
    ta: Vec2,
    tb: Vec2,
}
unsafe impl bytemuck::Zeroable for InstanceRgba {}
unsafe impl bytemuck::Pod for InstanceRgba {}

/// Image loader and storage
pub struct Images {
    atlas_rgba: atlases::Pipeline<InstanceRgba>,
    last_image_n: u32,
    images: HashMap<ImageId, Image>,
}

impl Images {
    /// Construct
    pub fn new(
        device: &wgpu::Device,
        shaders: &ShaderManager,
        bgl_common: &wgpu::BindGroupLayout,
    ) -> Self {
        let atlas_pipe = atlases::Pipeline::new(
            device,
            Some("images pipe"),
            bgl_common,
            2048,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::VertexState {
                module: &shaders.vert_image,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<InstanceRgba>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2,
                        1 => Float32x2,
                        2 => Float32x2,
                        3 => Float32x2,
                    ],
                }],
            },
            wgpu::FragmentState {
                module: &shaders.frag_image,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: super::RENDER_TEX_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            },
        );
        Images {
            atlas_rgba: atlas_pipe,
            last_image_n: 0,
            images: Default::default(),
        }
    }

    fn next_image_id(&mut self) -> ImageId {
        let n = self.last_image_n.wrapping_add(1);
        self.last_image_n = n;
        ImageId::try_new(n).expect("exhausted image IDs")
    }

    /// Allocate an image
    pub fn alloc(&mut self, size: (u32, u32)) -> Result<ImageId, AllocError> {
        let id = self.next_image_id();
        let (atlas, alloc, origin, tex_quad) = self.atlas_rgba.allocate(size)?;
        let image = Image {
            atlas,
            alloc,
            size,
            origin,
            tex_quad,
        };
        self.images.insert(id, image);
        Ok(id)
    }

    /// Upload an image to the GPU
    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        id: ImageId,
        data: &[u8],
        format: ImageFormat,
    ) {
        // The atlas pipe allocates textures lazily. Ensure ours is ready:
        self.atlas_rgba.prepare(device);

        match format {
            ImageFormat::Rgba8 => (),
        }

        if let Some(image) = self.images.get_mut(&id) {
            image.upload(&self.atlas_rgba, queue, data);
        }
    }

    /// Free an image allocation
    pub fn free(&mut self, id: ImageId) {
        if let Some(im) = self.images.remove(&id) {
            self.atlas_rgba.deallocate(im.atlas, im.alloc);
        }
    }

    /// Query image size
    pub fn image_size(&self, id: ImageId) -> Option<(u32, u32)> {
        self.images.get(&id).map(|im| im.size)
    }

    /// Write to textures
    pub fn prepare(
        &mut self,
        window: &mut Window,
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        window.write_buffers(device, staging_belt, encoder);
    }

    /// Get atlas and texture coordinates for an image
    pub fn get_im_atlas_coords(&self, id: ImageId) -> Option<(u32, Quad)> {
        self.images.get(&id).map(|im| (im.atlas, im.tex_quad))
    }

    /// Enqueue render commands
    pub fn render<'a>(
        &'a self,
        window: &'a Window,
        pass: usize,
        rpass: &mut wgpu::RenderPass<'a>,
        bg_common: &'a wgpu::BindGroup,
    ) {
        self.atlas_rgba
            .render(&window.atlas_rgba, pass, rpass, bg_common);
    }
}

#[derive(Debug, Default)]
pub struct Window {
    atlas_rgba: atlases::Window<InstanceRgba>,
}

impl Window {
    /// Prepare vertex buffers
    pub fn write_buffers(
        &mut self,
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        self.atlas_rgba.write_buffers(device, staging_belt, encoder);
    }

    /// Add a rectangle to the buffer
    pub fn rect(&mut self, pass: PassId, atlas: u32, tex: Quad, rect: Quad) {
        if !rect.a.lt(rect.b) {
            // zero / negative size: nothing to draw
            return;
        }

        let instance = InstanceRgba {
            a: rect.a,
            b: rect.b,
            ta: tex.a,
            tb: tex.b,
        };
        self.atlas_rgba.rect(pass, atlas, instance);
    }
}
