// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! "Handle" types used by themes

use std::convert::AsRef;
use std::ops::{Bound, Deref, DerefMut, Range, RangeBounds};

use super::{FrameStyle, SizeHandle, SizeMgr, TextClass};
use crate::dir::Direction;
use crate::draw::{color::Rgb, Draw, DrawShared, ImageId, PassType};
use crate::event::EventState;
use crate::geom::{Coord, Offset, Rect};
use crate::layout::SetRectMgr;
use crate::text::{AccelString, Text, TextApi, TextDisplay};
use crate::{TkAction, WidgetId};

/// Draw interface
///
/// This interface is provided to widgets in [`crate::Layout::draw`].
/// Lower-level interfaces may be accessed through [`Self::draw_device`].
///
/// Use [`DrawMgr::with_core`] to access draw methods.
pub struct DrawMgr<'a> {
    h: &'a mut dyn DrawHandle,
}

impl<'a> DrawMgr<'a> {
    /// Construct from a [`DrawMgr`] and [`EventState`]
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub fn new(h: &'a mut dyn DrawHandle) -> Self {
        DrawMgr { h }
    }

    /// Access event-management state
    pub fn ev_state(&mut self) -> &EventState {
        self.h.components().2
    }

    /// Access a [`SizeMgr`]
    pub fn size_mgr(&mut self) -> SizeMgr {
        SizeMgr::new(self.h.components().0)
    }

    /// Access a [`SetRectMgr`]
    pub fn set_rect_mgr<F: FnMut(&mut SetRectMgr) -> T, T>(&mut self, mut f: F) -> T {
        let (sh, ds, ev) = self.h.components();
        let mut mgr = SetRectMgr::new(sh, ds, ev);
        f(&mut mgr)
    }

    /// Access a [`DrawShared`]
    pub fn draw_shared(&mut self) -> &mut dyn DrawShared {
        self.h.components().1
    }

    /// Access the low-level draw device
    ///
    /// Note: this drawing API is modular, with limited functionality in the
    /// base trait [`Draw`]. To access further functionality, it is necessary
    /// to downcast with [`crate::draw::DrawIface::downcast_from`].
    pub fn draw_device(&mut self) -> &mut dyn Draw {
        self.h.draw_device()
    }

    /// Add context to allow draw operations
    pub fn with_id<'b>(&'b mut self, id: WidgetId) -> DrawCtx<'b> {
        let h = &mut *self.h;
        DrawCtx { h, id }
    }
}

/// Draw interface (with context)
///
/// This interface is provided to widgets in [`crate::Layout::draw`].
/// Lower-level interfaces may be accessed through [`Self::draw_device`].
///
/// Most methods draw some feature. Corresponding size properties may be
/// obtained through a [`SizeMgr`], e.g. through [`Self::size_mgr`].
pub struct DrawCtx<'a> {
    h: &'a mut dyn DrawHandle,
    // NOTE: it is a little unfortunate that we need a clone of the WidgetId
    // here (a borrow won't work due to borrow conflicts). Still, it's cheap.
    id: WidgetId,
}

