// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! "Handle" types used by themes

use std::convert::AsRef;
use std::ops::{Bound, Deref, DerefMut, Range, RangeBounds};

use crate::dir::Direction;
use crate::draw::{color::Rgb, Draw, ImageId, PassType};
use crate::geom::{Coord, Offset, Rect};
use crate::text::{AccelString, Text, TextApi, TextDisplay};
use crate::theme::{InputState, SizeHandle, SizeMgr, TextClass};

/// A handle to the active theme, used for drawing
///
/// The shell provides widgets a `&dyn DrawHandle` in [`crate::Layout::draw`].
/// The extension trait [`DrawHandleExt`] provides some additional methods on
/// draw handles.
///
/// Most methods draw some feature. Exceptions:
///
/// -   [`Self::size_mgr`] provides access to a [`SizeMgr`]
/// -   [`Self::draw_device`] provides a lower-level interface for draw operations
/// -   [`Self::new_pass`], [`DrawHandleExt::with_clip_region`],
///     [`DrawHandleExt::with_overlay`] construct new draw passes
/// -   [`Self::get_clip_rect`] returns the clip rect
///
/// See also [`SizeMgr`].
pub trait DrawHandle {
    /// Access a [`SizeHandle`]
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    fn size_handle(&self) -> &dyn SizeHandle;

    /// Access a [`SizeMgr`]
    fn size_mgr(&self) -> SizeMgr {
        SizeMgr::new(self.size_handle())
    }

    /// Access the low-level draw device
    ///
    /// Note: this drawing API is modular, with limited functionality in the
    /// base trait [`Draw`]. To access further functionality, it is necessary
    /// to downcast with [`crate::draw::DrawIface::downcast_from`].
    fn draw_device(&mut self) -> &mut dyn Draw;

    /// Add a draw pass
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
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
    /// [clip region](DrawHandleExt::with_clip_region) or an
    /// [overlay](DrawHandleExt::with_overlay). This may be used to cull hidden
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

    /// Method used to implement [`DrawHandleExt::text_selected`]
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
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
    fn edit_marker(&mut self, pos: Coord, text: &TextDisplay, class: TextClass, byte: usize);

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

/// Extension trait over [`DrawHandle`]
///
/// Importing this trait allows use of additional methods (which cannot be
/// defined directly on [`DrawHandle`]).
pub trait DrawHandleExt: DrawHandle {
    /// Draw to a new pass with clipping and offset (e.g. for scrolling)
    ///
    /// Adds a new draw pass of type [`PassType::Clip`], with draw operations
    /// clipped to `rect` and translated by `offset.
    fn with_clip_region(
        &mut self,
        rect: Rect,
        offset: Offset,
        f: &mut dyn FnMut(&mut dyn DrawHandle),
    ) {
        self.new_pass(rect, offset, PassType::Clip, f);
    }

    /// Draw to a new pass as an overlay (e.g. for pop-up menus)
    ///
    /// Adds a new draw pass of type [`PassType::Overlay`], with draw operations
    /// clipped to `rect`.
    ///
    /// The theme is permitted to enlarge the `rect` for the purpose of drawing
    /// a frame or shadow around this overlay, thus the
    /// [`DrawHandle::get_clip_rect`] may be larger than expected.
    fn with_overlay(&mut self, rect: Rect, f: &mut dyn FnMut(&mut dyn DrawHandle)) {
        self.new_pass(rect, Offset::ZERO, PassType::Overlay, f);
    }

    /// Draw some text using the standard font, with a subset selected
    ///
    /// Other than visually highlighting the selection, this method behaves
    /// identically to [`DrawHandle::text`]. It is likely to be replaced in the
    /// future by a higher-level API.
    fn text_selected<T: AsRef<TextDisplay>, R: RangeBounds<usize>>(
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
        self.text_selected_range(pos, text.as_ref(), range, class, state);
    }
}

impl<D: DrawHandle + ?Sized> DrawHandleExt for D {}

macro_rules! impl_ {
    (($($args:tt)*) DrawHandle for $ty:ty) => {
        impl<$($args)*> DrawHandle for $ty {
            fn size_handle(&self) -> &dyn SizeHandle {
                self.deref().size_handle()
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
            fn edit_marker(&mut self, pos: Coord, text: &TextDisplay, class: TextClass, byte: usize) {
                self.deref_mut().edit_marker(pos, text, class, byte)
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

    fn _draw_handle_ext(draw: &mut dyn DrawHandle) {
        // We can't call this method without constructing an actual DrawHandle.
        // But we don't need to: we just want to test that methods are callable.

        let _scale = draw.size_mgr().scale_factor();

        let text = crate::text::Text::new_single("sample");
        let class = TextClass::Label;
        let state = InputState::empty();
        draw.text_selected(Coord::ZERO, &text, .., class, state)
    }
}
