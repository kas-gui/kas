// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing APIs — draw interface

use super::color::Rgba;
use super::{DrawShared, DrawableShared, ImageId, PassId, PassType};
use crate::geom::{Offset, Quad, Rect, Vec2};
use crate::text::{Effect, TextDisplay};
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
/// automatically for structs containing references.)
///
/// This is created over a [`Drawable`] object created by the shell. The
/// [`Drawable`] trait provides a very limited set of draw routines, beyond
/// which optional traits such as [`DrawableRounded`] may be used.
///
/// The [`Draw`] object provides a "medium level" interface over known
/// "drawable" traits, for example one may use [`Draw::circle`] when
/// [`DrawableRounded`] is implemented. In other cases one may directly use
/// [`Draw::draw`], passing the result of [`Draw::pass`] as a parameter.
pub struct Draw<'a, DS: DrawableShared> {
    pub draw: &'a mut DS::Draw,
    pub shared: &'a mut DrawShared<DS>,
    pass: PassId,
}

#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
impl<'a, DS: DrawableShared> Draw<'a, DS> {
    /// Construct (this is only called by the shell)
    pub fn new(draw: &'a mut DS::Draw, shared: &'a mut DrawShared<DS>, pass: PassId) -> Self {
        Draw { draw, shared, pass }
    }

    /// Attempt to downcast a `&mut dyn DrawT` to a concrete [`Draw`] object
    pub fn downcast_from(obj: &'a mut dyn DrawT) -> Option<Self> {
        let pass = obj.pass();
        let (draw, shared) = obj.fields_as_any_mut();
        let draw = draw.downcast_mut()?;
        let shared = shared.downcast_mut()?;
        Some(Draw::new(draw, shared, pass))
    }
}

impl<'a, DS: DrawableShared> Draw<'a, DS> {
    /// Reborrow with a new lifetime
    pub fn reborrow<'b>(&'b mut self) -> Draw<'b, DS>
    where
        'a: 'b,
    {
        Draw {
            draw: &mut *self.draw,
            shared: &mut *self.shared,
            pass: self.pass,
        }
    }

    /// Get the current draw pass
    pub fn pass(&self) -> PassId {
        self.pass
    }

    /// Add a draw pass
    ///
    /// Adds a new draw pass. Passes affect draw order (operations in new passes
    /// happen after their parent pass), may clip drawing to a "clip rect"
    /// (see [`Draw::clip_rect`]) and may offset (translate) draw
    /// operations.
    ///
    /// Case `class == PassType::Clip`: the new pass is derived from
    /// `parent_pass`; `rect` and `offset` are specified relative to this parent
    /// and the intersecton of `rect` and the parent's "clip rect" is used.
    /// be clipped to `rect` (expressed in the parent's coordinate system).
    ///
    /// Case `class == PassType::Overlay`: the new pass is derived from the
    /// base pass (i.e. the window). Draw operations still happen after those in
    /// `parent_pass`.
    pub fn new_draw_pass(&mut self, rect: Rect, offset: Offset, class: PassType) -> Draw<DS> {
        let pass = self.draw.new_draw_pass(self.pass, rect, offset, class);
        Draw {
            draw: &mut *self.draw,
            shared: &mut *self.shared,
            pass,
        }
    }
}

/// Interface over [`Draw`]
pub trait DrawT {
    /// Get the current draw pass
    fn pass(&self) -> PassId;

    /// Cast fields to [`Any`] references
    fn fields_as_any_mut(&mut self) -> (&mut dyn Any, &mut dyn Any);

    /// Get drawable rect for a draw `pass`
    ///
    /// The result is in the current target's coordinate system, thus normally
    /// `Rect::pos` is zero (but this is not guaranteed).
    ///
    /// (This is not guaranteed to equal the rect passed to
    /// [`Draw::new_draw_pass`].)
    fn clip_rect(&self) -> Rect;

    /// Draw a rectangle of uniform colour
    fn rect(&mut self, rect: Quad, col: Rgba);