impl<'a> DrawCtx<'a> {
    /// Reborrow as a [`DrawMgr`] with a new lifetime
    ///
    /// Rust allows references like `&T` or `&mut T` to be "reborrowed" through
    /// coercion: essentially, the pointer is copied under a new, shorter, lifetime.
    /// Until rfcs#1403 lands, reborrows on user types require a method call.
    #[inline(always)]
    pub fn re<'b>(&'b mut self) -> DrawMgr<'b>
    where
        'a: 'b,
    {
        DrawMgr { h: self.h }
    }

    /// Reborrow as a [`DrawCtx`] with a new lifetime
    ///
    /// Usually one uses [`Self::re`] to construct a [`DrawMgr`] to pass to a
    /// child widget. This `re_ctx` may be useful within a single widget.
    #[inline(always)]
    pub fn re_ctx<'b>(&'b mut self) -> DrawCtx<'b>
    where
        'a: 'b,
    {
        DrawCtx {
            h: self.h,
            id: self.id.clone(),
        }
    }

    /// Access event-management state
    pub fn ev_state(&mut self) -> &EventState {
        self.h.components().2
    }

    /// Access a [`SizeMgr`]
    pub fn size_mgr(&mut self) -> SizeMgr {
        SizeMgr::new(self.h.components().0)
    }

    /// Access a [`SetRectMgr`]
    pub fn set_rect_mgr<F: FnMut(&mut SetRectMgr) -> T, T>(&mut self, mut f: F) -> T {
        let (sh, ds, ev) = self.h.components();
        let mut mgr = SetRectMgr::new(sh, ds, ev);
        f(&mut mgr)
    }

    /// Access a [`DrawShared`]
    pub fn draw_shared(&mut self) -> &mut dyn DrawShared {
        self.h.components().1
    }

    /// Access the low-level draw device
    ///
    /// Note: this drawing API is modular, with limited functionality in the
    /// base trait [`Draw`]. To access further functionality, it is necessary
    /// to downcast with [`crate::draw::DrawIface::downcast_from`].
    pub fn draw_device(&mut self) -> &mut dyn Draw {
        self.h.draw_device()
    }

    /// Draw to a new pass with clipping and offset (e.g. for scrolling)
    ///
    /// Adds a new draw pass of type [`PassType::Clip`], with draw operations
    /// clipped to `rect` and translated by `offset.
    pub fn with_clip_region<F: FnMut(DrawCtx)>(&mut self, rect: Rect, offset: Offset, mut f: F) {
        // NOTE: using FnOnce in DrawHandle::new_pass would let us clone id outside the closure
        let id = &self.id;
        self.h.new_pass(rect, offset, PassType::Clip, &mut |h| {
            let id = id.clone();
            f(DrawCtx { h, id })
        });
    }

    /// Draw to a new pass as an overlay (e.g. for pop-up menus)
    ///
    /// Adds a new draw pass of type [`PassType::Overlay`], with draw operations
    /// clipped to `rect`.
    ///
    /// The theme is permitted to enlarge the `rect` for the purpose of drawing
    /// a frame or shadow around this overlay, thus the
    /// [`DrawCtx::get_clip_rect`] may be larger than expected.
    pub fn with_overlay<F: FnMut(DrawCtx)>(&mut self, rect: Rect, mut f: F) {
        let id = &self.id;
        self.h
            .new_pass(rect, Offset::ZERO, PassType::Overlay, &mut |h| {
                let id = id.clone();
                f(DrawCtx { h, id })
            });
    }

    /// Target area for drawing
    ///
    /// Drawing is restricted to this [`Rect`], which may be the whole window, a
    /// [clip region](DrawCtx::with_clip_region) or an
    /// [overlay](DrawCtx::with_overlay). This may be used to cull hidden
    /// items from lists inside a scrollable view.
    pub fn get_clip_rect(&self) -> Rect {
        self.h.get_clip_rect()
    }

    /// Draw a frame inside the given `rect`
    ///
    /// The frame dimensions are given by [`SizeMgr::frame`].
    ///
    /// Note: for buttons, usage of [`Self::button`] does the same but allowing custom colours.
    pub fn frame(&mut self, rect: Rect, style: FrameStyle) {
        self.h.frame(&self.id, rect, style)
    }

    /// Draw a separator in the given `rect`
    pub fn separator(&mut self, rect: Rect) {
        self.h.separator(rect);
    }

    /// Draw a selection box
    ///
    /// This appears as a dashed box or similar around this `rect`. Note that
    /// the selection indicator is drawn *outside* of this rect, within a margin
    /// of size `inner_margin` that is expected to be present around this box.
    pub fn selection_box(&mut self, rect: Rect) {
        self.h.selection_box(rect);
    }

    /// Draw text
    ///
    /// [`SizeMgr::text_bound`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    pub fn text(&mut self, pos: Coord, text: &TextDisplay, class: TextClass) {
        self.h.text(&self.id, pos, text, class);
    }

    /// Draw text with effects
    ///
    /// [`DrawCtx::text`] already supports *font* effects: bold,
    /// emphasis, text size. In addition, this method supports underline and
    /// strikethrough effects.
    ///
    /// [`SizeMgr::text_bound`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    pub fn text_effects(&mut self, pos: Coord, text: &dyn TextApi, class: TextClass) {
        self.h.text_effects(&self.id, pos, text, class);
    }

    /// Draw an `AccelString` text
    ///
    /// The `text` is drawn within the rect from `pos` to `text.env().bounds`.
    ///
    /// [`SizeMgr::text_bound`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    pub fn text_accel(&mut self, pos: Coord, text: &Text<AccelString>, class: TextClass) {
        self.h.text_accel(&self.id, pos, text, class);
    }

    /// Draw some text using the standard font, with a subset selected
    ///
    /// Other than visually highlighting the selection, this method behaves
    /// identically to [`DrawCtx::text`]. It is likely to be replaced in the
    /// future by a higher-level API.
    pub fn text_selected<T: AsRef<TextDisplay>, R: RangeBounds<usize>>(
        &mut self,
        pos: Coord,
        text: T,
        range: R,
        class: TextClass,
    ) {
        let start = match range.start_bound() {
            Bound::Included(n) => *n,
            Bound::Excluded(n) => *n + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(n) => *n + 1,
            Bound::Excluded(n) => *n,
            Bound::Unbounded => usize::MAX,
        };
        let range = Range { start, end };
        self.h
            .text_selected_range(&self.id, pos, text.as_ref(), range, class);
    }

    /// Draw an edit marker at the given `byte` index on this `text`
    ///
    /// [`SizeMgr::text_bound`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    pub fn text_cursor(&mut self, pos: Coord, text: &TextDisplay, class: TextClass, byte: usize) {
        self.h.text_cursor(&self.id, pos, text, class, byte);
    }

    /// Draw button sides, background and margin-area highlight
    ///
    /// Optionally, a specific colour may be used.
    // TODO: Allow theme-provided named colours?
    pub fn button(&mut self, rect: Rect, col: Option<Rgb>) {
        self.h.button(&self.id, rect, col);
    }

    /// Draw UI element: checkbox
    ///
    /// The checkbox is a small, usually square, box with or without a check
    /// mark. A checkbox widget may include a text label, but that label is not
    /// part of this element.
    pub fn checkbox(&mut self, rect: Rect, checked: bool) {
        self.h.checkbox(&self.id, rect, checked);
    }

    /// Draw UI element: radiobox
    ///
    /// This is similar in appearance to a checkbox.
    pub fn radiobox(&mut self, rect: Rect, checked: bool) {
        self.h.radiobox(&self.id, rect, checked);
    }

    /// Draw UI element: scrollbar
    ///
    /// -   `id2`: [`WidgetId`] of the handle
    /// -   `rect`: area of whole widget (slider track)
    /// -   `h_rect`: area of slider handle
    /// -   `dir`: direction of bar
    pub fn scrollbar(&mut self, id2: &WidgetId, rect: Rect, h_rect: Rect, dir: Direction) {
        self.h.scrollbar(&self.id, id2, rect, h_rect, dir);
    }

    /// Draw UI element: slider
    ///
    /// -   `id2`: [`WidgetId`] of the handle
    /// -   `rect`: area of whole widget (slider track)
    /// -   `h_rect`: area of slider handle
    /// -   `dir`: direction of slider (currently only LTR or TTB)
    pub fn slider(&mut self, id2: &WidgetId, rect: Rect, h_rect: Rect, dir: Direction) {
        self.h.slider(&self.id, id2, rect, h_rect, dir);
    }

    /// Draw UI element: progress bar
    ///
    /// -   `rect`: area of whole widget
    /// -   `dir`: direction of progress bar
    /// -   `state`: highlighting information
    /// -   `value`: progress value, between 0.0 and 1.0
    pub fn progress_bar(&mut self, rect: Rect, dir: Direction, value: f32) {
        self.h.progress_bar(&self.id, rect, dir, value);
    }

    /// Draw an image
    pub fn image(&mut self, id: ImageId, rect: Rect) {
        self.h.image(id, rect);
    }
}

