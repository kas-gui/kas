// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing APIs — draw interface

use super::color::Rgba;
use super::{Pass, RegionClass};
use crate::geom::{Quad, Rect, Vec2};
use std::any::Any;

/// Interface over a (local) draw object
///
/// A [`Draw`] object is local to a draw context and may be created by the shell
/// or from another [`Draw`] object via upcast/downcast/reborrow.
///
/// Note that this object is little more than a mutable reference to the shell's
/// per-window draw state. As such, it is normal to pass *a new copy* created
/// via [`Draw::reborrow`] as a method argument. (Note that Rust automatically
/// "reborrows" reference types passed as method arguments, but cannot do so
/// (automatically) for methods containing references.)
///
/// This is created over a [`Drawable`] object created by the shell. The
/// [`Drawable`] trait provides a very limited set of draw routines; the shell
/// may implement extension traits such as [`DrawableRounded`],
/// [`DrawableShaded`], or traits defined elsewhere.
///
/// The [`Draw`] object provides a "medium level" interface over known
/// "drawable" traits, for example one may use [`Draw::circle`] when
/// [`DrawableRounded`] is implemented. In other cases one may directly use
/// [`Draw::draw`], passing the result of [`Draw::pass`] as a parameter.
pub struct Draw<'a, D: Any + ?Sized> {
    pass: Pass,
    pub draw: &'a mut D,
}

#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
impl<'a, D: Drawable + ?Sized> Draw<'a, D> {
    /// Construct (this is only called by the shell)
    pub fn new(draw: &'a mut D, pass: Pass) -> Self {
        Draw { pass, draw }
    }
}

impl<'a> Draw<'a, dyn Any> {
    /// Attempt to downcast to a derived type
    ///
    /// Rust does not (yet) support casting to trait object types (e.g.
    /// `dyn Drawable`); instead we can only downcast to the shell's
    /// implementing type, e.g. `kas_wgpu::draw::DrawWindow<()>`.
    pub fn downcast<'b, D>(&'b mut self) -> Option<Draw<'b, D>>
    where
        'a: 'b,
        D: Drawable,
    {
        let pass = self.pass;
        self.draw.downcast_mut().map(|draw| Draw { pass, draw })
    }
}

impl<'a> Draw<'a, dyn Drawable> {
    /// Attempt to downcast to a derived type
    ///
    /// Rust does not (yet) support casting to trait object types (e.g.
    /// `dyn Drawable`); instead we can only downcast to the shell's
    /// implementing type, e.g. `kas_wgpu::draw::DrawWindow<()>`.
    pub fn downcast<'b, D>(&'b mut self) -> Option<Draw<'b, D>>
    where
        'a: 'b,
        D: Drawable,
    {
        let pass = self.pass;
        self.draw
            .as_any_mut()
            .downcast_mut()
            .map(|draw| Draw { pass, draw })
    }
}

impl<'a, D: Drawable + ?Sized> Draw<'a, D> {
    /// Reborrow with a new lifetime
    pub fn reborrow<'b>(&'b mut self) -> Draw<'b, D>
    where
        'a: 'b,
    {
        Draw {
            draw: &mut *self.draw,
            pass: self.pass,
        }
    }

    /// Upcast to `dyn Drawable` type
    pub fn upcast_base<'b>(&'b mut self) -> Draw<'b, dyn Drawable>
    where
        'a: 'b,
    {
        Draw {
            draw: self.draw.as_drawable_mut(),
            pass: self.pass,
        }
    }

    /// Get current pass
    pub fn pass(&self) -> Pass {
        self.pass
    }

    /// Construct a clip region
    ///
    /// A clip region is a draw target within the same window with a new scissor
    /// rect (i.e. draw operations will be clipped to this `rect`).
    pub fn new_clip_region(&mut self, rect: Rect, class: RegionClass) -> Draw<D> {
        let pass = self.draw.add_clip_region(self.pass, rect, class);
        Draw {
            draw: &mut *self.draw,
            pass,
        }
    }

    /// Get drawable rect for current target
    ///
    /// (This may be smaller than the rect passed to [`Drawable::add_clip_region`].)
    pub fn clip_rect(&self) -> Rect {
        self.draw.get_clip_rect(self.pass)
    }

    /// Draw a rectangle of uniform colour
    pub fn rect(&mut self, rect: Quad, col: Rgba) {
        self.draw.rect(self.pass, rect, col);
    }

    /// Draw a frame of uniform colour
    ///
    /// The frame is defined by the area inside `outer` and not inside `inner`.
    pub fn frame(&mut self, outer: Quad, inner: Quad, col: Rgba) {
        self.draw.frame(self.pass, outer, inner, col);
    }
}

impl<'a, D: DrawableRounded + ?Sized> Draw<'a, D> {
    /// Draw a line with rounded ends and uniform colour
    ///
    /// This command draws a line segment between the points `p1` and `p2`.
    /// Pixels within the given `radius` of this segment are drawn, resulting
    /// in rounded ends and width `2 * radius`.
    ///
    /// Note that for rectangular, axis-aligned lines, [`Drawable::rect`] should be
    /// preferred.
    pub fn rounded_line(&mut self, p1: Vec2, p2: Vec2, radius: f32, col: Rgba) {
        self.draw.rounded_line(self.pass, p1, p2, radius, col);
    }

