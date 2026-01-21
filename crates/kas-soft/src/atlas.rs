// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Images pipeline

use guillotiere::{AllocId, AtlasAllocator};
use kas::config::SubpixelMode;
use std::collections::HashMap;

use kas::autoimpl;
use kas::cast::traits::*;
use kas::draw::{
    AllocError, Allocation, Allocator, ImageFormat, ImageId, PassId, UploadError, color,
};
use kas::geom::{Coord, Offset, Quad, Rect, Size, Vec2};
use kas::text::raster::{RenderQueue, Sprite, SpriteAllocator, SpriteType, UnpreparedSprite};

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

    fn upload(
        &mut self,
        atlas: u32,
        origin: (u32, u32),
        size: (u32, u32),
        data: &[u8],
    ) -> Result<(), UploadError> {
        let Some(atlas) = self.atlases.get_mut(usize::conv(atlas)) else {
            return Err(UploadError::AtlasIndex(atlas));
        };

        let (tx, ty): (usize, usize) = origin.cast();
        let (w, h): (usize, usize) = size.cast();
        let tex_size = atlas.alloc.size();
        let (tw, th): (usize, usize) = (tex_size.width.cast(), tex_size.height.cast());
        assert_eq!(atlas.tex.len(), tw * th);
        if tx + w > tw || ty + h > th {
            // log::error!(
            //     "upload: image of size {w}x{h} with origin {tx},{ty} not within bounds of texture {tw}x{th}"
            // );
            return Err(UploadError::TextureCoordinates);
        }

        let bytes = size_of::<F::C>();
        if data.len() != bytes * w * h {
            // log::error!(
            //     "upload: bad data length (received {} bytes for image of size {w}x{h})",
            //     data.len()
            // );
            return Err(UploadError::DataLen(data.len().cast()));
        }

        for (i, row) in data.chunks_exact(bytes * w).enumerate() {
            let ti = tx + (ty + i) * tw;
            let w = w + F::EXTRA_WIDTH;
            F::copy_texture_slice(row, &mut atlas.tex[ti..ti + w]);
        }

        Ok(())
    }
}