impl<'a> std::ops::BitOrAssign<TkAction> for DrawMgr<'a> {
    #[inline]
    fn bitor_assign(&mut self, action: TkAction) {
        self.h.components().2.send_action(action);
    }
}

impl<'a> std::ops::BitOrAssign<TkAction> for DrawCtx<'a> {
    #[inline]
    fn bitor_assign(&mut self, action: TkAction) {
        self.h.components().2.send_action(action);
    }
}

/// A handle to the active theme, used for drawing
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
pub trait DrawHandle {
    /// Access components: [`SizeHandle`], [`DrawShared`], [`EventState`]
    fn components(&mut self) -> (&dyn SizeHandle, &mut dyn DrawShared, &mut EventState);

    /// Access the low-level draw device
    ///
    /// Note: this drawing API is modular, with limited functionality in the
    /// base trait [`Draw`]. To access further functionality, it is necessary
    /// to downcast with [`crate::draw::DrawIface::downcast_from`].
    fn draw_device(&mut self) -> &mut dyn Draw;

    /// Construct a new pass
    fn new_pass(
        &mut self,
        rect: Rect,
        offset: Offset,
        class: PassType,
        f: &mut dyn FnMut(&mut dyn DrawHandle),
    );

    /// Target area for drawing
    ///
    /// Drawing is restricted to this [`Rect`], which may be the whole window, a
    /// [clip region](DrawCtx::with_clip_region) or an
    /// [overlay](DrawCtx::with_overlay). This may be used to cull hidden
    /// items from lists inside a scrollable view.
    fn get_clip_rect(&self) -> Rect;

