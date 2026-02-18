// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing APIs — draw interface

use super::{AnimationState, color::Rgba};
#[allow(unused)] use super::{DrawRounded, DrawRoundedImpl};
use super::{DrawShared, DrawSharedImpl, ImageId, PassId, PassType, SharedState, WindowCommon};
use crate::geom::{Offset, Quad, Rect, Vec2};
use crate::text::{Effect, TextDisplay};
use std::any::Any;
use std::time::Instant;

/// Draw interface object
///
/// [`Draw`] and extension traits such as [`DrawRounded`] provide draw
/// functionality over this object.
///
/// This type is used to present a unified mid-level draw interface, as
/// available from [`crate::theme::DrawCx::draw`].
/// A concrete `DrawIface` object may be obtained via downcast, e.g.:
/// ```ignore
/// # use kas::draw::{DrawIface, DrawRoundedImpl, DrawSharedImpl, DrawCx, DrawRounded, color::Rgba};
/// # use kas::geom::Rect;
/// # struct CircleWidget {
/// #     rect: Rect,
/// # }
/// impl CircleWidget {
///     fn draw(&self, mut draw: DrawCx) {
///         // This type assumes usage of kas_wgpu without a custom draw pipe:
///         type DrawIface = DrawIface<kas_wgpu::draw::DrawPipe<()>>;
///         if let Some(mut draw) = DrawIface::downcast_from(draw.draw()) {
///             draw.circle(self.rect.into(), 0.9, Rgba::BLACK);
///         }
///     }
/// }
/// ```
///
/// This object is effectively a fat pointer to draw state (both window-local
/// and shared components). As such, it is normal to pass *a new copy* created
/// via [`DrawIface::re`] as a method argument. (Note that Rust automatically
/// "reborrows" reference types passed as method arguments, but cannot do so
/// automatically for structs containing references.)
pub struct DrawIface<'a, DS: DrawSharedImpl> {
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    pub draw: &'a mut DS::Draw,
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    pub shared: &'a mut SharedState<DS>,
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    pub pass: PassId,
}

impl<'a, DS: DrawSharedImpl> DrawIface<'a, DS> {
    /// Construct a new instance
    ///
    /// For usage by graphics backends.
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    pub fn new(draw: &'a mut DS::Draw, shared: &'a mut SharedState<DS>) -> Self {
        DrawIface {
            draw,
            shared,
            pass: PassId::new(0),
        }
    }

    /// Attempt to downcast a `&mut dyn Draw` to a concrete [`DrawIface`] object
    ///
    /// Note: Rust does not (yet) support trait-object-downcast: it not possible
    /// to cast from `&mut dyn Draw` to (for example) `&mut dyn DrawRounded`.
    /// Instead, the target type must be the implementing object, which is
    /// provided by the graphics backend (e.g. `kas_wgpu`).
    /// See documentation on this type for an example, or examine
    /// [`clock.rs`](https://github.com/kas-gui/kas/blob/master/examples/clock.rs).
    pub fn downcast_from(obj: &'a mut dyn Draw) -> Option<Self> {
        let pass = obj.get_pass();
        let (draw, shared) = obj.get_fields_as_any_mut();
        let draw = draw.downcast_mut()?;
        let shared = shared.downcast_mut()?;
        Some(DrawIface { draw, shared, pass })
    }

    /// Reborrow with a new lifetime
    pub fn re<'b>(&'b mut self) -> DrawIface<'b, DS>
    where
        'a: 'b,
    {
        DrawIface {
            draw: &mut *self.draw,
            shared: &mut *self.shared,
            pass: self.pass,
        }
    }

    /// Add a draw pass
    ///
    /// Adds a new draw pass. Passes affect draw order (operations in new passes
    /// happen after their parent pass), may clip drawing to a "clip rect"
    /// (see [`Draw::get_clip_rect`]) and may offset (translate) draw
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
    pub fn new_pass(&mut self, rect: Rect, offset: Offset, class: PassType) -> DrawIface<'_, DS> {
        let pass = self.draw.new_pass(self.pass, rect, offset, class);
        DrawIface {
            draw: &mut *self.draw,
            shared: &mut *self.shared,
            pass,
        }
    }

    /// Draw text with a colour
    ///
    /// Text is drawn from `pos` and clipped to `bounding_box`.
    ///
    /// The `text` display must be prepared prior to calling this method.
    /// Typically this is done using a [`crate::theme::Text`] object.
    pub fn text(&mut self, pos: Vec2, bounding_box: Quad, text: &TextDisplay, col: Rgba) {
        self.text_effects(pos, bounding_box, text, &[col], &[]);
    }
}