    /// Draw a frame of uniform colour
    ///
    /// The frame is defined by the area inside `outer` and not inside `inner`.
    fn frame(&mut self, outer: Quad, inner: Quad, col: Rgba);

    /// Draw the image in the given `rect`
    fn image(&mut self, id: ImageId, rect: Quad);

    /// Draw text with a colour
    fn text(&mut self, pos: Vec2, text: &TextDisplay, col: Rgba);

    /// Draw text with a colour and effects
    ///
    /// The effects list does not contain colour information, but may contain
    /// underlining/strikethrough information. It may be empty.
    fn text_col_effects(
        &mut self,
        pos: Vec2,
        text: &TextDisplay,
        col: Rgba,
        effects: &[Effect<()>],
    );

    /// Draw text with effects
    ///
    /// The `effects` list provides both underlining and colour information.
    /// If the `effects` list is empty or the first entry has `start > 0`, a
    /// default entity will be assumed.
    fn text_effects(&mut self, pos: Vec2, text: &TextDisplay, effects: &[Effect<Rgba>]);
}

impl<'a, DS: DrawableShared> DrawT for Draw<'a, DS> {
    fn pass(&self) -> PassId {
        self.pass
    }

    fn fields_as_any_mut(&mut self) -> (&mut dyn Any, &mut dyn Any) {
        (self.draw, self.shared)
    }

    fn clip_rect(&self) -> Rect {
        self.draw.clip_rect(self.pass)
    }

    fn rect(&mut self, rect: Quad, col: Rgba) {
        self.draw.rect(self.pass, rect, col);
    }
    fn frame(&mut self, outer: Quad, inner: Quad, col: Rgba) {
        self.draw.frame(self.pass, outer, inner, col);
    }

    fn image(&mut self, id: ImageId, rect: Quad) {
        self.shared.draw.draw_image(self.draw, self.pass, id, rect);
    }

    fn text(&mut self, pos: Vec2, text: &TextDisplay, col: Rgba) {
        self.shared
            .draw
            .draw_text(self.draw, self.pass, pos, text, col);
    }

    fn text_col_effects(
        &mut self,
        pos: Vec2,
        text: &TextDisplay,
        col: Rgba,
        effects: &[Effect<()>],
    ) {
        self.shared
            .draw
            .draw_text_col_effects(self.draw, self.pass, pos, text, col, effects);
    }

    fn text_effects(&mut self, pos: Vec2, text: &TextDisplay, effects: &[Effect<Rgba>]) {
        self.shared
            .draw
            .draw_text_effects(self.draw, self.pass, pos, text, effects);
    }
}

/// Extension of [`DrawT`]
pub trait DrawRoundedT: DrawT {
    /// Draw a line with rounded ends and uniform colour
    ///
    /// This command draws a line segment between the points `p1` and `p2`.
    /// Pixels within the given `radius` of this segment are drawn, resulting
    /// in rounded ends and width `2 * radius`.
    ///
    /// Note that for rectangular, axis-aligned lines, [`Drawable::rect`] should be
    /// preferred.
    fn rounded_line(&mut self, p1: Vec2, p2: Vec2, radius: f32, col: Rgba);

    /// Draw a circle or oval of uniform colour
    ///
    /// More generally, this shape is an axis-aligned oval which may be hollow.
    ///
    /// The `inner_radius` parameter gives the inner radius relative to the
    /// outer radius: a value of `0.0` will result in the whole shape being
    /// painted, while `1.0` will result in a zero-width line on the outer edge.
    fn circle(&mut self, rect: Quad, inner_radius: f32, col: Rgba);

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
    fn rounded_frame(&mut self, outer: Quad, inner: Quad, inner_radius: f32, col: Rgba);
}