    /// Draw a frame inside the given `rect`
    ///
    /// The frame dimensions are given by [`SizeHandle::frame`].
    fn frame(&mut self, id: &WidgetId, rect: Rect, style: FrameStyle);

    /// Draw a separator in the given `rect`
    fn separator(&mut self, rect: Rect);

    /// Draw a selection box
    ///
    /// This appears as a dashed box or similar around this `rect`. Note that
    /// the selection indicator is drawn *outside* of this rect, within a margin
    /// of size `inner_margin` that is expected to be present around this box.
    fn selection_box(&mut self, rect: Rect);

    /// Draw text
    ///
    /// [`SizeMgr::text_bound`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    fn text(&mut self, id: &WidgetId, pos: Coord, text: &TextDisplay, class: TextClass);

    /// Draw text with effects
    ///
    /// [`DrawHandle::text`] already supports *font* effects: bold,
    /// emphasis, text size. In addition, this method supports underline and
    /// strikethrough effects.
    ///
    /// [`SizeMgr::text_bound`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    fn text_effects(&mut self, id: &WidgetId, pos: Coord, text: &dyn TextApi, class: TextClass);

    /// Draw an `AccelString` text
    ///
    /// The `text` is drawn within the rect from `pos` to `text.env().bounds`.
    ///
    /// [`SizeMgr::text_bound`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    fn text_accel(&mut self, id: &WidgetId, pos: Coord, text: &Text<AccelString>, class: TextClass);

    /// Method used to implement [`DrawCtx::text_selected`]
    fn text_selected_range(
        &mut self,
        id: &WidgetId,
        pos: Coord,
        text: &TextDisplay,
        range: Range<usize>,
        class: TextClass,
    );

    /// Draw an edit marker at the given `byte` index on this `text`
    ///
    /// [`SizeMgr::text_bound`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    fn text_cursor(
        &mut self,
        id: &WidgetId,
        pos: Coord,
        text: &TextDisplay,
        class: TextClass,
        byte: usize,
    );

    /// Draw button sides, background and margin-area highlight
    ///
    /// Optionally, a specific colour may be used.
    // TODO: Allow theme-provided named colours?
    fn button(&mut self, id: &WidgetId, rect: Rect, col: Option<Rgb>);

    /// Draw UI element: checkbox
    ///
    /// The checkbox is a small, usually square, box with or without a check
    /// mark. A checkbox widget may include a text label, but that label is not
    /// part of this element.
    fn checkbox(&mut self, id: &WidgetId, rect: Rect, checked: bool);

    /// Draw UI element: radiobox
    ///
    /// This is similar in appearance to a checkbox.
    fn radiobox(&mut self, id: &WidgetId, rect: Rect, checked: bool);

    /// Draw UI element: scrollbar
    ///
    /// -   `id`: [`WidgetId`] of the bar
    /// -   `id2`: [`WidgetId`] of the handle
    /// -   `rect`: area of whole widget (slider track)
    /// -   `h_rect`: area of slider handle
    /// -   `dir`: direction of bar
    fn scrollbar(
        &mut self,
        id: &WidgetId,
        id2: &WidgetId,
        rect: Rect,
        h_rect: Rect,
        dir: Direction,
    );

    /// Draw UI element: slider
    ///
    /// -   `id`: [`WidgetId`] of the bar
    /// -   `id2`: [`WidgetId`] of the handle
    /// -   `rect`: area of whole widget (slider track)
    /// -   `h_rect`: area of slider handle
    /// -   `dir`: direction of slider (currently only LTR or TTB)
    fn slider(&mut self, id: &WidgetId, id2: &WidgetId, rect: Rect, h_rect: Rect, dir: Direction);

    /// Draw UI element: progress bar
    ///
    /// -   `id`: [`WidgetId`] of the bar
    /// -   `rect`: area of whole widget
    /// -   `dir`: direction of progress bar
    /// -   `value`: progress value, between 0.0 and 1.0
    fn progress_bar(&mut self, id: &WidgetId, rect: Rect, dir: Direction, value: f32);

    /// Draw an image
    fn image(&mut self, id: ImageId, rect: Rect);
}

