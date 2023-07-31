// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing APIs — draw interface

use super::{color::Rgba, AnimationState};
#[allow(unused)] use super::{DrawRounded, DrawRoundedImpl};
use super::{DrawShared, DrawSharedImpl, ImageId, PassId, PassType, SharedState, WindowCommon};
use crate::geom::{Offset, Quad, Rect};
#[allow(unused)] use crate::text::TextApi;
use crate::text::{Effect, TextDisplay};
use std::any::Any;
use std::time::Instant;

/// Draw interface object
///
/// [`Draw`] and extension traits such as [`DrawRounded`] provide draw
/// functionality over this object.
///
/// This type is used to present a unified mid-level draw interface, as
/// available from [`crate::theme::DrawCx::draw_device`].
/// A concrete `DrawIface` object may be obtained via downcast, e.g.:
/// ```ignore
/// # use kas::draw::{DrawIface, DrawRoundedImpl, DrawSharedImpl, DrawCx, DrawRounded, color::Rgba};
/// # use kas::geom::Rect;
/// # struct CircleWidget<DS> {
/// #     rect: Rect,
/// #     _pd: std::marker::PhantomData<DS>,
/// # }
/// impl CircleWidget {
///     fn draw(&mut self, mut draw: DrawCx) {
///         // This type assumes usage of kas_wgpu without a custom draw pipe:
///         type DrawIface = DrawIface<kas_wgpu::draw::DrawPipe<()>>;
///         if let Some(mut draw) = DrawIface::downcast_from(draw.draw_device()) {
///             draw.circle(self.rect.into(), 0.9, Rgba::BLACK);
///         }
///     }
/// }
/// ```
///
/// Note that this object is little more than a mutable reference to the shell's
/// per-window draw state. As such, it is normal to pass *a new copy* created
/// via [`DrawIface::re`] as a method argument. (Note that Rust automatically
/// "reborrows" reference types passed as method arguments, but cannot do so
/// automatically for structs containing references.)
pub struct DrawIface<'a, DS: DrawSharedImpl> {
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub draw: &'a mut DS::Draw,
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub shared: &'a mut SharedState<DS>,
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub pass: PassId,
}

impl<'a, DS: DrawSharedImpl> DrawIface<'a, DS> {
    /// Construct a new instance
    ///
    /// For usage by the (graphical) shell.
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
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
    /// provided by the shell (e.g. `kas_wgpu`). See documentation on this type
    /// for an example, or see examine
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
    pub fn new_pass(&mut self, rect: Rect, offset: Offset, class: PassType) -> DrawIface<DS> {
        let pass = self.draw.new_pass(self.pass, rect, offset, class);
        DrawIface {
            draw: &mut *self.draw,
            shared: &mut *self.shared,
            pass,
        }
    }
}

/// Base drawing interface for [`DrawIface`]
///
/// Most methods draw some feature. Exceptions are those starting with `get_`
/// and [`Self::new_dyn_pass`].
///
/// Additional draw routines are available through extension traits, depending
/// on the shell. Since Rust does not (yet) support trait-object-downcast,
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
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
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

    /// Draw the image in the given `rect`
    fn image(&mut self, id: ImageId, rect: Quad);

    /// Draw text with a colour
    ///
    /// Text is drawn from `rect.pos` and clipped to `rect`. If the text
    /// scrolls, `rect` should be the size of the whole text, not the window.
    ///
    /// It is required to call [`TextApi::prepare`] or equivalent
    /// prior to this method to select a font, font size and perform layout.
    fn text(&mut self, rect: Rect, text: &TextDisplay, col: Rgba);

    /// Draw text with a single color and effects
    ///
    /// Text is drawn from `rect.pos` and clipped to `rect`. If the text
    /// scrolls, `rect` should be the size of the whole text, not the window.
    ///
    /// The effects list does not contain colour information, but may contain
    /// underlining/strikethrough information. It may be empty.
    ///
    /// It is required to call [`TextApi::prepare`] or equivalent
    /// prior to this method to select a font, font size and perform layout.
    fn text_effects(&mut self, rect: Rect, text: &TextDisplay, col: Rgba, effects: &[Effect<()>]);

    /// Draw text with effects (including [`Rgba`] color)
    ///
    /// Text is drawn from `rect.pos` and clipped to `rect`. If the text
    /// scrolls, `rect` should be the size of the whole text, not the window.
    ///
    /// The `effects` list provides both underlining and colour information.
    /// If the `effects` list is empty or the first entry has `start > 0`, a
    /// default entity will be assumed.
    ///
    /// It is required to call [`TextApi::prepare`] or equivalent
    /// prior to this method to select a font, font size and perform layout.
    fn text_effects_rgba(&mut self, rect: Rect, text: &TextDisplay, effects: &[Effect<Rgba>]);
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

    fn image(&mut self, id: ImageId, rect: Quad) {
        self.shared.draw.draw_image(self.draw, self.pass, id, rect);
    }

    fn text(&mut self, rect: Rect, text: &TextDisplay, col: Rgba) {
        self.shared
            .draw
            .draw_text(self.draw, self.pass, rect, text, col);
    }

    fn text_effects(&mut self, rect: Rect, text: &TextDisplay, col: Rgba, effects: &[Effect<()>]) {
        self.shared
            .draw
            .draw_text_effects(self.draw, self.pass, rect, text, col, effects);
    }

    fn text_effects_rgba(&mut self, rect: Rect, text: &TextDisplay, effects: &[Effect<Rgba>]) {
        self.shared
            .draw
            .draw_text_effects_rgba(self.draw, self.pass, rect, text, effects);
    }
}

/// Base abstraction over drawing
///
/// This trait covers only the bare minimum of functionality which *must* be
/// provided by the shell; extension traits such as [`DrawRoundedImpl`]
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
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
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
}