    /// Draw a circle or oval of uniform colour
    ///
    /// More generally, this shape is an axis-aligned oval which may be hollow.
    ///
    /// The `inner_radius` parameter gives the inner radius relative to the
    /// outer radius: a value of `0.0` will result in the whole shape being
    /// painted, while `1.0` will result in a zero-width line on the outer edge.
    pub fn circle(&mut self, rect: Quad, inner_radius: f32, col: Rgba) {
        self.draw.circle(self.pass, rect, inner_radius, col);
    }

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
    pub fn rounded_frame(&mut self, outer: Quad, inner: Quad, inner_radius: f32, col: Rgba) {
        self.draw
            .rounded_frame(self.pass, outer, inner, inner_radius, col);
    }
}

impl<'a, D: DrawableShaded + ?Sized> Draw<'a, D> {
    /// Add a shaded square to the draw buffer
    pub fn shaded_square(&mut self, rect: Quad, norm: (f32, f32), col: Rgba) {
        self.draw.shaded_square(self.pass, rect, norm, col);
    }

    /// Add a shaded circle to the draw buffer
    pub fn shaded_circle(&mut self, rect: Quad, norm: (f32, f32), col: Rgba) {
        self.draw.shaded_circle(self.pass, rect, norm, col);
    }

    /// Add a square shaded frame to the draw buffer.
    pub fn shaded_square_frame(
        &mut self,
        outer: Quad,
        inner: Quad,
        norm: (f32, f32),
        outer_col: Rgba,
        inner_col: Rgba,
    ) {
        self.draw
            .shaded_square_frame(self.pass, outer, inner, norm, outer_col, inner_col);
    }

    /// Add a rounded shaded frame to the draw buffer.
    pub fn shaded_round_frame(&mut self, outer: Quad, inner: Quad, norm: (f32, f32), col: Rgba) {
        self.draw
            .shaded_round_frame(self.pass, outer, inner, norm, col);
    }
}

/// Base abstraction over drawing
///
/// This trait covers only the bare minimum of functionality which *must* be
/// provided by the shell; extension traits such as [`DrawableRounded`] and
/// [`DrawableShaded`] optionally provide more functionality.
///
/// Coordinates are specified via a [`Vec2`] and rectangular regions via
/// [`Quad`] allowing fractional positions.
///
/// All draw operations may be batched; when drawn primitives overlap, the
/// results are only loosely defined. Draw operations involving transparency
/// should be ordered after those without transparency.
///
/// Draw operations take place over multiple render passes, identified by a
/// handle of type [`Pass`]. In general the user only needs to pass this value
/// into methods as required. [`Drawable::add_clip_region`] creates a new [`Pass`].
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
pub trait Drawable: Any {
    /// Cast self to [`Any`] reference
    ///
    /// A downcast on this value may be used to obtain a reference to a
    /// shell-specific API.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Upcast to dyn drawable
    fn as_drawable_mut(&mut self) -> &mut dyn Drawable;

    /// Add a clip region
    fn add_clip_region(&mut self, pass: Pass, rect: Rect, class: RegionClass) -> Pass;

    /// Get drawable rect for a clip region
    ///
    /// (This may be smaller than the rect passed to [`Drawable::add_clip_region`].)
    fn get_clip_rect(&self, pass: Pass) -> Rect;

    /// Draw a rectangle of uniform colour
    fn rect(&mut self, pass: Pass, rect: Quad, col: Rgba);

    /// Draw a frame of uniform colour
    fn frame(&mut self, pass: Pass, outer: Quad, inner: Quad, col: Rgba);
}

/// Drawing commands for rounded shapes
///
/// This trait is an extension over [`Drawable`] providing rounded shapes.
///
/// The primitives provided by this trait are partially transparent.
/// If the implementation buffers draw commands, it should draw these
/// primitives after solid primitives.
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
pub trait DrawableRounded: Drawable {
    /// Draw a line with rounded ends and uniform colour
    fn rounded_line(&mut self, pass: Pass, p1: Vec2, p2: Vec2, radius: f32, col: Rgba);

    /// Draw a circle or oval of uniform colour
    fn circle(&mut self, pass: Pass, rect: Quad, inner_radius: f32, col: Rgba);

    /// Draw a frame with rounded corners and uniform colour
    fn rounded_frame(&mut self, pass: Pass, outer: Quad, inner: Quad, inner_radius: f32, col: Rgba);
}

/// Drawing commands for shaded shapes
///
/// This trait is an extension over [`Drawable`] providing solid shaded shapes.
///
/// Some drawing primitives (the "round" ones) are partially transparent.
/// If the implementation buffers draw commands, it should draw these
/// primitives after solid primitives.
///
/// These are parameterised via a pair of normals, `(inner, outer)`. These may
/// have values from the closed range `[-1, 1]`, where -1 points inwards,
/// 0 is perpendicular to the screen towards the viewer, and 1 points outwards.
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
pub trait DrawableShaded: Drawable {
    /// Add a shaded square to the draw buffer
    fn shaded_square(&mut self, pass: Pass, rect: Quad, norm: (f32, f32), col: Rgba);

    /// Add a shaded circle to the draw buffer
    fn shaded_circle(&mut self, pass: Pass, rect: Quad, norm: (f32, f32), col: Rgba);

    /// Add a square shaded frame to the draw buffer.
    fn shaded_square_frame(
        &mut self,
        pass: Pass,
        outer: Quad,
        inner: Quad,
        norm: (f32, f32),
        outer_col: Rgba,
        inner_col: Rgba,
    );

    /// Add a rounded shaded frame to the draw buffer.
    fn shaded_round_frame(
        &mut self,
        pass: Pass,
        outer: Quad,
        inner: Quad,
        norm: (f32, f32),
        col: Rgba,
    );
}
