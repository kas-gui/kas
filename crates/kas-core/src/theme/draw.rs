// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! "Handle" types used by themes

use std::convert::AsRef;
use std::ops::{Bound, Range, RangeBounds};

use super::{FrameStyle, IdCoord, IdRect, SizeHandle, SizeMgr, TextClass};
use crate::dir::Direction;
use crate::draw::{color::Rgb, Draw, DrawShared, ImageId, PassType};
use crate::event::EventState;
use crate::geom::{Coord, Offset, Rect};
use crate::layout::SetRectMgr;
use crate::macros::autoimpl;
use crate::text::{AccelString, Text, TextApi, TextDisplay};
use crate::{TkAction, WidgetId};

/// Optional background colour
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Background {
    /// Use theme/feature's default
    Default,
    /// Error state
    Error,
    /// A given color
    Rgb(Rgb),
}

impl Default for Background {
    fn default() -> Self {
        Background::Default
    }
}

/// Draw interface
///
/// This interface is provided to widgets in [`crate::Layout::draw`].
/// Lower-level interfaces may be accessed through [`Self::draw_device`].
///
/// Draw methods take a "feature"; any type convertible to [`IdRect`] or (for
/// text methods) [`IdCoord`]. For convenience where the target [`Rect`] or
/// [`Coord`] coincides with the widget's own `rect` (or `rect.pos`), this may
/// be constructed from `&CoreData` or `&W where W: WidgetCore`. For example:
///
/// -   `draw.checkbox(&*self, self.state);` — note `&*self` to convert from to
///     `&W` from `&mut W`, since the latter would cause borrow conflicts
pub struct DrawMgr<'a> {
    h: &'a mut dyn DrawHandle,
}

impl<'a> DrawMgr<'a> {
    /// Reborrow with a new lifetime
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

    /// Draw to a new pass with clipping and offset (e.g. for scrolling)
    ///
    /// Adds a new draw pass of type [`PassType::Clip`], with draw operations
    /// clipped to `rect` and translated by `offset.
    pub fn with_clip_region<F: FnMut(DrawMgr)>(&mut self, rect: Rect, offset: Offset, mut f: F) {
        self.h
            .new_pass(rect, offset, PassType::Clip, &mut |h| f(DrawMgr { h }));
    }

    /// Draw to a new pass as an overlay (e.g. for pop-up menus)
    ///
    /// Adds a new draw pass of type [`PassType::Overlay`], with draw operations
    /// clipped to `rect`.
    ///
    /// The theme is permitted to enlarge the `rect` for the purpose of drawing
    /// a frame or shadow around this overlay, thus the
    /// [`Self::get_clip_rect`] may be larger than expected.
    pub fn with_overlay<F: FnMut(DrawMgr)>(&mut self, rect: Rect, mut f: F) {
        self.h
            .new_pass(rect, Offset::ZERO, PassType::Overlay, &mut |h| {
                f(DrawMgr { h })
            });
    }

    /// Target area for drawing
    ///
    /// Drawing is restricted to this [`Rect`], which may be the whole window, a
    /// [clip region](Self::with_clip_region) or an
    /// [overlay](Self::with_overlay). This may be used to cull hidden
    /// items from lists inside a scrollable view.
    pub fn get_clip_rect(&self) -> Rect {
        self.h.get_clip_rect()
    }

