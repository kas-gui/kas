// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! "Handle" types used by themes

use std::ops::{Deref, DerefMut};

use super::{TextClass, TextProperties};
use kas::event::HighlightState;
use kas::geom::{Coord, Rect, Size};
use kas::layout::{AxisInfo, SizeRules};
use kas::Direction;
#[cfg(feature = "stack_dst")]
use kas::StackDst;

/// Handle passed to objects during draw and sizing operations
pub trait SizeHandle {
    /// Size of a frame around child widget(s)
    ///
    /// Returns `(top_left, bottom_right)` dimensions as two `Size`s.
    fn outer_frame(&self) -> (Size, Size);

    /// The margin around content within a widget
    ///
    /// This area may be used to draw focus indicators.
    fn inner_margin(&self) -> Size;

    /// The margin between UI elements, where desired
    fn outer_margin(&self) -> Size;

    /// The height of a line of text
    fn line_height(&self, class: TextClass) -> u32;

    /// Get a text label size bound
    ///
    /// Sizing requirements of [`DrawHandle::text`].
    ///
    /// Since only a subset of [`TextProperties`] fields are required, these are
    /// passed directly.
    fn text_bound(&mut self, text: &str, class: TextClass, axis: AxisInfo) -> SizeRules;

    /// Size of the sides of a button.
    ///
    /// Includes each side (as in `outer_frame`), minus the content area (to be added separately).
    fn button_surround(&self) -> (Size, Size);

    /// Size of the sides of an edit box.
    ///
    /// Includes each side (as in `outer_frame`), minus the content area (to be added separately).
    fn edit_surround(&self) -> (Size, Size);

    /// Size of the element drawn by [`DrawHandle::checkbox`].
    fn checkbox(&self) -> Size;

    /// Size of the element drawn by [`DrawHandle::radiobox`].
    fn radiobox(&self) -> Size;

    /// Dimensions for a scrollbar
    ///
    /// Returns three components:
    ///
    /// -   `thickness`: scroll-bar width (for vertical scroll bars)
    /// -   `min_handle_len`: minimum length for the handle
    /// -   `min_len`: minimum length for the whole bar
    ///
    /// Generally, one expects `min_len` is significantly greater than
    /// `min_handle_len` (so that some movement is always possible).
    /// It is required that `min_len >= min_handle_len`.
    fn scrollbar(&self) -> (u32, u32, u32);
}

/// Handle passed to objects during draw and sizing operations
pub trait DrawHandle {
    /// Construct a new draw-handle on a given region and pass to a callback.
    ///
    /// This new region uses coordinates relative to `offset` (i.e. coordinates
    /// are subtracted by `offset`).
    ///
    /// All content drawn by the new region is clipped to the given `rect`
    /// (in the current coordinate space, i.e. not translated by `offset`).
    fn clip_region(&mut self, rect: Rect, offset: Coord, f: &mut dyn FnMut(&mut dyn DrawHandle));

    /// Target area for drawing
    ///
    /// This is the `Rect` passed to [`Theme::draw_handle`] or
    /// [`DrawHandle::clip_region`], minus any offsets.
    ///
    /// [`Theme::draw_handle`]: super::Theme::draw_handle
    fn target_rect(&self) -> Rect;

    /// Draw a frame in the given [`Rect`]
    ///
    /// The frame dimensions should equal those of [`SizeHandle::outer_frame`].
    fn outer_frame(&mut self, rect: Rect);

    /// Draw some text using the standard font
    ///
    /// The dimensions required for this text may be queried with [`SizeHandle::text_bound`].
    fn text(&mut self, rect: Rect, text: &str, props: TextProperties);

    /// Draw button sides, background and margin-area highlight
    fn button(&mut self, rect: Rect, highlights: HighlightState);

    /// Draw edit box sides, background and margin-area highlight
    fn edit_box(&mut self, rect: Rect, highlights: HighlightState);

    /// Draw UI element: checkbox
    ///
    /// The checkbox is a small, usually square, box with or without a check
    /// mark. A checkbox widget may include a text label, but that label is not
    /// part of this element.
    fn checkbox(&mut self, rect: Rect, checked: bool, highlights: HighlightState);

    /// Draw UI element: radiobox
    ///
    /// This is similar in appearance to a checkbox.
    fn radiobox(&mut self, rect: Rect, checked: bool, highlights: HighlightState);

    /// Draw UI element: scrollbar
    ///
    /// -   `rect`: area of whole widget (slider track)
    /// -   `h_rect`: area of slider handle
    /// -   `dir`: direction of bar
    /// -   `highlights`: highlighting information
    fn scrollbar(&mut self, rect: Rect, h_rect: Rect, dir: Direction, highlights: HighlightState);
}

impl<S: SizeHandle> SizeHandle for Box<S> {
    fn outer_frame(&self) -> (Size, Size) {
        self.deref().outer_frame()
    }
    fn inner_margin(&self) -> Size {
        self.deref().inner_margin()
    }
    fn outer_margin(&self) -> Size {
        self.deref().outer_margin()
    }

