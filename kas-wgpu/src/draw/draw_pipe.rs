// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing API for `kas_wgpu`
//!
//! TODO: move traits up to kas?

use std::borrow::Cow;
use std::f32::consts::FRAC_PI_2;

use wgpu_glyph::{GlyphBrush, GlyphBrushBuilder, GlyphCruncher, VariedSection};

use kas::draw::{Colour, Draw, Quad, Style, Theme, Vec2};
use kas::geom::Size;

use super::round_pipe::RoundPipe;
use super::square_pipe::SquarePipe;

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
    size: Size,
    round_pipe: RoundPipe,
    square_pipe: SquarePipe,
    glyph_brush: GlyphBrush<'static, ()>,
}

impl DrawPipe {
    /// Construct
    pub fn new<D: Theme>(
        device: &mut wgpu::Device,
        tex_format: wgpu::TextureFormat,
        size: Size,
        theme: &D,
    ) -> Self {
        let dir = theme.light_direction();
        assert!(dir.0 >= 0.0);
        assert!(dir.0 < FRAC_PI_2);
        let a = (dir.0.sin(), dir.0.cos());
        // We normalise intensity:
        let f = a.0 / a.1;
        let norm = [dir.1.sin() * f, -dir.1.cos() * f, 1.0];

        let glyph_brush =
            GlyphBrushBuilder::using_fonts(theme.get_fonts()).build(device, tex_format);

        DrawPipe {
            size,
            square_pipe: SquarePipe::new(device, size, norm),
            round_pipe: RoundPipe::new(device, size, norm),
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
        clear_color: wgpu::Color,
    ) -> wgpu::CommandBuffer {
        let rpass_color_attachment = wgpu::RenderPassColorAttachmentDescriptor {
            attachment: frame_view,
            resolve_target: None,
            load_op: wgpu::LoadOp::Clear,
            store_op: wgpu::StoreOp::Store,
            clear_color,
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

impl Draw for DrawPipe {
    #[inline]
    fn draw_quad(&mut self, quad: Quad, style: Style, col: Colour) {
        // TODO: support styles
        let _ = style;
        self.square_pipe.add_quad(quad, col)
    }

    #[inline]
    fn draw_frame(&mut self, outer: Quad, inner: Quad, style: Style, col: Colour) {
        match style {
            Style::Flat => self
                .square_pipe
                .add_frame(outer, inner, Vec2::splat(0.0), col),
            Style::Square(norm) => self.square_pipe.add_frame(outer, inner, norm, col),
            Style::Round(norm) => self.round_pipe.add_frame(outer, inner, norm, col),
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