/// Basic draw interface for [`DrawIface`]
///
/// Most methods draw some feature. Exceptions are those starting with `get_`
/// and [`Self::new_dyn_pass`].
///
/// Additional draw routines are available through extension traits, depending
/// on the graphics backend. Since Rust does not (yet) support trait-object-downcast,
/// accessing these requires reconstruction of the implementing type via
/// [`DrawIface::downcast_from`].
pub trait Draw {
    /// Access shared draw state
    fn shared(&mut self) -> &mut dyn DrawShared;

    /// Request redraw at the next frame time
    ///
    /// Animations should call this each frame until complete.
    fn animate(&mut self);

    /// Request a redraw at a specific time
    ///
    /// This may be used for animations with delays, e.g. flashing. Calling this
    /// method only ensures that the *next* draw happens *no later* than `time`,
    /// thus the method should be called again in each following frame.
    fn animate_at(&mut self, time: Instant);

    /// Get the current draw pass
    fn get_pass(&self) -> PassId;

    /// Cast fields to [`Any`] references
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    fn get_fields_as_any_mut(&mut self) -> (&mut dyn Any, &mut dyn Any);

    /// Add a draw pass
    ///
    /// Adds a new draw pass. Passes affect draw order (operations in new passes
    /// happen after their parent pass), may clip drawing to a "clip rect"
    /// (see [`Draw::get_clip_rect`]) and may offset (translate) draw
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
    fn new_dyn_pass<'b>(
        &'b mut self,
        rect: Rect,
        offset: Offset,
        class: PassType,
    ) -> Box<dyn Draw + 'b>;

    /// Get drawable rect for a draw `pass`
    ///
    /// The result is in the current target's coordinate system, thus normally
    /// `Rect::pos` is zero (but this is not guaranteed).
    ///
    /// (This is not guaranteed to equal the rect passed to
    /// [`DrawIface::new_pass`].)
    fn get_clip_rect(&self) -> Rect;

    /// Draw a rectangle of uniform colour
    ///
    /// Note: where the implementation batches and/or re-orders draw calls,
    /// this should be one of the first items drawn such that almost anything
    /// else will draw "in front of" a rect.
    fn rect(&mut self, rect: Quad, col: Rgba);

    /// Draw a frame of uniform colour
    ///
    /// The frame is defined by the area inside `outer` and not inside `inner`.
    fn frame(&mut self, outer: Quad, inner: Quad, col: Rgba);

    /// Draw a line with uniform colour
    ///
    /// This command draws a line segment between the points `p1` and `p2`.
    ///
    /// The line will be roughly `width` pixels wide and may exhibit aliasing
    /// (appearance is implementation-defined). Unless you are targetting only
    /// the simplest of backends you probably don't want to use this.
    ///
    /// Note that for rectangular, axis-aligned lines, [`DrawImpl::rect`] should be
    /// preferred.
    fn line(&mut self, p1: Vec2, p2: Vec2, width: f32, col: Rgba);

    /// Draw the image in the given `rect`
    fn image(&mut self, id: ImageId, rect: Quad);

    /// Draw text with effects
    ///
    /// Text is drawn from `pos` and clipped to `bounding_box`.
    ///
    /// The `effects` list provides underlining/strikethrough information via
    /// [`Effect::flags`] and an index [`Effect::e`].
    ///
    /// Text colour lookup uses index `e` and is essentially:
    /// `colors.get(e).unwrap_or(Rgba::BLACK)`.
    ///
    /// The `text` display must be prepared prior to calling this method.
    /// Typically this is done using a [`crate::theme::Text`] object.
    fn text_effects(
        &mut self,
        pos: Vec2,
        bounding_box: Quad,
        text: &TextDisplay,
        colors: &[Rgba],
        effects: &[Effect],
    );
}

impl<'a, DS: DrawSharedImpl> Draw for DrawIface<'a, DS> {
    fn shared(&mut self) -> &mut dyn DrawShared {
        self.shared
    }