    fn line_height(&self, class: TextClass) -> u32 {
        self.deref().line_height(class)
    }
    fn text_bound(&mut self, text: &str, class: TextClass, axis: AxisInfo) -> SizeRules {
        self.deref_mut().text_bound(text, class, axis)
    }

    fn button_surround(&self) -> (Size, Size) {
        self.deref().button_surround()
    }
    fn edit_surround(&self) -> (Size, Size) {
        self.deref().edit_surround()
    }

    fn checkbox(&self) -> Size {
        self.deref().checkbox()
    }
    fn radiobox(&self) -> Size {
        self.deref().radiobox()
    }
    fn scrollbar(&self) -> (u32, u32, u32) {
        self.deref().scrollbar()
    }
}

#[cfg(feature = "stack_dst")]
impl SizeHandle for StackDst<dyn SizeHandle> {
    fn outer_frame(&self) -> (Size, Size) {
        self.deref().outer_frame()
    }
    fn inner_margin(&self) -> Size {
        self.deref().inner_margin()
    }
    fn outer_margin(&self) -> Size {
        self.deref().outer_margin()
    }

    fn line_height(&self, class: TextClass) -> u32 {
        self.deref().line_height(class)
    }
    fn text_bound(&mut self, text: &str, class: TextClass, axis: AxisInfo) -> SizeRules {
        self.deref_mut().text_bound(text, class, axis)
    }

    fn button_surround(&self) -> (Size, Size) {
        self.deref().button_surround()
    }
    fn edit_surround(&self) -> (Size, Size) {
        self.deref().edit_surround()
    }

    fn checkbox(&self) -> Size {
        self.deref().checkbox()
    }
    fn radiobox(&self) -> Size {
        self.deref().radiobox()
    }
    fn scrollbar(&self) -> (u32, u32, u32) {
        self.deref().scrollbar()
    }
}

impl<H: DrawHandle> DrawHandle for Box<H> {
    fn clip_region(&mut self, rect: Rect, offset: Coord, f: &mut dyn FnMut(&mut dyn DrawHandle)) {
        self.deref_mut().clip_region(rect, offset, f)
    }
    fn target_rect(&self) -> Rect {
        self.deref().target_rect()
    }
    fn outer_frame(&mut self, rect: Rect) {
        self.deref_mut().outer_frame(rect)
    }
    fn text(&mut self, rect: Rect, text: &str, props: TextProperties) {
        self.deref_mut().text(rect, text, props)
    }
    fn button(&mut self, rect: Rect, highlights: HighlightState) {
        self.deref_mut().button(rect, highlights)
    }
    fn edit_box(&mut self, rect: Rect, highlights: HighlightState) {
        self.deref_mut().edit_box(rect, highlights)
    }
    fn checkbox(&mut self, rect: Rect, checked: bool, highlights: HighlightState) {
        self.deref_mut().checkbox(rect, checked, highlights)
    }
    fn radiobox(&mut self, rect: Rect, checked: bool, highlights: HighlightState) {
        self.deref_mut().radiobox(rect, checked, highlights)
    }
    fn scrollbar(&mut self, rect: Rect, h_rect: Rect, dir: Direction, highlights: HighlightState) {
        self.deref_mut().scrollbar(rect, h_rect, dir, highlights)
    }
}

#[cfg(feature = "stack_dst")]
impl DrawHandle for StackDst<dyn DrawHandle> {
    fn clip_region(&mut self, rect: Rect, offset: Coord, f: &mut dyn FnMut(&mut dyn DrawHandle)) {
        self.deref_mut().clip_region(rect, offset, f)
    }
    fn target_rect(&self) -> Rect {
        self.deref().target_rect()
    }
    fn outer_frame(&mut self, rect: Rect) {
        self.deref_mut().outer_frame(rect)
    }
    fn text(&mut self, rect: Rect, text: &str, props: TextProperties) {
        self.deref_mut().text(rect, text, props)
    }
    fn button(&mut self, rect: Rect, highlights: HighlightState) {
        self.deref_mut().button(rect, highlights)
    }
    fn edit_box(&mut self, rect: Rect, highlights: HighlightState) {
        self.deref_mut().edit_box(rect, highlights)
    }
    fn checkbox(&mut self, rect: Rect, checked: bool, highlights: HighlightState) {
        self.deref_mut().checkbox(rect, checked, highlights)
    }
    fn radiobox(&mut self, rect: Rect, checked: bool, highlights: HighlightState) {
        self.deref_mut().radiobox(rect, checked, highlights)
    }
    fn scrollbar(&mut self, rect: Rect, h_rect: Rect, dir: Direction, highlights: HighlightState) {
        self.deref_mut().scrollbar(rect, h_rect, dir, highlights)
    }
}