    /// Draw a frame inside the given `rect`
    ///
    /// The frame dimensions are given by [`SizeMgr::frame`].
    pub fn frame<'b>(&mut self, feature: impl Into<IdRect<'b>>, style: FrameStyle, bg: Background) {
        let f = feature.into();
        self.h.frame(f.0, f.1, style, bg)
    }

    /// Draw a separator in the given `rect`
    pub fn separator<'b>(&mut self, feature: impl Into<IdRect<'b>>) {
        let f = feature.into();
        self.h.separator(f.1);
    }

    /// Draw a selection box
    ///
    /// This appears as a dashed box or similar around this `rect`. Note that
    /// the selection indicator is drawn *outside* of this rect, within a margin
    /// of size `inner_margin` that is expected to be present around this box.
    pub fn selection_box<'b>(&mut self, feature: impl Into<IdRect<'b>>) {
        let f = feature.into();
        self.h.selection_box(f.1);
    }

    /// Draw text
    ///
    /// [`SizeMgr::text_bound`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    pub fn text<'b>(
        &mut self,
        feature: impl Into<IdCoord<'b>>,
        text: &TextDisplay,
        class: TextClass,
    ) {
        let f = feature.into();
        self.h.text(f.0, f.1, text, class);
    }

    /// Draw text with effects
    ///
    /// [`Self::text`] already supports *font* effects: bold,
    /// emphasis, text size. In addition, this method supports underline and
    /// strikethrough effects.
    ///
    /// [`SizeMgr::text_bound`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    pub fn text_effects<'b>(
        &mut self,
        feature: impl Into<IdCoord<'b>>,
        text: &dyn TextApi,
        class: TextClass,
    ) {
        let f = feature.into();
        self.h.text_effects(f.0, f.1, text, class);
    }

    /// Draw an `AccelString` text
    ///
    /// [`SizeMgr::text_bound`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    pub fn text_accel<'b>(
        &mut self,
        feature: impl Into<IdCoord<'b>>,
        text: &Text<AccelString>,
        class: TextClass,
    ) {
        let f = feature.into();
        self.h.text_accel(f.0, f.1, text, class);
    }

    /// Draw some text using the standard font, with a subset selected
    ///
    /// Other than visually highlighting the selection, this method behaves
    /// identically to [`Self::text`]. It is likely to be replaced in the
    /// future by a higher-level API.
    pub fn text_selected<'b, T: AsRef<TextDisplay>, R: RangeBounds<usize>>(
        &mut self,
        feature: impl Into<IdCoord<'b>>,
        text: T,
        range: R,
        class: TextClass,
    ) {
        let f = feature.into();
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
            .text_selected_range(f.0, f.1, text.as_ref(), range, class);
    }

    /// Draw an edit marker at the given `byte` index on this `text`
    ///
    /// [`SizeMgr::text_bound`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    pub fn text_cursor<'b>(
        &mut self,
        feature: impl Into<IdCoord<'b>>,
        text: &TextDisplay,
        class: TextClass,
        byte: usize,
    ) {
        let f = feature.into();
        self.h.text_cursor(f.0, f.1, text, class, byte);
    }

    /// Draw UI element: checkbox
    ///
    /// The checkbox is a small, usually square, box with or without a check
    /// mark. A checkbox widget may include a text label, but that label is not
    /// part of this element.
    pub fn checkbox<'b>(&mut self, feature: impl Into<IdRect<'b>>, checked: bool) {
        let f = feature.into();
        self.h.checkbox(f.0, f.1, checked);
    }

    /// Draw UI element: radiobox
    ///
    /// This is similar in appearance to a checkbox.
    pub fn radiobox<'b>(&mut self, feature: impl Into<IdRect<'b>>, checked: bool) {
        let f = feature.into();
        self.h.radiobox(f.0, f.1, checked);
    }

    /// Draw UI element: scrollbar
    ///
    /// -   `id2`: [`WidgetId`] of the handle
    /// -   `rect`: area of whole widget (slider track)
    /// -   `h_rect`: area of slider handle
    /// -   `dir`: direction of bar
    pub fn scrollbar<'b, 'c>(
        &mut self,
        feature: impl Into<IdRect<'b>>,
        handle: impl Into<IdRect<'c>>,
        dir: Direction,
    ) {
        let f = feature.into();
        let g = handle.into();
        self.h.scrollbar(f.0, g.0, f.1, g.1, dir);
    }

    /// Draw UI element: slider
    ///
    /// -   `id2`: [`WidgetId`] of the handle
    /// -   `rect`: area of whole widget (slider track)
    /// -   `h_rect`: area of slider handle
    /// -   `dir`: direction of slider (currently only LTR or TTB)
    pub fn slider<'b, 'c>(
        &mut self,
        feature: impl Into<IdRect<'b>>,
        handle: impl Into<IdRect<'c>>,
        dir: Direction,
    ) {
        let f = feature.into();
        let g = handle.into();
        self.h.slider(f.0, g.0, f.1, g.1, dir);
    }

    /// Draw UI element: progress bar
    ///
    /// -   `rect`: area of whole widget
    /// -   `dir`: direction of progress bar
    /// -   `state`: highlighting information
    /// -   `value`: progress value, between 0.0 and 1.0
    pub fn progress_bar<'b>(&mut self, feature: impl Into<IdRect<'b>>, dir: Direction, value: f32) {
        let f = feature.into();
        self.h.progress_bar(f.0, f.1, dir, value);
    }

    /// Draw an image
    pub fn image<'b>(&mut self, feature: impl Into<IdRect<'b>>, id: ImageId) {
        let f = feature.into();
        self.h.image(id, f.1);
    }
}

impl<'a> std::ops::BitOrAssign<TkAction> for DrawMgr<'a> {
    #[inline]
    fn bitor_assign(&mut self, action: TkAction) {
        self.h.components().2.send_action(action);
    }
}

/// A handle to the active theme, used for drawing
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
#[autoimpl(for<H: trait + ?Sized> Box<H>)]
#[cfg_attr(feature = "stack_dst", autoimpl(
    for<H: trait + ?Sized, S: Default + Copy + AsRef<[usize]> + AsMut<[usize]>>
    stack_dst::ValueA<H, S>
))]
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
    /// [clip region](Self::with_clip_region) or an
    /// [overlay](Self::with_overlay). This may be used to cull hidden
    /// items from lists inside a scrollable view.
    fn get_clip_rect(&self) -> Rect;

    /// Draw a frame inside the given `rect`
    ///
    /// The frame dimensions are given by [`SizeHandle::frame`].
    fn frame(&mut self, id: &WidgetId, rect: Rect, style: FrameStyle, bg: Background);

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

    /// Method used to implement [`Self::text_selected`]
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

#[cfg(test)]
mod test {
    use super::*;

    fn _draw_handle_ext(mut draw: DrawMgr) {
        // We can't call this method without constructing an actual DrawHandle.
        // But we don't need to: we just want to test that methods are callable.

        let _scale = draw.size_mgr().scale_factor();

        let id = WidgetId::ROOT;
        let feature = IdCoord(&id, Coord::ZERO);
        let text = crate::text::Text::new_single("sample");
        let class = TextClass::Label;
        draw.text_selected(feature, &text, .., class)
    }
}
