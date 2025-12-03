// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Images pipeline

use guillotiere::{AllocId, AtlasAllocator};
use std::collections::HashMap;
use std::mem::size_of;
use std::num::NonZeroU64;
use std::ops::Range;

use kas::autoimpl;
use kas::cast::{Cast, CastFloat, Conv};
use kas::draw::{AllocError, Allocation, Allocator, ImageFormat, ImageId, PassId, color};
use kas::geom::{Quad, Size, Vec2};
use kas::text::raster::UnpreparedSprite;

fn to_vec2(p: guillotiere::Point) -> Vec2 {
    Vec2(p.x.cast(), p.y.cast())
}

/// One texture atlas
struct Atlas<T> {
    alloc: AtlasAllocator,
    tex: Box<[T]>,
}

impl<T: Clone + Default> Atlas<T> {
    /// Construct a new allocator
    fn new(size: (i32, i32)) -> Self {
        let alloc = AtlasAllocator::new(size.into());
        let len = usize::conv(size.0) * usize::conv(size.1);
        let tex = vec![T::default(); len].into_boxed_slice();

        Atlas { alloc, tex }
    }
}

/// Maximum permitted texture size on each side
pub const MAX_TEX_SIZE: i32 = 4096;

/// A set of texture atlases (shared between all windows)
struct Atlases<F: Format> {
    tex_size: i32,
    atlases: Vec<Atlas<F::C>>,
}

impl<F: Format> Atlases<F> {
    /// Construct
    ///
    /// Configuration:
    ///
    /// -   `tex_size`: side length of square texture atlases
    fn new(tex_size: i32) -> Self {
        Atlases {
            tex_size,
            atlases: vec![],
        }
    }

    fn allocate_space(
        &mut self,
        size: (i32, i32),
    ) -> Result<(u32, guillotiere::Allocation, (i32, i32)), AllocError> {
        let mut tex_size = (self.tex_size, self.tex_size);
        let size2d = size.into();
        let mut atlas = 0;
        while atlas < self.atlases.len() {
            if let Some(alloc) = self.atlases[atlas].alloc.allocate(size2d) {
                let tex_size = self.atlases[atlas].alloc.size().to_tuple();
                return Ok((atlas.cast(), alloc, tex_size));
            }
            atlas += 1;
        }

        if size.0 > self.tex_size || size.1 > self.tex_size {
            // Too large to fit in a regular atlas: use a special allocation
            if size.0 > MAX_TEX_SIZE || size.1 > MAX_TEX_SIZE {
                return Err(AllocError);
            }
            tex_size = size;
        }

        self.atlases.push(Atlas::new(tex_size));
        match self.atlases.last_mut().unwrap().alloc.allocate(size2d) {
            Some(alloc) => Ok((atlas.cast(), alloc, tex_size)),
            None => unreachable!(),
        }
    }

    fn upload_rgba8(&mut self, image: &Image, data: &[u8])
    where
        F: Format<C = u32>,
    {
        let atlas: usize = image.alloc.atlas.cast();
        let Some(atlas) = self.atlases.get_mut(atlas) else {
            log::warn!(
                "upload_rgba8: unknown atlas {} of {}",
                image.alloc.atlas,
                self.atlases.len()
            );
            return;
        };

        let (tx, ty): (usize, usize) = image.alloc.origin.cast();
        let (w, h): (usize, usize) = image.size.cast();
        let tex_size = atlas.alloc.size();
        let (tw, th): (usize, usize) = (tex_size.width.cast(), tex_size.height.cast());
        assert_eq!(atlas.tex.len(), tw * th);
        if tx + w > tw || ty + h > th {
            log::error!(
                "upload_rgba8: image of size {w}x{h} with origin {tx},{ty} not within bounds of texture {tw}x{th}"
            );
            return;
        }

        if data.len() != 4 * w * h {
            log::error!(
                "upload_rgba8: bad data length (received {} bytes for image of size {w}x{h})",
                data.len()
            );
            return;
        }

        for (i, row) in data.chunks_exact(4 * w).enumerate() {
            let ti = tx + (ty + i) * tw;
            let dst = &mut atlas.tex[ti..ti + w];
            assert!(row.len() == 4 * dst.len());
            for (out, chunk) in dst.iter_mut().zip(row.chunks_exact(4)) {
                // We convert color from input RGBA (LE) to ARGB (BE) with pre-multiplied alpha
                let c = u32::from_le_bytes(chunk.try_into().unwrap());
                let a = c >> 24 & 0xFF;
                let b = a * (c >> 16 & 0xFF) / 255;
                let g = a * (c >> 8 & 0xFF) / 255;
                let r = a * (c & 0xFF) / 255;
                *out = a << 24 | r << 16 | g << 8 | b;
            }
        }
    }
}