impl<'a, DS: DrawableShared> DrawRoundedT for Draw<'a, DS>
where
    DS::Draw: DrawableRounded,
{
    fn rounded_line(&mut self, p1: Vec2, p2: Vec2, radius: f32, col: Rgba) {
        self.draw.rounded_line(self.pass, p1, p2, radius, col);
    }
    fn circle(&mut self, rect: Quad, inner_radius: f32, col: Rgba) {
        self.draw.circle(self.pass, rect, inner_radius, col);
    }
    fn rounded_frame(&mut self, outer: Quad, inner: Quad, inner_radius: f32, col: Rgba) {
        self.draw
            .rounded_frame(self.pass, outer, inner, inner_radius, col);
    }
}

/// Base abstraction over drawing
///
/// This trait covers only the bare minimum of functionality which *must* be
/// provided by the shell; extension traits such as [`DrawableRounded`]
/// optionally provide more functionality.
///
/// Coordinates are specified via a [`Vec2`] and rectangular regions via
/// [`Quad`] allowing fractional positions.
///
/// All draw operations may be batched; when drawn primitives overlap, the
/// results are only loosely defined. Draw operations involving transparency
/// should be ordered after those without transparency.
///
/// Draw operations take place over multiple render passes, identified by a
/// handle of type [`PassId`]. In general the user only needs to pass this value
/// into methods as required. [`Drawable::new_draw_pass`] creates a new [`PassId`].
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
pub trait Drawable: Any {
    /// Cast self to [`Any`] reference
    ///
    /// A downcast on this value may be used to obtain a reference to a
    /// shell-specific API.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Upcast to dyn drawable
    fn as_drawable_mut(&mut self) -> &mut dyn Drawable;

    /// Add a draw pass
    ///
    /// Adds a new draw pass. Passes affect draw order (operations in new passes
    /// happen after their parent pass), may clip drawing to a "clip rect"
    /// (see [`Drawable::clip_rect`]) and may offset (translate) draw
    /// operations.
    ///
    /// Case `class == PassType::Clip`: the new pass is derived from
    /// `parent_pass`; `rect` and `offset` are specified relative to this parent
    /// and the intersecton of `rect` and the parent's "clip rect" is used.
    /// be clipped to `rect` (expressed in the parent's coordinate system).
    ///
    /// Case `class == PassType::Overlay`: the new pass is derived from the
    /// base pass (i.e. the window). Draw operations still happen after those in
    /// `parent_pass`.
    fn new_draw_pass(
        &mut self,
        parent_pass: PassId,
        rect: Rect,
        offset: Offset,
        class: PassType,
    ) -> PassId;

    /// Get drawable rect for a draw `pass`
    ///
    /// The result is in the current target's coordinate system, thus normally
    /// `Rect::pos` is zero (but this is not guaranteed).
    ///
    /// (This is not guaranteed to equal the rect passed to
    /// [`Drawable::new_draw_pass`].)
    fn clip_rect(&self, pass: PassId) -> Rect;

    /// Draw a rectangle of uniform colour
    fn rect(&mut self, pass: PassId, rect: Quad, col: Rgba);

    /// Draw a frame of uniform colour
    fn frame(&mut self, pass: PassId, outer: Quad, inner: Quad, col: Rgba);
}

/// Drawing commands for rounded shapes
///
/// This trait is an extension over [`Drawable`] providing rounded shapes.
///
/// The primitives provided by this trait are partially transparent.
/// If the implementation buffers draw commands, it should draw these
/// primitives after solid primitives.
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
pub trait DrawableRounded: Drawable {
    /// Draw a line with rounded ends and uniform colour
    fn rounded_line(&mut self, pass: PassId, p1: Vec2, p2: Vec2, radius: f32, col: Rgba);

    /// Draw a circle or oval of uniform colour
    fn circle(&mut self, pass: PassId, rect: Quad, inner_radius: f32, col: Rgba);

    /// Draw a frame with rounded corners and uniform colour
    fn rounded_frame(
        &mut self,
        pass: PassId,
        outer: Quad,
        inner: Quad,
        inner_radius: f32,
        col: Rgba,
    );
}