impl<F: Format> Allocator for Atlases<F> {
    /// Allocate space within a texture atlas
    ///
    /// Fails if `size` is zero in any dimension.
    fn allocate(&mut self, size: Size) -> Result<Allocation, AllocError> {
        if size.0 == 0 || size.1 == 0 {
            return Err(AllocError);
        }

        let (atlas, alloc, tex_size) = self.allocate_space((size.0, size.1))?;

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

impl<I: Format> AtlasWindow<I> {
    /// Enqueue render commands
    fn render(
        &mut self,
        atlases: &Atlases<I>,
        pass: usize,
        buffer: &mut [u32],
        size: (usize, usize),
        clip_rect: Rect,
        offset: Offset,
    ) {
        if let Some(pass) = self.passes.get_mut(pass) {
            for (atlas, data) in pass.atlases.iter_mut().enumerate() {
                let Some(atlas) = atlases.atlases.get(atlas) else {
                    data.instances.clear();
                    continue;
                };
                let tex = &atlas.tex;
                let tex_size = atlas.alloc.size();
                let tex_size = (tex_size.width.cast(), tex_size.height.cast());

                for inst in data.instances.drain(..) {
                    inst.render(tex, tex_size, buffer, size, clip_rect, offset);
                }
            }
        }
    }
}

#[derive(Debug)]
struct Image {
    size: Size,
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

/// Screen and texture coordinates (8-bit coverage mask)
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct InstanceMask {
    a: Vec2,
    b: Vec2,
    ta: Vec2,
    tb: Vec2,
    col: u32, // 0RGB BE format
}

/// Screen and texture coordinates (32-bit coverage mask)
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
#[autoimpl(Deref, DerefMut using self.0)]
struct InstanceRgbaMask(InstanceMask);

trait Format {
    const EXTRA_WIDTH: usize = 0;

    /// Color representation type
    type C: Clone + Default;

    fn copy_texture_slice(src: &[u8], dst: &mut [Self::C]);

    fn render(
        &self,
        tex: &[Self::C],
        tex_size: (usize, usize),
        buffer: &mut [u32],
        buf_size: (usize, usize),
        clip_rect: Rect,
        offset: Offset,
    );
}

impl Format for InstanceRgba {
    type C = u32;

    fn copy_texture_slice(src: &[u8], dst: &mut [Self::C]) {
        assert!(src.len() == 4 * dst.len());
        for (out, chunk) in dst.iter_mut().zip(src.chunks_exact(4)) {
            // We convert color from input RGBA (LE) to ARGB (BE) with pre-multiplied alpha
            let c = u32::from_le_bytes(chunk.try_into().unwrap());
            let a = c >> 24 & 0xFF;
            let b = a * (c >> 16 & 0xFF) / 255;
            let g = a * (c >> 8 & 0xFF) / 255;
            let r = a * (c & 0xFF) / 255;
            *out = a << 24 | r << 16 | g << 8 | b;
        }
    }

    fn render(
        &self,
        tex: &[Self::C],
        tex_size: (usize, usize),
        buffer: &mut [u32],
        buf_size: (usize, usize),
        clip_rect: Rect,
        offset: Offset,
    ) {
        let (clip_p, clip_q) = (clip_rect.pos, clip_rect.pos2());
        let p = (Coord::conv_nearest(self.a) - offset).clamp(clip_p, clip_q);
        let q = (Coord::conv_nearest(self.b) - offset).clamp(clip_p, clip_q);

        let xdi = 1.0 / (self.b.0 - self.a.0);
        let txd = self.tb.0 - self.ta.0;
        let ydi = 1.0 / (self.b.1 - self.a.1);
        let tyd = self.tb.1 - self.ta.1;

        for y in p.1..q.1 {
            let ly = (f32::conv(y + offset.1) - self.a.1) * ydi;
            let ty: usize = (self.ta.1 + tyd * ly).cast_nearest();

            for x in p.0..q.0 {
                let lx = (f32::conv(x + offset.0) - self.a.0) * xdi;
                let tx: usize = (self.ta.0 + txd * lx).cast_nearest();

                let tc = tex[ty * tex_size.0 + tx];
                let a = tc >> 24;
                let ba = 255 - a;

                let index = usize::conv(y) * buf_size.0 + usize::conv(x);
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

impl Format for InstanceMask {
    type C = u8;

    fn copy_texture_slice(src: &[u8], dst: &mut [Self::C]) {
        dst.copy_from_slice(src);
    }

    fn render(
        &self,
        tex: &[Self::C],
        tex_size: (usize, usize),
        buffer: &mut [u32],
        buf_size: (usize, usize),
        clip_rect: Rect,
        offset: Offset,
    ) {
        let (clip_p, clip_q) = (clip_rect.pos, clip_rect.pos2());
        let p = (Coord::conv_nearest(self.a) - offset).clamp(clip_p, clip_q);
        let q = (Coord::conv_nearest(self.b) - offset).clamp(clip_p, clip_q);

        let xdi = 1.0 / (self.b.0 - self.a.0);
        let txd = self.tb.0 - self.ta.0;
        let ydi = 1.0 / (self.b.1 - self.a.1);
        let tyd = self.tb.1 - self.ta.1;

        for y in p.1..q.1 {
            let ly = (f32::conv(y + offset.1) - self.a.1) * ydi;
            let ty: usize = (self.ta.1 + tyd * ly).cast_nearest();

            for x in p.0..q.0 {
                let lx = (f32::conv(x + offset.0) - self.a.0) * xdi;
                let tx: usize = (self.ta.0 + txd * lx).cast_nearest();

                let a = tex[ty * tex_size.0 + tx] as u32;
                let ba = 255 - a;

                let r = a * (self.col >> 16 & 0xFF) / 255;
                let g = a * (self.col >> 8 & 0xFF) / 255;
                let b = a * (self.col & 0xFF) / 255;

                let index = usize::conv(y) * buf_size.0 + usize::conv(x);
                let bc = buffer[index];
                let br = ba * (bc >> 16 & 0xFF) / 255;
                let bg = ba * (bc >> 8 & 0xFF) / 255;
                let bb = ba * (bc & 0xFF) / 255;

                let c = r << 16 | g << 8 | b;
                let ab = br << 16 | bg << 8 | bb;
                buffer[index] = c + ab;
            }
        }
    }
}

// TODO: replace with portable SIMD type when stable
// Components are u16 to allow widening multiplication
#[derive(Clone, Copy)]
struct U16x4([u16; 4]);

impl std::ops::Mul for U16x4 {
    type Output = u16;

    fn mul(self, rhs: Self) -> u16 {
        self.0[0] * rhs.0[0] + self.0[1] * rhs.0[1] + self.0[2] * rhs.0[2] + self.0[3] * rhs.0[3]
    }
}

impl From<[u8; 4]> for U16x4 {
    fn from([r, g, b, a]: [u8; 4]) -> Self {
        U16x4([r as u16, g as u16, b as u16, a as u16])
    }
}

impl Format for InstanceRgbaMask {
    const EXTRA_WIDTH: usize = 2;
    type C = u32;

    fn copy_texture_slice(src: &[u8], dst: &mut [Self::C]) {
        // dst is 2 longer than src row to allow room for filter
        assert_eq!(src.len() + 8, 4 * dst.len());

        let (chunks, rem) = src.as_chunks();
        debug_assert!(rem.is_empty());
        if chunks.is_empty() {
            return;
        }

        // TODO: filter is cut off at the edges
        // Weights are copied from FreeType2's default weights, per RGBA sub-pixels
        // https://freetype.org/freetype2/docs/reference/ft2-lcd_rendering.html
        const A: u16 = 0x08;
        const B: u16 = 0x4D;
        const C: u16 = 0x56;
        const SUM: u16 = 0x100;
        assert_eq!(2 * A + 2 * B + C, SUM);

        // NOTE: weights assume RGB sub-pixel layout
        // Weight tables are designed to be SIMD-friendly
        const R1: U16x4 = U16x4([0, A, B, 0]);
        const R2: U16x4 = U16x4([C, B, A, 0]);
        const R3: U16x4 = U16x4([0, 0, 0, 0]);
        const G1: U16x4 = U16x4([0, 0, A, 0]);
        const G2: U16x4 = U16x4([B, C, B, 0]);
        const G3: U16x4 = U16x4([A, 0, 0, 0]);
        const B1: U16x4 = U16x4([0, 0, 0, 0]);
        const B2: U16x4 = U16x4([A, B, C, 0]);
        const B3: U16x4 = U16x4([B, A, 0, 0]);

        let mut x = U16x4([0u16; 4]);
        let mut y: U16x4 = chunks[0].into();

        let r = (R3 * y) / SUM;
        let g = (G3 * y) / SUM;
        let b = (B3 * y) / SUM;
        dst[0] = (r as u32) << 16 | (g as u32) << 8 | (b as u32);

        for i in 1..chunks.len() {
            let z: U16x4 = chunks[i].into();
            let r = (R1 * x + R2 * y + R3 * z) / SUM;
            let g = (G1 * x + G2 * y + G3 * z) / SUM;
            let b = (B1 * x + B2 * y + B3 * z) / SUM;

            // Output format is ARGB with A=0:
            dst[i] = (r as u32) << 16 | (g as u32) << 8 | (b as u32);

            x = y;
            y = z;
        }

        let r = (R1 * x + R2 * y) / SUM;
        let g = (G1 * x + G2 * y) / SUM;
        let b = (B1 * x + B2 * y) / SUM;
        dst[dst.len() - 2] = (r as u32) << 16 | (g as u32) << 8 | (b as u32);

        let r = (R1 * y) / SUM;
        let g = (G1 * y) / SUM;
        let b = (B1 * y) / SUM;
        dst[dst.len() - 1] = (r as u32) << 16 | (g as u32) << 8 | (b as u32);
    }

    fn render(
        &self,
        tex: &[Self::C],
        tex_size: (usize, usize),
        buffer: &mut [u32],
        buf_size: (usize, usize),
        clip_rect: Rect,
        offset: Offset,
    ) {
        let (clip_p, clip_q) = (clip_rect.pos, clip_rect.pos2());
        let p = (Coord::conv_nearest(self.a) - offset).clamp(clip_p, clip_q);
        let q = (Coord::conv_nearest(self.b) - offset).clamp(clip_p, clip_q);

        let xdi = 1.0 / (self.b.0 - self.a.0);
        let txd = self.tb.0 - self.ta.0;
        let ydi = 1.0 / (self.b.1 - self.a.1);
        let tyd = self.tb.1 - self.ta.1;

        for y in p.1..q.1 {
            let ly = (f32::conv(y + offset.1) - self.a.1) * ydi;
            let ty: usize = (self.ta.1 + tyd * ly).cast_nearest();

            for x in p.0..q.0 {
                let lx = (f32::conv(x + offset.0) - self.a.0) * xdi;
                let tx: usize = (self.ta.0 + txd * lx).cast_nearest();

                let m = tex[ty * tex_size.0 + tx];
                let mr = m >> 16 & 0xFF;
                let mg = m >> 8 & 0xFF;
                let mb = m & 0xFF;

                let r = mr * (self.col >> 16 & 0xFF) / 255;
                let g = mg * (self.col >> 8 & 0xFF) / 255;
                let b = mb * (self.col & 0xFF) / 255;

                let index = usize::conv(y) * buf_size.0 + usize::conv(x);
                let bc = buffer[index];

                let br = (255 - mr) * (bc >> 16 & 0xFF) / 255;
                let bg = (255 - mg) * (bc >> 8 & 0xFF) / 255;
                let bb = (255 - mb) * (bc & 0xFF) / 255;

                let c = r << 16 | g << 8 | b;
                let ab = br << 16 | bg << 8 | bb;
                buffer[index] = c + ab;
            }
        }
    }
}

/// Image loader and storage
pub struct Shared {
    atlas_rgba: Atlases<InstanceRgba>,
    atlas_mask: Atlases<InstanceMask>,
    atlas_rgba_mask: Atlases<InstanceRgbaMask>,
    last_image_n: u32,
    images: HashMap<ImageId, Image>,
}

impl Shared {
    /// Construct
    pub fn new() -> Self {
        Shared {
            atlas_rgba: Atlases::new(2048),
            atlas_mask: Atlases::new(512),
            atlas_rgba_mask: Atlases::new(512),
            last_image_n: 0,
            images: Default::default(),
        }
    }

    /// Configure
    pub fn set_raster_config(&mut self, config: &kas::config::RasterConfig) {
        match config.subpixel_mode {
            SubpixelMode::None => (),
            SubpixelMode::HorizontalRGB => {
                // <InstanceRgbaMask as Format>::copy_texture_slice assumes this mode
            } // NOTE: additional modes will require changes to the subpixel filter weights
        }
    }

    fn next_image_id(&mut self) -> ImageId {
        let n = self.last_image_n.wrapping_add(1);
        self.last_image_n = n;
        ImageId::try_new(n).expect("exhausted image IDs")
    }

    /// Allocate an image
    pub fn alloc(&mut self, format: ImageFormat, size: Size) -> Result<ImageId, AllocError> {
        Ok(match format {
            ImageFormat::Rgba8 => {
                let id = self.next_image_id();
                let alloc = self.atlas_rgba.allocate(size)?;
                let image = Image { size, alloc };
                self.images.insert(id, image);
                id
            }
        })
    }

    /// Upload an image
    pub fn upload(&mut self, id: ImageId, data: &[u8]) -> Result<(), UploadError> {
        let Some(image) = self.images.get_mut(&id) else {
            return Err(UploadError::ImageId(id));
        };

        let atlas = image.alloc.atlas.cast();
        let origin = image.alloc.origin;
        self.atlas_rgba
            .upload(atlas, origin, image.size.cast(), data)
    }

    /// Free an image allocation
    pub fn free(&mut self, id: ImageId) {
        if let Some(im) = self.images.remove(&id) {
            self.atlas_rgba.deallocate(im.alloc.atlas, im.alloc.alloc);
        }
    }

    /// Query image size
    pub fn image_size(&self, id: ImageId) -> Option<Size> {
        self.images.get(&id).map(|im| im.size)
    }

    /// Get atlas and texture coordinates for an image
    #[inline]
    pub fn get_im_atlas_coords(&self, id: ImageId) -> Option<(u32, Quad)> {
        self.images
            .get(&id)
            .map(|im| (im.alloc.atlas, im.alloc.tex_quad))
    }

    /// Write to textures
    pub fn prepare(&mut self, text: &mut kas::text::raster::State) {
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
            let result = match ty {
                SpriteType::Mask => self.atlas_mask.upload(atlas, origin, size, &data),
                SpriteType::RgbaMask => self.atlas_rgba_mask.upload(atlas, origin, size, &data),
                SpriteType::Bitmap => self.atlas_rgba.upload(atlas, origin, size, &data),
            };

            if let Err(err) = result {
                log::warn!("Sprite upload failed: {err}");
            }
        }
    }
}

impl SpriteAllocator for Shared {
    fn query_subpixel_rendering(&self) -> bool {
        true
    }

    fn alloc_mask(&mut self, size: (u32, u32)) -> Result<Allocation, AllocError> {
        self.atlas_mask.allocate(size.cast())
    }

    fn alloc_rgba_mask(&mut self, mut size: (u32, u32)) -> Result<Allocation, AllocError> {
        size.0 += 2; // allow for correct filtering
        self.atlas_rgba_mask.allocate(size.cast())
    }

    fn alloc_rgba(&mut self, size: (u32, u32)) -> Result<Allocation, AllocError> {
        self.atlas_rgba.allocate(size.cast())
    }
}

#[derive(Debug, Default)]
pub struct Window {
    atlas_rgba: AtlasWindow<InstanceRgba>,
    atlas_mask: AtlasWindow<InstanceMask>,
    atlas_rgba_mask: AtlasWindow<InstanceRgbaMask>,
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

    pub fn render(
        &mut self,
        shared: &Shared,
        pass: usize,
        buffer: &mut [u32],
        size: (usize, usize),
        clip_rect: Rect,
        offset: Offset,
    ) {
        self.atlas_rgba
            .render(&shared.atlas_rgba, pass, buffer, size, clip_rect, offset);
        self.atlas_mask
            .render(&shared.atlas_mask, pass, buffer, size, clip_rect, offset);
        self.atlas_rgba_mask.render(
            &shared.atlas_rgba_mask,
            pass,
            buffer,
            size,
            clip_rect,
            offset,
        );
    }
}

impl RenderQueue for Window {
    fn push_sprite(
        &mut self,
        pass: PassId,
        glyph_pos: Vec2,
        rect: Quad,
        col: color::Rgba,
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

        if ty == SpriteType::RgbaMask {
            // Correct for extra room allocated for filter, assuming pixel-perfect placement
            a.0 -= 1.0;
            b.0 += 1.0;
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

        let col: color::Rgba8Srgb = col.into();
        let col = u32::from_le_bytes(col.0).swap_bytes() >> 8;

        match ty {
            SpriteType::Mask => {
                let instance = InstanceMask { a, b, ta, tb, col };
                self.atlas_mask.rect(pass, sprite.atlas, instance);
            }
            SpriteType::RgbaMask => {
                let instance = InstanceRgbaMask(InstanceMask { a, b, ta, tb, col });
                self.atlas_rgba_mask.rect(pass, sprite.atlas, instance);
            }
            SpriteType::Bitmap => {
                let instance = InstanceRgba { a, b, ta, tb };
                self.atlas_rgba.rect(pass, sprite.atlas, instance);
            }
        }
    }
}