impl<F: Format> Allocator for Atlases<F> {
    /// Allocate space within a texture atlas
    ///
    /// Fails if `size` is zero in any dimension.
    fn allocate(&mut self, size: (u32, u32)) -> Result<Allocation, AllocError> {
        if size.0 == 0 || size.1 == 0 {
            return Err(AllocError);
        }

        let (atlas, alloc, tex_size) = self.allocate_space((size.0.cast(), size.1.cast()))?;

        let origin = (alloc.rectangle.min.x.cast(), alloc.rectangle.min.y.cast());

        let tex_size = Vec2::conv(Size::from(tex_size));
        let a = to_vec2(alloc.rectangle.min);
        let b = to_vec2(alloc.rectangle.max);
        debug_assert!(Vec2::ZERO <= a && a <= b && b <= tex_size);
        let tex_quad = Quad { a, b };

        Ok(Allocation {
            atlas,
            alloc: alloc.id.serialize(),
            origin,
            tex_quad,
        })
    }

    /// Free an allocation
    fn deallocate(&mut self, atlas: u32, alloc: u32) {
        let index = usize::conv(atlas);
        if let Some(data) = self.atlases.get_mut(index) {
            data.alloc.deallocate(AllocId::deserialize(alloc));
        }

        // Do not remove empty atlases since we use the index as a key.
        // TODO(opt): free unused memory at some point. Maybe also reset/resize
        // special-sized allocations.
    }
}

#[autoimpl(Default)]
#[derive(Clone, Debug)]
struct AtlasPassData<I> {
    instances: Vec<I>,
}

#[autoimpl(Default)]
#[derive(Clone, Debug)]
struct PassData<I> {
    atlases: Vec<AtlasPassData<I>>,
}

/// Per-window atlas state (pending draws lists)
#[autoimpl(Default)]
#[derive(Debug)]
struct AtlasWindow<I> {
    passes: Vec<PassData<I>>,
}

impl<I: Format> AtlasWindow<I> {
    /// Add a rectangle to the buffer
    fn rect(&mut self, pass: PassId, atlas: u32, instance: I) {
        let pass = pass.pass();
        if self.passes.len() <= pass {
            // We only need one more, but no harm in adding extra
            self.passes.resize_with(pass + 8, Default::default);
        }
        let pass = &mut self.passes[pass];

        let atlas = usize::conv(atlas);
        if pass.atlases.len() <= atlas {
            // Warning: length must not excced number of atlases
            pass.atlases.resize_with(atlas + 1, Default::default);
        }

        pass.atlases[atlas].instances.push(instance);
    }
}

impl AtlasWindow<InstanceRgba> {
    /// Enqueue render commands
    fn render(
        &mut self,
        atlases: &Atlases<InstanceRgba>,
        pass: usize,
        buffer: &mut [u32],
        size: (usize, usize),
    ) {
        if let Some(pass) = self.passes.get_mut(pass) {
            for (atlas, data) in pass.atlases.iter_mut().enumerate() {
                let Some(atlas) = atlases.atlases.get(atlas) else {
                    continue;
                };
                let tex = &atlas.tex;
                let tex_size = atlas.alloc.size();
                let tex_size = (tex_size.width.cast(), tex_size.height.cast());

                for inst in data.instances.drain(..) {
                    inst.render(tex, tex_size, buffer, size);
                }
            }
        }
    }
}

