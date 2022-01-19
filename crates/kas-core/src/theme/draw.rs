// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! "Handle" types used by themes

use std::convert::AsRef;
use std::ops::{Bound, Deref, DerefMut, Range, RangeBounds};

use crate::dir::Direction;
use crate::draw::{color::Rgb, Draw, DrawShared, ImageId, PassType};
use crate::event::EventState;
use crate::geom::{Coord, Offset, Rect};
use crate::layout::SetRectMgr;
use crate::text::{AccelString, Text, TextApi, TextDisplay};
use crate::theme::{InputState, SizeHandle, SizeMgr, TextClass};
use crate::{TkAction, WidgetCore};

/// Draw interface
///
/// This interface is provided to widgets in [`crate::Layout::draw`].
/// Lower-level interfaces may be accessed through [`Self::draw_device`].
///
/// Most methods draw some feature. Corresponding size properties may be
/// obtained through a [`SizeMgr`], e.g. through [`Self::size_mgr`].
///
/// Other notable methods:
///
/// -   [`Self::with_clip_region`] constructs a new pass with clipping
/// -   [`Self::with_overlay`] constructs a new pass for an overlay (e.g. pop-up menu or tooltip)
/// -   [`Self::get_clip_rect`] returns the current clip rect
pub struct DrawMgr<'a>(&'a mut dyn DrawHandle, &'a mut EventState);

impl<'a> DrawMgr<'a> {
    /// Construct from a [`DrawMgr`] and [`EventState`]
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub fn new(h: &'a mut dyn DrawHandle, mgr: &'a mut EventState) -> Self {
        DrawMgr(h, mgr)
    }

