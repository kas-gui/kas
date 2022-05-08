// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! "Handle" types used by themes

use std::convert::AsRef;
use std::ops::{Bound, Range, RangeBounds};

use super::{FrameStyle, MarkStyle, SizeHandle, SizeMgr, TextClass};
use crate::dir::Direction;
use crate::draw::{color::Rgb, Draw, DrawShared, ImageId, PassType};
use crate::event::EventState;
use crate::geom::{Coord, Offset, Rect};
use crate::layout::SetRectMgr;
use crate::macros::autoimpl;
use crate::text::{TextApi, TextDisplay};
use crate::{TkAction, Widget, WidgetId};

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
/// `DrawMgr` is not a `Copy` or `Clone` type; instead it may be "reborrowed"
/// via [`Self::re`]. Widgets should assign their [`WidgetId`] via
/// [`DrawMgr::set_id`] before draw operations.
///
/// -   `draw.checkbox(&*self, self.state);` — note `&*self` to convert from to
///     `&W` from `&mut W`, since the latter would cause borrow conflicts
pub struct DrawMgr<'a> {
    h: &'a mut dyn DrawHandle,
    id: WidgetId,
}

impl<'a> DrawMgr<'a> {
    /// Reborrow with a new lifetime
    ///
    /// Rust allows references like `&T` or `&mut T` to be "reborrowed" through
    /// coercion: essentially, the pointer is copied under a new, shorter, lifetime.
    /// Until rfcs#1403 lands, reborrows on user types require a method call.
    ///
    /// The embedded [`WidgetId`] is cloned.
    #[inline(always)]
    pub fn re<'b>(&'b mut self) -> DrawMgr<'b>
    where
        'a: 'b,
    {
        DrawMgr {
            h: self.h,
            id: self.id.clone(),
        }
    }

    /// Construct from a [`DrawMgr`] and [`EventState`]
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub fn new(h: &'a mut dyn DrawHandle, id: WidgetId) -> Self {
        DrawMgr { h, id }
    }

    /// Assign a new [`WidgetId`]
    #[inline(always)]
    pub fn set_id(&mut self, id: WidgetId) {
        self.id = id;
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
    pub fn with_clip_region<F: FnOnce(DrawMgr)>(&mut self, rect: Rect, offset: Offset, f: F) {
        let id = self.id.clone();
        self.h.new_pass(
            rect,
            offset,
            PassType::Clip,
            Box::new(|h| f(DrawMgr { h, id })),
        );
    }

    /// Draw to a new pass as an overlay (e.g. for pop-up menus)
    ///
    /// Adds a new draw pass of type [`PassType::Overlay`], with draw operations
    /// clipped to `rect`.
    ///
    /// The theme is permitted to enlarge the `rect` for the purpose of drawing
    /// a frame or shadow around this overlay, thus the
    /// [`Self::get_clip_rect`] may be larger than expected.
    pub fn with_overlay<F: FnOnce(DrawMgr)>(&mut self, rect: Rect, f: F) {
        let id = self.id.clone();
        self.h.new_pass(
            rect,
            Offset::ZERO,
            PassType::Overlay,
            Box::new(|h| f(DrawMgr { h, id })),
        );
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
    pub fn frame(&mut self, rect: Rect, style: FrameStyle, bg: Background) {
        self.h.frame(&self.id, rect, style, bg)
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
    /// [`Self::text`] already supports *font* effects: bold,
    /// emphasis, text size. In addition, this method supports underline and
    /// strikethrough effects.
    ///
    /// [`SizeMgr::text_bound`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    pub fn text_effects(&mut self, pos: Coord, text: &dyn TextApi, class: TextClass) {
        self.h.text_effects(&self.id, pos, text, class);
    }

    /// Draw some text using the standard font, with a subset selected
    ///
    /// Other than visually highlighting the selection, this method behaves
    /// identically to [`Self::text`]. It is likely to be replaced in the
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

    /// Draw UI element: mark
    pub fn mark(&mut self, rect: Rect, style: MarkStyle) {
        self.h.mark(&self.id, rect, style);
    }

    /// Draw UI element: scrollbar
    pub fn scrollbar(&mut self, track_rect: Rect, handle: &dyn Widget, dir: Direction) {
        self.h
            .scrollbar(&self.id, handle.id_ref(), track_rect, handle.rect(), dir);
    }

    /// Draw UI element: slider
    pub fn slider(&mut self, track_rect: Rect, handle: &dyn Widget, dir: Direction) {
        self.h
            .slider(&self.id, handle.id_ref(), track_rect, handle.rect(), dir);
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
    pub fn image(&mut self, rect: Rect, id: ImageId) {
        self.h.image(id, rect);
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
    fn new_pass<'a>(
        &mut self,
        rect: Rect,
        offset: Offset,
        class: PassType,
        f: Box<dyn FnOnce(&mut dyn DrawHandle) + 'a>,
    );

    /// Target area for drawing
    ///
    /// Drawing is restricted to this [`Rect`]. Affected by [`Self::new_pass`].
    /// This may be used to cull hidden items from lists inside a scrollable view.
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

    /// Method used to implement [`DrawMgr::text_selected`]
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

    /// Draw UI element: mark
    fn mark(&mut self, id: &WidgetId, rect: Rect, style: MarkStyle);

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
        let text = crate::text::Text::new_single("sample");
        let class = TextClass::Label(false);
        draw.text_selected(Coord::ZERO, &text, .., class)
    }
}
