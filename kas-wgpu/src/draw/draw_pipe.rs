// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing API for `kas_wgpu`
//!
//! TODO: move traits up to kas?

use std::any::Any;
use std::borrow::Cow;
use std::f32::consts::FRAC_PI_2;

use wgpu_glyph::{GlyphBrush, GlyphBrushBuilder, GlyphCruncher, VariedSection};

use super::{Colour, Draw, Quad, Vec2};
use kas::geom::{Coord, Rect, Size};
use kas::theme;

use super::round_pipe::RoundPipe;
use super::square_pipe::SquarePipe;
use crate::shared::SharedState;

/// Style of drawing
pub enum ShadeStyle {
    /// Square corners, shading according to the given normals
    ///
    /// Normal has two components, `(outer, inner)`, interpreted as the
    /// horizontal component of the direction vector outwards from the drawn
    /// feature. Both values are constrained to the closed range `[-1, 1]`.
    Square(Vec2),
    /// Round corners, shading according to the given normals
    ///
    /// Normal has two components, `(outer, inner)`, interpreted as the
    /// horizontal component of the direction vector outwards from the drawn
    /// feature. Both values are constrained to the closed range `[-1, 1]`.
    Round(Vec2),
}

/// Abstraction over drawing commands
///
/// Implementations may support drawing each feature with multiple styles, but
/// do not guarantee an exact match in each case.
///
/// Certain bounds on input are expected in each case. In case these are not met
/// the implementation may tweak parameters to ensure valid drawing. In the case
/// that the outer region does not have positive size or has reversed
/// coordinates, drawing may not occur at all.
pub trait DrawShaded: Draw {
    /// Add a rounded shaded frame to the draw buffer.
    fn draw_frame(
        &mut self,
        region: Self::Region,
        outer: Quad,
        inner: Quad,
        style: ShadeStyle,
        col: Colour,
    );
}

/// Abstraction over text rendering
///
/// TODO: this API is heavily dependent on `glyph_brush`. Eventually we want our
/// own API, encapsulating translation functionality and with more default
/// values (e.g. scale). When we get there, we should be able to move
/// `SampleTheme` to `kas`.
pub trait DrawText {
    /// Queues a text section/layout.
    fn draw_text<'a, S>(&mut self, section: S)
    where
        S: Into<Cow<'a, VariedSection<'a>>>;

    /// Returns a bounding box for the section glyphs calculated using each glyph's
    /// vertical & horizontal metrics.
    ///
    /// If the section is empty or would result in no drawn glyphs will return `None`.
    ///
    /// Invisible glyphs, like spaces, are discarded during layout so trailing ones will
    /// not affect the bounds.
    ///
    /// The bounds will always lay within the specified layout bounds, ie that returned
    /// by the layout's `bounds_rect` function.
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    fn glyph_bounds<'a, S>(&mut self, section: S) -> Option<(Vec2, Vec2)>
    where
        S: Into<Cow<'a, VariedSection<'a>>>;
}

/// Manager of draw pipes and implementor of [`Draw`]
pub struct DrawPipe {
    clip_regions: Vec<Rect>,
    round_pipe: RoundPipe,
    square_pipe: SquarePipe,
    glyph_brush: GlyphBrush<'static, ()>,
}

impl DrawPipe {
    /// Construct
    // TODO: do we want to share state across windows? With glyph_brush this is
    // not trivial but with our "pipes" it shouldn't be difficult.
    pub fn new<T: theme::Theme<Self>>(
        shared: &mut SharedState<T>,
        tex_format: wgpu::TextureFormat,
        size: Size,
    ) -> Self {
        let dir = shared.theme.light_direction();
        assert!(dir.0 >= 0.0);
        assert!(dir.0 < FRAC_PI_2);
        let a = (dir.0.sin(), dir.0.cos());
        // We normalise intensity:
        let f = a.0 / a.1;
        let norm = [dir.1.sin() * f, -dir.1.cos() * f, 1.0];

        let glyph_brush = GlyphBrushBuilder::using_fonts(shared.theme.get_fonts())
            .build(&mut shared.device, tex_format);

        let region = Rect {
            pos: Coord::ZERO,
            size,
        };
        DrawPipe {
            clip_regions: vec![region],
            square_pipe: SquarePipe::new(shared, size, norm),
            round_pipe: RoundPipe::new(shared, size, norm),
            glyph_brush,
        }
    }

