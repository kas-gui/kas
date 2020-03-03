// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing API
//!
//! This module includes abstractions over the drawing API and some associated
//! types.
//!
//! All draw operations may be batched; when drawn primitives overlap, the
//! results are only loosely defined. All opaque draw commands replace prior
//! contents, and generally executed in the order queued.
//! Any partially-transparent draw commands are executed after opaque commands.
//!
//! ### High-level interface
//!
//! High-level drawing primitives are provided by the [`DrawHandle`] trait, an
//! implementation of which is passed to [`kas::Layout::draw`]. A companion
//! trait, [`SizeHandle`], is passed to [`kas::Layout::size_rules`].
//!
//! This interface allows themes a degree of flexibility over both drawing and
//! sizing of elements and provides users with a very high-level interface to
//! common UI shapes. It is expected to be extended significantly.
//!
//! ### Medium-level interface
//!
//! The [`Draw`] trait and its extensions provide a simpler, limited, drawing
//! API, covering shapes such as lines, circles and rectangles, with
//! single-colour (flat) shading or (depending upon the implementation) some
//! other shading options.
//!
//! The [`Draw`] trait itself contains very little; extension traits
//! [`DrawRounded`], [`DrawShaded`] and [`DrawText`] provide some additionaol
//! routines. Toolkits must implement support for [`Draw`] while other
//! extensions are optional; toolkits may also provide their own extensions.
//!
//! ### Low-level interface
//!
//! There is no universal graphics API, hence none is provided by this crate.
//! Instead, toolkits may provide their own extensions allowing direct access
//! to the host graphics API, for example `kas-wgpu::draw::CustomPipe`.

mod colour;
mod handle;
mod text;

use std::any::Any;

use crate::geom::{Coord, Rect};

pub use colour::Colour;
pub use handle::{DrawHandle, SizeHandle, TextClass};
pub use text::{DrawText, DrawTextShared, Font, FontId, TextProperties};

/// Type returned by [`Draw::add_clip_region`].
///
/// Supports [`Default`], which may be used to target the root region.
#[derive(Copy, Clone, Default)]
pub struct Region(pub usize);

/// Bounds on type shared across [`Draw`] implementations
pub trait DrawShared {
    type Draw: Draw;
}

/// Base abstraction over drawing
///
/// All draw operations target some region identified by a handle of type
/// [`Region`]; this may be the whole window, some sub-region, or perhaps
/// something else such as a texture. In general the user doesn't need to know
/// what this is, but merely pass the given handle.
///
/// The primitives provided by this trait all draw solid areas, replacing prior
/// contents.
pub trait Draw: Any {
    /// Cast self to [`std::any::Any`] reference.
    ///
    /// A downcast on this value may be used to obtain a reference to a
    /// toolkit-specific API.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Add a clip region
    ///
    /// Clip regions are cleared each frame and so must be recreated on demand.
    fn add_clip_region(&mut self, region: Rect) -> Region;

    /// Draw a rectangle of uniform colour
    fn rect(&mut self, region: Region, rect: Rect, col: Colour);

    /// Draw a frame of uniform colour
    ///
    /// The frame is defined by the area inside `outer` and not inside `inner`.
    fn frame(&mut self, region: Region, outer: Rect, inner: Rect, col: Colour);
}

/// Drawing commands for rounded shapes
///
/// This trait is an extension over [`Draw`] providing rounded shapes.
///
/// The primitives provided by this trait are partially transparent.
/// If the implementation buffers draw commands, it should draw these
/// primitives after solid primitives.
pub trait DrawRounded: Draw {
    /// Draw a line with rounded ends and uniform colour
    ///
    /// This command draws a line segment between the points `p1` and `p2`.
    /// Pixels within the given `radius` of this segment are drawn, resulting
    /// in rounded ends and width `2 * radius`.
    ///
    /// Note that for rectangular, axis-aligned lines, [`Draw::rect`] should be
    /// preferred.
    fn rounded_line(&mut self, region: Region, p1: Coord, p2: Coord, radius: f32, col: Colour);

    /// Draw a circle or oval of uniform colour
    ///
    /// More generally, this shape is an axis-aligned oval which may be hollow.
    ///
    /// The `inner_radius` parameter gives the inner radius relative to the
    /// outer radius: a value of `0.0` will result in the whole shape being
    /// painted, while `1.0` will result in a zero-width line on the outer edge.
    fn circle(&mut self, region: Region, rect: Rect, inner_radius: f32, col: Colour);

    /// Draw a frame with rounded corners and uniform colour
    ///
    /// All drawing occurs within the `outer` rect and outside of the `inner`
    /// rect. Corners are circular (or more generally, ovular), centered on the
    /// inner corners.
    ///
    /// The `inner_radius` parameter gives the inner radius relative to the
    /// outer radius: a value of `0.0` will result in the whole shape being
    /// painted, while `1.0` will result in a zero-width line on the outer edge.
    /// When `inner_radius > 0`, the frame will be visually thinner than the
    /// allocated area.
    fn rounded_frame(
        &mut self,
        region: Region,
        outer: Rect,
        inner: Rect,
        inner_radius: f32,
        col: Colour,
    );
}

/// Drawing commands for shaded shapes
///
/// This trait is an extension over [`Draw`] providing solid shaded shapes.
///
/// Some drawing primitives (the "round" ones) are partially transparent.
/// If the implementation buffers draw commands, it should draw these
/// primitives after solid primitives.
///
/// These are parameterised via a pair of normals, `(inner, outer)`. These may
/// have values from the closed range `[-1, 1]`, where -1 points inwards,
/// 0 is perpendicular to the screen towards the viewer, and 1 points outwards.
pub trait DrawShaded: Draw {
    /// Add a shaded square to the draw buffer
    fn shaded_square(&mut self, region: Region, rect: Rect, norm: (f32, f32), col: Colour);

    /// Add a shaded circle to the draw buffer
    fn shaded_circle(&mut self, region: Region, rect: Rect, norm: (f32, f32), col: Colour);

    /// Add a square shaded frame to the draw buffer.
    fn shaded_square_frame(
        &mut self,
        region: Region,
        outer: Rect,
        inner: Rect,
        norm: (f32, f32),
        col: Colour,
    );

    /// Add a rounded shaded frame to the draw buffer.
    fn shaded_round_frame(
        &mut self,
        region: Region,
        outer: Rect,
        inner: Rect,
        norm: (f32, f32),
        col: Colour,
    );
}