#[derive(Debug)]
struct Image {
    size: (u32, u32),
    alloc: Allocation,
}

/// Screen and texture coordinates
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct InstanceRgba {
    a: Vec2,
    b: Vec2,
    ta: Vec2,
    tb: Vec2,
}

trait Format {
    /// Color representation type
    type C: Clone + Default;

    fn render(
        &self,
        tex: &[Self::C],
        tex_size: (usize, usize),
        buffer: &mut [u32],
        buf_size: (usize, usize),
    );
}

impl Format for InstanceRgba {
    type C = u32;

    fn render(
        &self,
        tex: &[Self::C],
        tex_size: (usize, usize),
        buffer: &mut [u32],
        buf_size: (usize, usize),
    ) {
        let x0: usize = self.a.0.cast_nearest();
        let x1: usize = self.b.0.cast_nearest();
        let x1 = x1.min(buf_size.0);
        let xdi = 1.0 / (self.b.0 - self.a.0);
        let txd = self.tb.0 - self.ta.0;

        let y0: usize = self.a.1.cast_nearest();
        let y1: usize = self.b.1.cast_nearest();
        let y1 = y1.min(buf_size.1);
        let ydi = 1.0 / (self.b.1 - self.a.1);
        let tyd = self.tb.1 - self.ta.1;

        for y in y0..y1 {
            let ly = (f32::conv(y) - self.a.1) * ydi;
            let ty: usize = (self.ta.1 + tyd * ly).cast_nearest();

            for x in x0..x1 {
                let lx = (f32::conv(x) - self.a.0) * xdi;
                let tx: usize = (self.ta.0 + txd * lx).cast_nearest();

                let tc = tex[ty * tex_size.0 + tx];
                let a = tc >> 24;
                let ba = 255 - a;

                let index = y * buf_size.0 + x;
                let bc = buffer[index];
                let br = ba * (bc >> 16 & 0xFF) / 255;
                let bg = ba * (bc >> 8 & 0xFF) / 255;
                let bb = ba * (bc & 0xFF) / 255;

                let ab = br << 16 | bg << 8 | bb;
                buffer[index] = (tc & 0x00FFFFFF) + ab;
            }
        }
    }
}

/// Image loader and storage
pub struct Shared {
    text: kas::text::raster::State,
    atlas_rgba: Atlases<InstanceRgba>,
    last_image_n: u32,
    images: HashMap<ImageId, Image>,
}

impl Shared {
    /// Construct
    pub fn new() -> Self {
        Shared {
            text: Default::default(),
            atlas_rgba: Atlases::new(2048),
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

    /// Upload an image
    pub fn upload(&mut self, id: ImageId, data: &[u8], format: ImageFormat) {
        match format {
            ImageFormat::Rgba8 => (),
        }

        if let Some(image) = self.images.get_mut(&id) {
            self.atlas_rgba.upload_rgba8(&image, data);
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

    /// Get atlas and texture coordinates for an image
    #[inline]
    pub fn get_im_atlas_coords(&self, id: ImageId) -> Option<(u32, Quad)> {
        self.images
            .get(&id)
            .map(|im| (im.alloc.atlas, im.alloc.tex_quad))
    }
}

#[derive(Debug, Default)]
pub struct Window {
    atlas_rgba: AtlasWindow<InstanceRgba>,
}

impl Window {
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

    /// Enqueue render commands
    pub fn render(
        &mut self,
        shared: &Shared,
        pass: usize,
        buffer: &mut [u32],
        size: (usize, usize),
    ) {
        self.atlas_rgba
            .render(&shared.atlas_rgba, pass, buffer, size);
    }
}
