// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Images pipeline

use kas::draw::color::Rgba;
use std::collections::HashMap;
use std::mem::size_of;

use super::{ShaderManager, atlases};
use kas::cast::Conv;
use kas::draw::{AllocError, Allocation, Allocator, ImageFormat, ImageId, PassId};
use kas::geom::{Quad, Vec2};
use kas::text::raster::{RenderQueue, Sprite, SpriteAllocator, SpriteType, UnpreparedSprite};

#[derive(Debug)]
struct Image {
    size: (u32, u32),
    alloc: Allocation,
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
                texture: atlas_rgba.get_texture(self.alloc.atlas),
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: self.alloc.origin.0,
                    y: self.alloc.origin.1,
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
    pub(super) a: Vec2,
    pub(super) b: Vec2,
    pub(super) ta: Vec2,
    pub(super) tb: Vec2,
}
unsafe impl bytemuck::Zeroable for InstanceRgba {}
unsafe impl bytemuck::Pod for InstanceRgba {}

/// Screen and texture coordinates (8-bit coverage mask)
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct InstanceMask {
    pub(super) a: Vec2,
    pub(super) b: Vec2,
    pub(super) ta: Vec2,
    pub(super) tb: Vec2,
    pub(super) col: Rgba,
}

unsafe impl bytemuck::Zeroable for InstanceMask {}
unsafe impl bytemuck::Pod for InstanceMask {}

/// Image loader and storage
pub struct Images {
    pub(super) atlas_rgba: atlases::Pipeline<InstanceRgba>,
    pub(super) atlas_mask: atlases::Pipeline<InstanceMask>,
    pub(super) atlas_rgba_mask: Option<atlases::Pipeline<InstanceMask>>,
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
        let atlas_rgba = atlases::Pipeline::new(
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

        let atlas_mask = atlases::Pipeline::new(
            device,
            Some("text pipe"),
            bgl_common,
            512,
            wgpu::TextureFormat::R8Unorm,
            wgpu::VertexState {
                module: &shaders.vert_glyph,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<InstanceMask>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2,
                        1 => Float32x2,
                        2 => Float32x2,
                        3 => Float32x2,
                        4 => Float32x4,
                    ],
                }],
            },
            wgpu::FragmentState {
                module: &shaders.frag_glyph,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: super::RENDER_TEX_FORMAT,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            },
        );

        let atlas_rgba_mask = None;
        Images {
            atlas_rgba,
            atlas_mask,
            atlas_rgba_mask,
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
        let alloc = self.atlas_rgba.allocate(size)?;
        let image = Image { size, alloc };
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
            self.atlas_rgba.deallocate(im.alloc.atlas, im.alloc.alloc);
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
        queue: &wgpu::Queue,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
        text: &mut kas::text::raster::State,
    ) {
        self.atlas_mask.prepare(device);
        if let Some(pipeline) = self.atlas_rgba_mask.as_mut() {
            pipeline.prepare(device);
        }

        let unprepared = text.unprepared_sprites();
        if !unprepared.is_empty() {
            log::trace!("prepare: uploading {} sprites", unprepared.len());
        }
        for UnpreparedSprite {
            atlas,
            ty,
            origin,
            size,
            data,
        } in unprepared.drain(..)
        {
            let texture;
            let texel_layout = match ty {
                SpriteType::Mask => {
                    texture = self.atlas_mask.get_texture(atlas);
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(size.0),
                        rows_per_image: Some(size.1),
                    }
                }
                SpriteType::Bitmap => {
                    texture = self.atlas_rgba.get_texture(atlas);
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * size.0),
                        rows_per_image: Some(size.1),
                    }
                }
            };

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: origin.0,
                        y: origin.1,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &data,
                texel_layout,
                wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
            );
        }

        window.write_buffers(device, staging_belt, encoder);
    }

    /// Get atlas and texture coordinates for an image
    pub fn get_im_atlas_coords(&self, id: ImageId) -> Option<(u32, Quad)> {
        self.images
            .get(&id)
            .map(|im| (im.alloc.atlas, im.alloc.tex_quad))
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
        self.atlas_mask
            .render(&window.atlas_mask, pass, rpass, bg_common);
    }
}

impl SpriteAllocator for Images {
    fn query_subpixel_rendering(&self) -> bool {
        self.atlas_rgba_mask.is_some()
    }

    fn alloc_mask(&mut self, size: (u32, u32)) -> Result<Allocation, AllocError> {
        self.atlas_mask.allocate(size)
    }

    fn alloc_rgba_mask(&mut self, size: (u32, u32)) -> Result<Allocation, AllocError> {
        self.atlas_rgba_mask
            .as_mut()
            .expect("subpixel rendering feature is unavailable")
            .allocate(size)
    }

    fn alloc_rgba(&mut self, size: (u32, u32)) -> Result<Allocation, AllocError> {
        self.atlas_rgba.allocate(size)
    }
}

#[derive(Debug, Default)]
pub struct Window {
    pub(super) atlas_rgba: atlases::Window<InstanceRgba>,
    pub(super) atlas_mask: atlases::Window<InstanceMask>,
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
        self.atlas_mask.write_buffers(device, staging_belt, encoder);
    }

    /// Add a rectangle to the buffer
    pub fn rect(&mut self, pass: PassId, atlas: u32, tex: Quad, rect: Quad) {
        if !(rect.a < rect.b) {
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

impl RenderQueue for Window {
    fn push_sprite(
        &mut self,
        pass: PassId,
        glyph_pos: Vec2,
        rect: Quad,
        col: Rgba,
        sprite: &Sprite,
    ) {
        let mut a = glyph_pos.floor() + sprite.offset;
        let mut b = a + sprite.size;

        let Some(ty) = sprite.ty else {
            return;
        };
        if !(a.0 < rect.b.0 && a.1 < rect.b.1 && b.0 > rect.a.0 && b.1 > rect.a.1) {
            return;
        }

        let (mut ta, mut tb) = (sprite.tex_quad.a, sprite.tex_quad.b);
        if !(a >= rect.a) || !(b <= rect.b) {
            let size_inv = Vec2::splat(1.0) / (b - a);
            let fa0 = 0f32.max((rect.a.0 - a.0) * size_inv.0);
            let fa1 = 0f32.max((rect.a.1 - a.1) * size_inv.1);
            let fb0 = 1f32.min((rect.b.0 - a.0) * size_inv.0);
            let fb1 = 1f32.min((rect.b.1 - a.1) * size_inv.1);

            let ts = tb - ta;
            tb = ta + ts * Vec2(fb0, fb1);
            ta += ts * Vec2(fa0, fa1);

            a.0 = a.0.clamp(rect.a.0, rect.b.0);
            a.1 = a.1.clamp(rect.a.1, rect.b.1);
            b.0 = b.0.clamp(rect.a.0, rect.b.0);
            b.1 = b.1.clamp(rect.a.1, rect.b.1);
        }

        match ty {
            SpriteType::Mask => {
                let instance = InstanceMask { a, b, ta, tb, col };
                self.atlas_mask.rect(pass, sprite.atlas, instance);
            }
            SpriteType::Bitmap => {
                let instance = InstanceRgba { a, b, ta, tb };
                self.atlas_rgba.rect(pass, sprite.atlas, instance);
            }
        }
    }
}
