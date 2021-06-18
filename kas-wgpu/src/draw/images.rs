// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Images pipeline

use guillotiere::AllocId;
use image::RgbaImage;
use std::collections::HashMap;
use std::mem::size_of;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use thiserror::Error;

use super::{atlases, ShaderManager};
use kas::draw::{ImageId, Pass};
use kas::geom::{Quad, Size, Vec2};

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

#[derive(Debug)]
pub struct Image {
    image: RgbaImage,
    atlas: u32,
    alloc: AllocId,
    tex_quad: Quad,
}

impl Image {
    /// Load an image
    pub fn load_path(
        atlas_pipe: &mut atlases::Pipeline<Instance>,
        path: &Path,
    ) -> Result<(Self, (u32, u32)), ImageError> {
        let image = image::io::Reader::open(path)?.decode()?;
        // TODO(opt): we convert to RGBA8 since this is the only format common
        // to both the image crate and WGPU. It may not be optimal however.
        // It also assumes that the image colour space is sRGB.
        let image = image.into_rgba8();
        let size = image.dimensions();

        let (atlas, alloc, origin, tex_quad) = atlas_pipe.allocate(size)?;

        let image = Image {
            image,
            atlas: atlas,
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
    pub fn write_to_tex(
        &mut self,
        atlas_pipe: &atlases::Pipeline<Instance>,
        origin: (u32, u32),
        queue: &wgpu::Queue,
    ) {
        // TODO(opt): use StagingBelt for upload (when supported)? Or our own equivalent.
        let size = self.image.dimensions();
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: atlas_pipe.get_texture(self.atlas),
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: origin.0,
                    y: origin.1,
                    z: 0,
                },
            },
            &self.image,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(4 * size.0),
                rows_per_image: NonZeroU32::new(size.1),
            },
            wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn tex_quad(&self) -> Quad {
        self.tex_quad
    }
}

/// Screen and texture coordinates
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Instance {
    a: Vec2,
    b: Vec2,
    ta: Vec2,
    tb: Vec2,
}
unsafe impl bytemuck::Zeroable for Instance {}
unsafe impl bytemuck::Pod for Instance {}

/// Image loader and storage
pub struct Images {
    atlas_pipe: atlases::Pipeline<Instance>,
    last_image_n: u32,
    paths: HashMap<PathBuf, (ImageId, u32)>,
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
            &bgl_common,
            2048,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::VertexState {
                module: &shaders.vert_image,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<Instance>() as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Instance,
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
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: super::RENDER_TEX_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            },
        );
        Images {
            atlas_pipe,
            last_image_n: 0,
            paths: Default::default(),
            images: Default::default(),
        }
    }

    fn next_image_id(&mut self) -> ImageId {
        let n = self.last_image_n.wrapping_add(1);
        self.last_image_n = n;
        ImageId::try_new(n).expect("exhausted image IDs")
    }

    /// Load an image from the file-system
    pub fn load_path(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &Path,
    ) -> Result<ImageId, Box<dyn std::error::Error + 'static>> {
        if let Some((id, ref mut count)) = self.paths.get_mut(path) {
            *count += 1;
            return Ok(*id);
        }

        let id = self.next_image_id();
        let (image, origin) = Image::load_path(&mut self.atlas_pipe, path)?;
        self.images.insert(id, image);

        self.atlas_pipe.prepare(device);
        if let Some(image) = self.images.get_mut(&id) {
            image.write_to_tex(&self.atlas_pipe, origin, queue);
        }

        self.paths.insert(path.to_owned(), (id, 1));

        Ok(id)
    }

    /// Reduce usage count for `id` and free if zero
    pub fn remove(&mut self, id: ImageId) {
        // We don't have a map from id to path, hence have to iterate. We can
        // however do a fast check that id is used.
        if !self.images.contains_key(&id) {
            return;
        }

        let mut ref_count = 0;
        self.paths.retain(|_, obj| {
            if obj.0 == id {
                obj.1 -= 1;
                ref_count = obj.1;
                ref_count != 0
            } else {
                true
            }
        });

        if ref_count == 0 {
            if let Some(im) = self.images.remove(&id) {
                self.atlas_pipe.deallocate(im.atlas, im.alloc);
            }
        }
    }

    /// Query image size
    pub fn image_size(&self, id: ImageId) -> Option<Size> {
        self.images.get(&id).map(|im| im.size().into())
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
        self.images.get(&id).map(|im| (im.atlas, im.tex_quad()))
    }

    /// Enqueue render commands
    pub fn render<'a>(
        &'a self,
        window: &'a Window,
        pass: usize,
        rpass: &mut wgpu::RenderPass<'a>,
        bg_common: &'a wgpu::BindGroup,
    ) {
        self.atlas_pipe
            .render(&window.atlas, pass, rpass, bg_common);
    }
}

#[derive(Debug, Default)]
pub struct Window {
    atlas: atlases::Window<Instance>,
}

impl Window {
    /// Prepare vertex buffers
    pub fn write_buffers(
        &mut self,
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        self.atlas.write_buffers(device, staging_belt, encoder);
    }

    /// Add a rectangle to the buffer
    pub fn rect(&mut self, pass: Pass, atlas: u32, tex: Quad, rect: Quad) {
        if !rect.a.lt(rect.b) {
            // zero / negative size: nothing to draw
            return;
        }

        let instance = Instance {
            a: rect.a,
            b: rect.b,
            ta: tex.a,
            tb: tex.b,
        };
        self.atlas.rect(pass, atlas, instance);
    }
}