    /// Process window resize
    pub fn resize(&mut self, device: &wgpu::Device, size: Size) -> wgpu::CommandBuffer {
        self.clip_regions[0].size = size;
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
        self.square_pipe.resize(device, &mut encoder, size);
        self.round_pipe.resize(device, &mut encoder, size);
        encoder.finish()
    }

    /// Render batched draw instructions via `rpass`
    pub fn render(
        &mut self,
        device: &mut wgpu::Device,
        frame_view: &wgpu::TextureView,
        clear_color: wgpu::Color,
    ) -> wgpu::CommandBuffer {
        let desc = wgpu::CommandEncoderDescriptor { todo: 0 };
        let mut encoder = device.create_command_encoder(&desc);
        let mut load_op = wgpu::LoadOp::Clear;

        // We use a separate render pass for each clipped region.
        for (pass, region) in self.clip_regions.iter().enumerate() {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: frame_view,
                    resolve_target: None,
                    load_op: load_op,
                    store_op: wgpu::StoreOp::Store,
                    clear_color,
                }],
                depth_stencil_attachment: None,
            });
            rpass.set_scissor_rect(
                region.pos.0 as u32,
                region.pos.1 as u32,
                region.size.0,
                region.size.1,
            );

            self.square_pipe.render(device, pass, &mut rpass);
            self.round_pipe.render(device, pass, &mut rpass);
            drop(rpass);

            load_op = wgpu::LoadOp::Load;
        }

        // Fonts use their own render pass(es).
        let size = self.clip_regions[0].size;
        self.glyph_brush
            .draw_queued(device, &mut encoder, frame_view, size.0, size.1)
            .expect("glyph_brush.draw_queued");

        // Keep only first clip region (which is the entire window)
        self.clip_regions.truncate(1);

        encoder.finish()
    }
}

impl Draw for DrawPipe {
    type Region = usize;

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn add_clip_region(&mut self, region: Rect) -> usize {
        let pass = self.clip_regions.len();
        self.clip_regions.push(region);
        pass
    }

    #[inline]
    fn rect(&mut self, region: Self::Region, rect: Rect, col: Colour) {
        self.square_pipe.rect(region, rect, col);
    }

    #[inline]
    fn frame(&mut self, region: Self::Region, outer: Rect, inner: Rect, col: Colour) {
        self.square_pipe.frame(region, outer, inner, col);
    }
}

impl DrawShaded for DrawPipe {
    #[inline]
    fn draw_frame(
        &mut self,
        pass: usize,
        outer: Quad,
        inner: Quad,
        style: ShadeStyle,
        col: Colour,
    ) {
        match style {
            ShadeStyle::Square(norm) => self.square_pipe.add_frame(pass, outer, inner, norm, col),
            ShadeStyle::Round(norm) => self.round_pipe.add_frame(pass, outer, inner, norm, col),
        }
    }
}

impl DrawText for DrawPipe {
    #[inline]
    fn draw_text<'a, S>(&mut self, section: S)
    where
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        self.glyph_brush.queue(section)
    }

    #[inline]
    fn glyph_bounds<'a, S>(&mut self, section: S) -> Option<(Vec2, Vec2)>
    where
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        self.glyph_brush
            .glyph_bounds(section)
            .map(|rect| (Vec2(rect.min.x, rect.min.y), Vec2(rect.max.x, rect.max.y)))
    }
}