    fn animate(&mut self) {
        self.draw.animate();
    }

    fn animate_at(&mut self, time: Instant) {
        self.draw.animate_at(time);
    }

    fn get_pass(&self) -> PassId {
        self.pass
    }

    fn get_fields_as_any_mut(&mut self) -> (&mut dyn Any, &mut dyn Any) {
        (self.draw, self.shared)
    }

    fn new_dyn_pass<'b>(
        &'b mut self,
        rect: Rect,
        offset: Offset,
        class: PassType,
    ) -> Box<dyn Draw + 'b> {
        Box::new(self.new_pass(rect, offset, class))
    }

    fn get_clip_rect(&self) -> Rect {
        self.draw.get_clip_rect(self.pass)
    }

    fn rect(&mut self, rect: Quad, col: Rgba) {
        self.draw.rect(self.pass, rect, col);
    }
    fn frame(&mut self, outer: Quad, inner: Quad, col: Rgba) {
        self.draw.frame(self.pass, outer, inner, col);
    }
    fn line(&mut self, p1: Vec2, p2: Vec2, width: f32, col: Rgba) {
        self.draw.line(self.pass, p1, p2, width, col);
    }

    fn image(&mut self, id: ImageId, rect: Quad) {
        self.shared.draw.draw_image(self.draw, self.pass, id, rect);
    }

    fn text_effects(
        &mut self,
        pos: Vec2,
        bb: Quad,
        text: &TextDisplay,
        colors: &[Rgba],
        effects: &[Effect],
    ) {
        self.shared
            .draw
            .draw_text_effects(self.draw, self.pass, pos, bb, text, colors, effects);
    }
}

/// Implementation target for [`Draw`]
///
/// This trait covers only the bare minimum of functionality which *must* be
/// provided by the graphics backend; extension traits such as [`DrawRoundedImpl`]
/// optionally provide more functionality.
///
/// Coordinates for many primitives are specified using floating-point types
/// allowing fractional precision, deliberately excepting text which must be
/// pixel-aligned for best appearance.
///
/// All draw operations may be batched; when drawn primitives overlap, the
/// results are only loosely defined. Draw operations involving transparency
/// should be ordered after those without transparency.
///
/// Draw operations take place over multiple render passes, identified by a
/// handle of type [`PassId`]. In general the user only needs to pass this value
/// into methods as required. [`DrawImpl::new_pass`] creates a new [`PassId`].
pub trait DrawImpl: Any {
    /// Access common data
    fn common_mut(&mut self) -> &mut WindowCommon;

    /// Request redraw at the next frame time
    ///
    /// Animations should call this each frame until complete.
    fn animate(&mut self) {
        self.common_mut().anim.merge_in(AnimationState::Animate);
    }

    /// Request a redraw at a specific time
    ///
    /// This may be used for animations with delays, e.g. flashing. Calling this
    /// method only ensures that the *next* draw happens *no later* than `time`,
    /// thus the method should be called again in each following frame.
    fn animate_at(&mut self, time: Instant) {
        self.common_mut().anim.merge_in(AnimationState::Timed(time));
    }

    /// Add a draw pass
    ///
    /// Adds a new draw pass. Passes have the following effects:
    ///
    /// -   Draw operations of a pass occur *after* those of the parent pass
    /// -   Drawing is clipped to `rect` (in the base's coordinate space) and
    ///     translated by `offset` (relative to the base's offset)
    ///
    /// The *parent pass* is the one used as the `self` argument of this method.
    /// The *base pass* is dependent on `class`:
    ///
    /// -   `PassType::Clip`: the base is the parent
    /// -   `PassType::Overlay`: the base is the initial pass (i.e. whole window
    ///     with no offset)
    fn new_pass(
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
    /// [`DrawImpl::new_pass`].)
    fn get_clip_rect(&self, pass: PassId) -> Rect;

    /// Draw a rectangle of uniform colour
    fn rect(&mut self, pass: PassId, rect: Quad, col: Rgba);

    /// Draw a frame of uniform colour
    fn frame(&mut self, pass: PassId, outer: Quad, inner: Quad, col: Rgba);

    /// Draw a line segment of uniform colour
    fn line(&mut self, pass: PassId, p1: Vec2, p2: Vec2, width: f32, col: Rgba);
}
