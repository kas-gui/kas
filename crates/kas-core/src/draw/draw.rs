// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing APIs — draw interface

use super::color::Rgba;
#[allow(unused)]
use super::{DrawRounded, DrawRoundedImpl};
use super::{DrawSharedImpl, ImageId, PassId, PassType, SharedState};
use crate::geom::{Offset, Quad, Rect, Vec2};
#[allow(unused)]
use crate::text::TextApi;
use crate::text::{Effect, TextDisplay};
use std::any::Any;

/// Draw interface object
///
/// [`Draw`] and extension traits such as [`DrawRounded`] provide draw
/// functionality over this object.
///
/// This type is used to present a unified mid-level draw interface, as
/// available from [`crate::theme::DrawMgr::draw_device`].
/// A concrete `DrawIface` object may be obtained via downcast, e.g.:
/// ```ignore
/// # use kas::draw::{DrawIface, DrawRoundedImpl, DrawSharedImpl, DrawMgr, DrawRounded, color::Rgba};
/// # use kas::geom::Rect;
/// # struct CircleWidget<DS> {
/// #     rect: Rect,
/// #     _pd: std::marker::PhantomData<DS>,
/// # }
/// impl CircleWidget {
///     fn draw(&mut self, mut draw: DrawMgr) {
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
/// via [`DrawIface::reborrow`] as a method argument. (Note that Rust automatically
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
    pub fn reborrow<'b>(&'b mut self) -> DrawIface<'b, DS>
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
    /// (see [`DrawIface::get_clip_rect`]) and may offset (translate) draw
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
    /// (see [`DrawIface::get_clip_rect`]) and may offset (translate) draw
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
    #[cfg(feature = "stack_dst")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "stack_dst")))]
    fn new_dyn_pass<'b>(
        &'b mut self,
        rect: Rect,
        offset: Offset,
        class: PassType,
    ) -> stack_dst::ValueA<dyn Draw + 'b, [usize; 4]>;

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
    /// It is required to call [`TextDisplay::prepare`] or [`TextApi::prepare`]
    /// prior to this method to select a font, font size and perform layout.
    fn text(&mut self, pos: Vec2, text: &TextDisplay, col: Rgba);

    /// Draw text with a single color and effects
    ///
    /// The effects list does not contain colour information, but may contain
    /// underlining/strikethrough information. It may be empty.
    ///
    /// It is required to call [`TextDisplay::prepare`] or [`TextApi::prepare`]
    /// prior to this method to select a font, font size and perform layout.
    fn text_col_effects(
        &mut self,
        pos: Vec2,
        text: &TextDisplay,
        col: Rgba,
        effects: &[Effect<()>],
    );

    /// Draw text with effects (including colors)
    ///
    /// The `effects` list provides both underlining and colour information.
    /// If the `effects` list is empty or the first entry has `start > 0`, a
    /// default entity will be assumed.
    ///
    /// It is required to call [`TextDisplay::prepare`] or [`TextApi::prepare`]
    /// prior to this method to select a font, font size and perform layout.
    fn text_effects(&mut self, pos: Vec2, text: &TextDisplay, effects: &[Effect<Rgba>]);
}

impl<'a, DS: DrawSharedImpl> Draw for DrawIface<'a, DS> {
    fn get_pass(&self) -> PassId {
        self.pass
    }

    fn get_fields_as_any_mut(&mut self) -> (&mut dyn Any, &mut dyn Any) {
        (self.draw, self.shared)
    }

    #[cfg(feature = "stack_dst")]
    fn new_dyn_pass<'b>(
        &'b mut self,
        rect: Rect,
        offset: Offset,
        class: PassType,
    ) -> stack_dst::ValueA<dyn Draw + 'b, [usize; 4]> {
        let draw = self.new_pass(rect, offset, class);
        stack_dst::ValueA::new_stable(draw, |d| d as &dyn Draw)
            .unwrap_or_else(|_| panic!("boxed window too big for StackDst!"))
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

/// Base abstraction over drawing
///
/// This trait covers only the bare minimum of functionality which *must* be
/// provided by the shell; extension traits such as [`DrawRoundedImpl`]
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
/// into methods as required. [`DrawImpl::new_pass`] creates a new [`PassId`].
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
pub trait DrawImpl: Any {
    /// Add a draw pass
    ///
    /// Adds a new draw pass. Passes affect draw order (operations in new passes
    /// happen after their parent pass), may clip drawing to a "clip rect"
    /// (see [`DrawImpl::get_clip_rect`]) and may offset (translate) draw
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