macro_rules! impl_ {
    (($($args:tt)*) DrawHandle for $ty:ty) => {
        impl<$($args)*> DrawHandle for $ty {
            fn components(&mut self) -> (&dyn SizeHandle, &mut dyn DrawShared, &mut EventState) {
                self.deref_mut().components()
            }
            fn draw_device(&mut self) -> &mut dyn Draw {
                self.deref_mut().draw_device()
            }
            fn new_pass(
                &mut self,
                rect: Rect,
                offset: Offset,
                class: PassType,
                f: &mut dyn FnMut(&mut dyn DrawHandle),
            ) {
                self.deref_mut().new_pass(rect, offset, class, f);
            }
            fn get_clip_rect(&self) -> Rect {
                self.deref().get_clip_rect()
            }
            fn frame(&mut self, id: &WidgetId, rect: Rect, style: FrameStyle) {
                self.deref_mut().frame(id, rect, style);
            }
            fn separator(&mut self, rect: Rect) {
                self.deref_mut().separator(rect);
            }
            fn selection_box(&mut self, rect: Rect) {
                self.deref_mut().selection_box(rect);
            }
            fn text(&mut self, id: &WidgetId, pos: Coord, text: &TextDisplay, class: TextClass) {
                self.deref_mut().text(id, pos, text, class)
            }
            fn text_effects(
                &mut self,
                id: &WidgetId, pos: Coord,
                text: &dyn TextApi,
                class: TextClass,
            ) {
                self.deref_mut().text_effects(id, pos, text, class);
            }
            fn text_accel(
                &mut self,
                id: &WidgetId, pos: Coord,
                text: &Text<AccelString>,
                class: TextClass,
            ) {
                self.deref_mut().text_accel(id, pos, text, class);
            }
            fn text_selected_range(
                &mut self,
                id: &WidgetId, pos: Coord,
                text: &TextDisplay,
                range: Range<usize>,
                class: TextClass,
            ) {
                self.deref_mut()
                    .text_selected_range(id, pos, text, range, class);
            }
            fn text_cursor(&mut self, id: &WidgetId, pos: Coord, text: &TextDisplay, class: TextClass, byte: usize) {
                self.deref_mut().text_cursor(id, pos, text, class, byte)
            }
            fn button(&mut self, id: &WidgetId, rect: Rect, col: Option<Rgb>) {
                self.deref_mut().button(id, rect, col)
            }
            fn checkbox(&mut self, id: &WidgetId, rect: Rect, checked: bool) {
                self.deref_mut().checkbox(id, rect, checked)
            }
            fn radiobox(&mut self, id: &WidgetId, rect: Rect, checked: bool) {
                self.deref_mut().radiobox(id, rect, checked)
            }
            fn scrollbar(&mut self, id: &WidgetId, id2: &WidgetId, rect: Rect, h_rect: Rect, dir: Direction) {
                self.deref_mut().scrollbar(id, id2, rect, h_rect, dir)
            }
            fn slider(&mut self, id: &WidgetId, id2: &WidgetId, rect: Rect, h_rect: Rect, dir: Direction) {
                self.deref_mut().slider(id, id2, rect, h_rect, dir)
            }
            fn progress_bar(&mut self, id: &WidgetId, rect: Rect, dir: Direction, value: f32) {
                self.deref_mut().progress_bar(id, rect, dir, value);
            }
            fn image(&mut self, id: ImageId, rect: Rect) {
                self.deref_mut().image(id, rect);
            }
        }
    };
}

impl_! { (H: DrawHandle) DrawHandle for Box<H> }
#[cfg(feature = "stack_dst")]
impl_! {
    ('a, S: Default + Copy + AsRef<[usize]> + AsMut<[usize]>)
    DrawHandle for stack_dst::ValueA<dyn DrawHandle + 'a, S>
}

#[cfg(test)]
mod test {
    use super::*;

    fn _draw_handle_ext(mut draw: DrawMgr) {
        // We can't call this method without constructing an actual DrawHandle.
        // But we don't need to: we just want to test that methods are callable.

        let _scale = draw.size_mgr().scale_factor();
        let mut draw = draw.with_id(WidgetId::ROOT);

        let text = crate::text::Text::new_single("sample");
        let class = TextClass::Label;
        draw.text_selected(Coord::ZERO, &text, .., class)
    }
}
