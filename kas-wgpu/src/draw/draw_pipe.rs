// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing API for `kas_wgpu`
//!
//! TODO: move traits up to kas?

use std::borrow::Cow;

use wgpu_glyph::{Font, GlyphBrush, GlyphBrushBuilder, GlyphCruncher, VariedSection};

use kas::geom::Size;

use super::round_pipe::RoundPipe;
use super::square_pipe::SquarePipe;
use crate::colour::{self, Colour};
use crate::vertex::Vec2;

/// Abstraction over flat drawing commands
pub trait DrawFlat {
    /// Add a rectangle to the draw buffer defined by two corners
    /// `aa` and `bb` with solid colour `col`.
    fn draw_flat_quad(&mut self, aa: Vec2, bb: Vec2, col: Colour);
}

/// Abstraction over square drawing commands
pub trait DrawSquare {
    /// Add a frame to the buffer, defined by two outer corners, `aa` and `bb`,
    /// and two inner corners, `cc` and `dd`, with solid colour `col`.
    ///
    /// The frame is shaded according to its normal, and appears flat when
    /// `norm = (0.0, 0.0)`.
    ///
    /// The normal is calculated from the `x` component (for verticals) or `y`
    /// component (for horizontals); the other `x` / `y` component is set to
    /// zero while the `z` component is calculated such that `x² + y² + z² = 1`.
    ///
    /// The normal component itself is calculated via linear interpolation
    /// between `outer` and `inner`, where parameter `norm = (outer, inner)`,
    /// with both parameters pointing towards the centre of the frame (thus
    /// positive values make the frame appear sunken).
    ///
    /// Component-wise bounds: `aa < cc < dd < bb`; `-1 < norm < 1`.
    fn draw_square_frame(
        &mut self,
        aa: Vec2,
        bb: Vec2,
        cc: Vec2,
        dd: Vec2,
        norm: (f32, f32),
        col: Colour,
    );
}

/// Abstraction over rounded drawing commands
pub trait DrawRound {
    /// Add a frame to the buffer, defined by two outer corners, `aa` and `bb`,
    /// and two inner corners, `cc` and `dd` with colour `col`.
    // TODO: allow control of normals
    fn draw_round_frame(&mut self, aa: Vec2, bb: Vec2, cc: Vec2, dd: Vec2, col: Colour);
}

/// Abstraction over text rendering
pub trait DrawText {
    // TODO: should we have an API not dependent on glyph_brush?
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
    size: Size,
    round_pipe: RoundPipe,
    square_pipe: SquarePipe,
    glyph_brush: GlyphBrush<'static, ()>,
}

impl DrawPipe {
    /// Construct
    pub fn new(
        device: &mut wgpu::Device,
        tex_format: wgpu::TextureFormat,
        fonts: Vec<Font<'static>>,
        size: Size,
    ) -> Self {
        let glyph_brush = GlyphBrushBuilder::using_fonts(fonts).build(device, tex_format);

        DrawPipe {
            size,
            square_pipe: SquarePipe::new(device, size),
            round_pipe: RoundPipe::new(device, size),
            glyph_brush,
        }
    }

    /// Process window resize
    pub fn resize(&mut self, device: &wgpu::Device, size: Size) -> wgpu::CommandBuffer {
        self.size = size;
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
    ) -> wgpu::CommandBuffer {
        let rpass_color_attachment = wgpu::RenderPassColorAttachmentDescriptor {
            attachment: frame_view,
            resolve_target: None,
            load_op: wgpu::LoadOp::Clear,
            store_op: wgpu::StoreOp::Store,
            clear_color: colour::BACKGROUND.into(),
        };

        let desc = wgpu::CommandEncoderDescriptor { todo: 0 };
        let mut encoder = device.create_command_encoder(&desc);

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[rpass_color_attachment],
            depth_stencil_attachment: None,
        });

        self.square_pipe.render(device, &mut rpass);
        self.round_pipe.render(device, &mut rpass);
        drop(rpass);

        self.glyph_brush
            .draw_queued(device, &mut encoder, frame_view, self.size.0, self.size.1)
            .expect("glyph_brush.draw_queued");

        encoder.finish()
    }
}

impl DrawFlat for DrawPipe {
    fn draw_flat_quad(&mut self, aa: Vec2, bb: Vec2, col: Colour) {
        // TODO: is it more efficient to have a dedicated pipeline for this?
        self.square_pipe.add_quad(aa, bb, col)
    }
}

impl DrawSquare for DrawPipe {
    fn draw_square_frame(
        &mut self,
        aa: Vec2,
        bb: Vec2,
        cc: Vec2,
        dd: Vec2,
        norm: (f32, f32),
        col: Colour,
    ) {
        self.square_pipe.add_frame(aa, bb, cc, dd, norm, col)
    }
}

impl DrawRound for DrawPipe {
    fn draw_round_frame(&mut self, aa: Vec2, bb: Vec2, cc: Vec2, dd: Vec2, col: Colour) {
        self.round_pipe.add_frame(aa, bb, cc, dd, col)
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