    /// Reborrow with a new lifetime
    ///
    /// Rust allows references like `&T` or `&mut T` to be "reborrowed" through
    /// coercion: essentially, the pointer is copied under a new, shorter, lifetime.
    /// Until rfcs#1403 lands, reborrows on user types require a method call.
    ///
    /// Calling this method is zero-cost.
    #[inline(always)]
    pub fn re<'b>(&'b mut self) -> DrawMgr<'b>
    where
        'a: 'b,
    {
        DrawMgr(self.0, self.1)
    }

    /// Access event-management state
    pub fn ev_state(&self) -> &EventState {
        self.1
    }

    /// Shortcut: calculate [`InputState`] for a widget
    pub fn input_state<W: WidgetCore + ?Sized>(&self, w: &W, disabled: bool) -> InputState {
        self.1.draw_state(w.core_data(), disabled)
    }

    /// Access a [`SizeMgr`]
    pub fn size_mgr(&mut self) -> SizeMgr {
        SizeMgr::new(self.0.size_and_draw_shared().0)
    }

    /// Access a [`SetRectMgr`]
    pub fn set_rect_mgr<F: FnMut(&mut SetRectMgr) -> T, T>(&mut self, mut f: F) -> T {
        let (sh, ds) = self.0.size_and_draw_shared();
        let mut mgr = SetRectMgr::new(sh, ds);
        let t = f(&mut mgr);
        self.1.send_action(mgr.take_action());
        t
    }

    /// Access a [`DrawShared`]
    pub fn draw_shared(&mut self) -> &mut dyn DrawShared {
        self.0.size_and_draw_shared().1
    }

    /// Access the low-level draw device
    ///
    /// Note: this drawing API is modular, with limited functionality in the
    /// base trait [`Draw`]. To access further functionality, it is necessary
    /// to downcast with [`crate::draw::DrawIface::downcast_from`].
    pub fn draw_device(&mut self) -> &mut dyn Draw {
        self.0.draw_device()
    }

    /// Draw to a new pass with clipping and offset (e.g. for scrolling)
    ///
    /// Adds a new draw pass of type [`PassType::Clip`], with draw operations
    /// clipped to `rect` and translated by `offset.
    pub fn with_clip_region<F: FnMut(DrawMgr)>(&mut self, rect: Rect, offset: Offset, mut f: F) {
        self.0.new_pass(rect, offset, PassType::Clip, &mut |draw| {
            f(DrawMgr(draw, self.1))
        });
    }

    /// Draw to a new pass as an overlay (e.g. for pop-up menus)
    ///
    /// Adds a new draw pass of type [`PassType::Overlay`], with draw operations
    /// clipped to `rect`.
    ///
    /// The theme is permitted to enlarge the `rect` for the purpose of drawing
    /// a frame or shadow around this overlay, thus the
    /// [`DrawMgr::get_clip_rect`] may be larger than expected.
    pub fn with_overlay<F: FnMut(DrawMgr)>(&mut self, rect: Rect, mut f: F) {
        self.0
            .new_pass(rect, Offset::ZERO, PassType::Overlay, &mut |draw| {
                f(DrawMgr(draw, self.1))
            });
    }

    /// Target area for drawing
    ///
    /// Drawing is restricted to this [`Rect`], which may be the whole window, a
    /// [clip region](DrawMgr::with_clip_region) or an
    /// [overlay](DrawMgr::with_overlay). This may be used to cull hidden
    /// items from lists inside a scrollable view.
    pub fn get_clip_rect(&self) -> Rect {
        self.0.get_clip_rect()
    }

    /// Draw a frame inside the given `rect`
    ///
    /// The frame dimensions equal those of [`SizeMgr::frame`] on each side.
    pub fn outer_frame(&mut self, rect: Rect) {
        self.0.outer_frame(rect)
    }

    /// Draw a separator in the given `rect`
    pub fn separator(&mut self, rect: Rect) {
        self.0.separator(rect);
    }

    /// Draw a navigation highlight frame in the given `rect`
    ///
    /// This is a margin area which may have a some type of navigation highlight
    /// drawn in it, or may be empty.
    pub fn nav_frame(&mut self, rect: Rect, state: InputState) {
        self.0.nav_frame(rect, state);
    }

    /// Draw a selection box
    ///
    /// This appears as a dashed box or similar around this `rect`. Note that
    /// the selection indicator is drawn *outside* of this rect, within a margin
    /// of size `inner_margin` that is expected to be present around this box.
    pub fn selection_box(&mut self, rect: Rect) {
        self.0.selection_box(rect);
    }

    /// Draw text
    ///
    /// [`SizeMgr::text_bound`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    pub fn text(&mut self, pos: Coord, text: &TextDisplay, class: TextClass, state: InputState) {
        self.0.text(pos, text, class, state);
    }

    /// Draw text with effects
    ///
    /// [`DrawMgr::text`] already supports *font* effects: bold,
    /// emphasis, text size. In addition, this method supports underline and
    /// strikethrough effects.
    ///
    /// [`SizeMgr::text_bound`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    pub fn text_effects(
        &mut self,
        pos: Coord,
        text: &dyn TextApi,
        class: TextClass,
        state: InputState,
    ) {
        self.0.text_effects(pos, text, class, state);
    }

    /// Draw an `AccelString` text
    ///
    /// The `text` is drawn within the rect from `pos` to `text.env().bounds`.
    ///
    /// [`SizeMgr::text_bound`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    pub fn text_accel(
        &mut self,
        pos: Coord,
        text: &Text<AccelString>,
        accel: bool,
        class: TextClass,
        state: InputState,
    ) {
        self.0.text_accel(pos, text, accel, class, state);
    }

    /// Draw some text using the standard font, with a subset selected
    ///
    /// Other than visually highlighting the selection, this method behaves
    /// identically to [`DrawMgr::text`]. It is likely to be replaced in the
    /// future by a higher-level API.
    pub fn text_selected<T: AsRef<TextDisplay>, R: RangeBounds<usize>>(
        &mut self,
        pos: Coord,
        text: T,
        range: R,
        class: TextClass,
        state: InputState,
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
        self.0
            .text_selected_range(pos, text.as_ref(), range, class, state);
    }

    /// Draw an edit marker at the given `byte` index on this `text`
    ///
    /// [`SizeMgr::text_bound`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    pub fn text_cursor(&mut self, pos: Coord, text: &TextDisplay, class: TextClass, byte: usize) {
        self.0.text_cursor(pos, text, class, byte);
    }

    /// Draw the background of a menu entry
    pub fn menu_entry(&mut self, rect: Rect, state: InputState) {
        self.0.menu_entry(rect, state);
    }

    /// Draw button sides, background and margin-area highlight
    ///
    /// Optionally, a specific colour may be used.
    // TODO: Allow theme-provided named colours?
    pub fn button(&mut self, rect: Rect, col: Option<Rgb>, state: InputState) {
        self.0.button(rect, col, state);
    }

    /// Draw edit box sides, background and margin-area highlight
    pub fn edit_box(&mut self, rect: Rect, state: InputState) {
        self.0.edit_box(rect, state);
    }

    /// Draw UI element: checkbox
    ///
    /// The checkbox is a small, usually square, box with or without a check
    /// mark. A checkbox widget may include a text label, but that label is not
    /// part of this element.
    pub fn checkbox(&mut self, rect: Rect, checked: bool, state: InputState) {
        self.0.checkbox(rect, checked, state);
    }

    /// Draw UI element: radiobox
    ///
    /// This is similar in appearance to a checkbox.
    pub fn radiobox(&mut self, rect: Rect, checked: bool, state: InputState) {
        self.0.radiobox(rect, checked, state);
    }

    /// Draw UI element: scrollbar
    ///
    /// -   `rect`: area of whole widget (slider track)
    /// -   `h_rect`: area of slider handle
    /// -   `dir`: direction of bar
    /// -   `state`: highlighting information
    pub fn scrollbar(&mut self, rect: Rect, h_rect: Rect, dir: Direction, state: InputState) {
        self.0.scrollbar(rect, h_rect, dir, state);
    }

    /// Draw UI element: slider
    ///
    /// -   `rect`: area of whole widget (slider track)
    /// -   `h_rect`: area of slider handle
    /// -   `dir`: direction of slider (currently only LTR or TTB)
    /// -   `state`: highlighting information
    pub fn slider(&mut self, rect: Rect, h_rect: Rect, dir: Direction, state: InputState) {
        self.0.slider(rect, h_rect, dir, state);
    }

    /// Draw UI element: progress bar
    ///
    /// -   `rect`: area of whole widget
    /// -   `dir`: direction of progress bar
    /// -   `state`: highlighting information
    /// -   `value`: progress value, between 0.0 and 1.0
    pub fn progress_bar(&mut self, rect: Rect, dir: Direction, state: InputState, value: f32) {
        self.0.progress_bar(rect, dir, state, value);
    }

    /// Draw an image
    pub fn image(&mut self, id: ImageId, rect: Rect) {
        self.0.image(id, rect);
    }
}

impl<'a> std::ops::BitOrAssign<TkAction> for DrawMgr<'a> {
    #[inline]
    fn bitor_assign(&mut self, action: TkAction) {
        self.1.send_action(action);
    }
}

/// A handle to the active theme, used for drawing
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
pub trait DrawHandle {
    /// Access a [`SizeHandle`] and a [`DrawShared`]
    fn size_and_draw_shared(&mut self) -> (&dyn SizeHandle, &mut dyn DrawShared);

    /// Access the low-level draw device
    ///
    /// Note: this drawing API is modular, with limited functionality in the
    /// base trait [`Draw`]. To access further functionality, it is necessary
    /// to downcast with [`crate::draw::DrawIface::downcast_from`].
    fn draw_device(&mut self) -> &mut dyn Draw;

    /// Add a draw pass
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
    /// [clip region](DrawMgr::with_clip_region) or an
    /// [overlay](DrawMgr::with_overlay). This may be used to cull hidden
    /// items from lists inside a scrollable view.
    fn get_clip_rect(&self) -> Rect;

    /// Draw a frame inside the given `rect`
    ///
    /// The frame dimensions equal those of [`SizeMgr::frame`] on each side.
    fn outer_frame(&mut self, rect: Rect);

    /// Draw a separator in the given `rect`
    fn separator(&mut self, rect: Rect);

    /// Draw a navigation highlight frame in the given `rect`
    ///
    /// This is a margin area which may have a some type of navigation highlight
    /// drawn in it, or may be empty.
    fn nav_frame(&mut self, rect: Rect, state: InputState);

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
    fn text(&mut self, pos: Coord, text: &TextDisplay, class: TextClass, state: InputState);

    /// Draw text with effects
    ///
    /// [`DrawHandle::text`] already supports *font* effects: bold,
    /// emphasis, text size. In addition, this method supports underline and
    /// strikethrough effects.
    ///
    /// [`SizeMgr::text_bound`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    fn text_effects(&mut self, pos: Coord, text: &dyn TextApi, class: TextClass, state: InputState);

    /// Draw an `AccelString` text
    ///
    /// The `text` is drawn within the rect from `pos` to `text.env().bounds`.
    ///
    /// [`SizeMgr::text_bound`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    fn text_accel(
        &mut self,
        pos: Coord,
        text: &Text<AccelString>,
        accel: bool,
        class: TextClass,
        state: InputState,
    );

    /// Method used to implement [`DrawMgr::text_selected`]
    fn text_selected_range(
        &mut self,
        pos: Coord,
        text: &TextDisplay,
        range: Range<usize>,
        class: TextClass,
        state: InputState,
    );

    /// Draw an edit marker at the given `byte` index on this `text`
    ///
    /// [`SizeMgr::text_bound`] should be called prior to this method to
    /// select a font, font size and wrap options (based on the [`TextClass`]).
    fn text_cursor(&mut self, pos: Coord, text: &TextDisplay, class: TextClass, byte: usize);

    /// Draw the background of a menu entry
    fn menu_entry(&mut self, rect: Rect, state: InputState);

    /// Draw button sides, background and margin-area highlight
    ///
    /// Optionally, a specific colour may be used.
    // TODO: Allow theme-provided named colours?
    fn button(&mut self, rect: Rect, col: Option<Rgb>, state: InputState);

    /// Draw edit box sides, background and margin-area highlight
    fn edit_box(&mut self, rect: Rect, state: InputState);

    /// Draw UI element: checkbox
    ///
    /// The checkbox is a small, usually square, box with or without a check
    /// mark. A checkbox widget may include a text label, but that label is not
    /// part of this element.
    fn checkbox(&mut self, rect: Rect, checked: bool, state: InputState);

    /// Draw UI element: radiobox
    ///
    /// This is similar in appearance to a checkbox.
    fn radiobox(&mut self, rect: Rect, checked: bool, state: InputState);

    /// Draw UI element: scrollbar
    ///
    /// -   `rect`: area of whole widget (slider track)
    /// -   `h_rect`: area of slider handle
    /// -   `dir`: direction of bar
    /// -   `state`: highlighting information
    fn scrollbar(&mut self, rect: Rect, h_rect: Rect, dir: Direction, state: InputState);

    /// Draw UI element: slider
    ///
    /// -   `rect`: area of whole widget (slider track)
    /// -   `h_rect`: area of slider handle
    /// -   `dir`: direction of slider (currently only LTR or TTB)
    /// -   `state`: highlighting information
    fn slider(&mut self, rect: Rect, h_rect: Rect, dir: Direction, state: InputState);

    /// Draw UI element: progress bar
    ///
    /// -   `rect`: area of whole widget
    /// -   `dir`: direction of progress bar
    /// -   `state`: highlighting information
    /// -   `value`: progress value, between 0.0 and 1.0
    fn progress_bar(&mut self, rect: Rect, dir: Direction, state: InputState, value: f32);

    /// Draw an image
    fn image(&mut self, id: ImageId, rect: Rect);
}

macro_rules! impl_ {
    (($($args:tt)*) DrawHandle for $ty:ty) => {
        impl<$($args)*> DrawHandle for $ty {
            fn size_and_draw_shared(&mut self) -> (&dyn SizeHandle, &mut dyn DrawShared) {
                self.deref_mut().size_and_draw_shared()
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
            fn outer_frame(&mut self, rect: Rect) {
                self.deref_mut().outer_frame(rect);
            }
            fn separator(&mut self, rect: Rect) {
                self.deref_mut().separator(rect);
            }
            fn nav_frame(&mut self, rect: Rect, state: InputState) {
                self.deref_mut().nav_frame(rect, state);
            }
            fn selection_box(&mut self, rect: Rect) {
                self.deref_mut().selection_box(rect);
            }
            fn text(&mut self, pos: Coord, text: &TextDisplay, class: TextClass, state: InputState) {
                self.deref_mut().text(pos, text, class, state)
            }
            fn text_effects(
                &mut self,
                pos: Coord,
                text: &dyn TextApi,
                class: TextClass,
                state: InputState,
            ) {
                self.deref_mut().text_effects(pos, text, class, state);
            }
            fn text_accel(
                &mut self,
                pos: Coord,
                text: &Text<AccelString>,
                accel: bool,
                class: TextClass,
                state: InputState,
            ) {
                self.deref_mut().text_accel(pos, text, accel, class, state);
            }
            fn text_selected_range(
                &mut self,
                pos: Coord,
                text: &TextDisplay,
                range: Range<usize>,
                class: TextClass,
                state: InputState,
            ) {
                self.deref_mut()
                    .text_selected_range(pos, text, range, class, state);
            }
            fn text_cursor(&mut self, pos: Coord, text: &TextDisplay, class: TextClass, byte: usize) {
                self.deref_mut().text_cursor(pos, text, class, byte)
            }
            fn menu_entry(&mut self, rect: Rect, state: InputState) {
                self.deref_mut().menu_entry(rect, state)
            }
            fn button(&mut self, rect: Rect, col: Option<Rgb>, state: InputState) {
                self.deref_mut().button(rect, col, state)
            }
            fn edit_box(&mut self, rect: Rect, state: InputState) {
                self.deref_mut().edit_box(rect, state)
            }
            fn checkbox(&mut self, rect: Rect, checked: bool, state: InputState) {
                self.deref_mut().checkbox(rect, checked, state)
            }
            fn radiobox(&mut self, rect: Rect, checked: bool, state: InputState) {
                self.deref_mut().radiobox(rect, checked, state)
            }
            fn scrollbar(&mut self, rect: Rect, h_rect: Rect, dir: Direction, state: InputState) {
                self.deref_mut().scrollbar(rect, h_rect, dir, state)
            }
            fn slider(&mut self, rect: Rect, h_rect: Rect, dir: Direction, state: InputState) {
                self.deref_mut().slider(rect, h_rect, dir, state)
            }
            fn progress_bar(&mut self, rect: Rect, dir: Direction, state: InputState, value: f32) {
                self.deref_mut().progress_bar(rect, dir, state, value);
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

        let text = crate::text::Text::new_single("sample");
        let class = TextClass::Label;
        let state = InputState::empty();
        draw.text_selected(Coord::ZERO, &text, .., class, state)
    }
}
