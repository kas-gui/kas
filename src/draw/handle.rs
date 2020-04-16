// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! "Handle" types used by themes

use std::ops::{Deref, DerefMut};

use kas::draw::{Draw, Region};
use kas::geom::{Coord, Rect, Size};
use kas::layout::{AxisInfo, Margins, SizeRules};
use kas::{Align, Direction};

/// Input and highlighting state of a widget
///
/// Multiple instances can be combined via [`std::ops::BitOr`]: `lhs | rhs`.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct InputState {
    /// Is the widget disabled?
    pub disabled: bool,
    /// Is the input state erroneous?
    pub error: bool,
    /// "Hover" is true if the mouse is over this element
    pub hover: bool,
    /// Elements may be depressed during interaction
    ///
    /// Elements such as buttons, handles and menu entries may be depressed
    /// (visually pushed) by a click or touch event or an accelerator key.
    /// This is often visualised by a darker colour and/or by offsetting
    /// graphics. The `hover` state should be ignored when depressed.
    pub depress: bool,
    /// Keyboard navigation of UIs moves a "focus" from widget to widget.
    pub nav_focus: bool,
    /// "Character focus" implies this widget is ready to receive text input
    /// (e.g. typing into an input field).
    pub char_focus: bool,
}

impl std::ops::BitOr for InputState {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        InputState {
            disabled: self.disabled || rhs.disabled,
            error: self.error || rhs.error,
            hover: self.hover || rhs.hover,
            depress: self.depress || rhs.depress,
            nav_focus: self.nav_focus || rhs.nav_focus,
            char_focus: self.char_focus || rhs.char_focus,
        }
    }
}

/// Class of text drawn
///
/// Themes choose font, font size, colour, and alignment based on this.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum TextClass {
    /// Label text is drawn over the background colour
    Label,
    /// Button text is drawn over a button
    Button,
    /// Class of text drawn in a single-line edit box
    Edit,
    /// Class of text drawn in a multi-line edit box
    EditMulti,
}

/// Default class: Label
impl Default for TextClass {
    fn default() -> Self {
        TextClass::Label
    }
}

/// Handle passed to objects during draw and sizing operations
///
/// This handle is provided by the toolkit (usually via a theme implementation)
/// in order to provide sizing information of the elements drawn by
/// [`DrawHandle`].
pub trait SizeHandle {
    /// Get the scale (DPI) factor
    ///
    /// "Traditional" PC screens have a scale factor of 1; high-DPI screens
    /// may have a factor of 2 or higher; this may be fractional. It is
    /// recommended to calculate sizes as follows:
    /// ```
    /// # let scale_factor = 1.5f32;
    /// let size = (100.0 * scale_factor).round() as u32;
    /// ```
    ///
    /// This value may change during a program's execution (e.g. when a window
    /// is moved to a different monitor).
    fn scale_factor(&self) -> f32;

    /// Size of a frame around child widget(s)
    ///
    /// Returns dimensions of the frame on each side.
    fn frame(&self) -> Size;

    /// The margin around content within a widget
    ///
    /// This area may be used to draw focus indicators.
    fn inner_margin(&self) -> Size;

    /// The margin between UI elements, where desired
    fn outer_margins(&self) -> Margins;

    /// The height of a line of text
    fn line_height(&self, class: TextClass) -> u32;

    /// Get a text label size bound
    ///
    /// Sizing requirements of [`DrawHandle::text`].
    fn text_bound(&mut self, text: &str, class: TextClass, axis: AxisInfo) -> SizeRules;

    /// Size of the sides of a button.
    ///
    /// Returns `(top_left, bottom_right)` dimensions as two `Size`s.
    /// Excludes size of content area.
    fn button_surround(&self) -> (Size, Size);

    /// Size of the sides of an edit box.
    ///
    /// Returns `(top_left, bottom_right)` dimensions as two `Size`s.
    /// Excludes size of content area.
    fn edit_surround(&self) -> (Size, Size);

    /// Size of the element drawn by [`DrawHandle::checkbox`].
    fn checkbox(&self) -> Size;

    /// Size of the element drawn by [`DrawHandle::radiobox`].
    fn radiobox(&self) -> Size;

    /// Dimensions for a scrollbar
    ///
    /// Returns:
    ///
    /// -   `size`: minimum size of handle in horizontal orientation;
    ///     `size.1` is also the dimension of the scrollbar
    /// -   `min_len`: minimum length for the whole bar
    ///
    /// Required bound: `min_len >= size.0`.
    fn scrollbar(&self) -> (Size, u32);

    /// Dimensions for a slider
    ///
    /// Returns:
    ///
    /// -   `size`: minimum size of handle in horizontal orientation;
    ///     `size.1` is also the dimension of the slider
    /// -   `min_len`: minimum length for the whole bar
    ///
    /// Required bound: `min_len >= size.0`.
    fn slider(&self) -> (Size, u32);
}

/// Handle passed to objects during draw and sizing operations
///
/// This handle is provided by the toolkit (usually via a theme implementation)
/// as a high-level drawing interface. See also the companion trait,
/// [`SizeHandle`].
pub trait DrawHandle {
    /// Access the low-level draw device
    ///
    /// Returns `(region, offset, draw)`.
    ///
    /// One may use [`Draw::as_any_mut`] to downcast the `draw` object when necessary.
    ///
    /// **Important**: all positions ([`Rect`] and [`Coord`]) must be adjusted
    /// (as below) by the `given` offset before being passed to the methods of
    /// [`Draw`] and its extension traits. This offset is used by
    /// [`kas::widget::ScrollRegion`] to adjust its contents.
    /// ```
    /// # use kas::geom::*;
    /// # let offset = Coord::ZERO;
    /// # let rect = Rect::new(offset, Size::ZERO);
    /// let rect = rect + offset;
    /// ```
    fn draw_device(&mut self) -> (Region, Coord, &mut dyn Draw);

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
    /// If this instance of [`DrawHandle`] was created via
    /// [`DrawHandle::clip_region`], then this returns the `rect` passed to
    /// that method; otherwise this returns the window's `rect`.
    fn target_rect(&self) -> Rect;

    /// Draw a frame inside the given `rect`
    ///
    /// The frame dimensions equal those of [`SizeHandle::frame`] on each side.
    fn outer_frame(&mut self, rect: Rect);

    /// Draw a separator in the given `rect`
    fn separator(&mut self, rect: Rect);

    /// Draw some text using the standard font
    ///
    /// The dimensions required for this text may be queried with [`SizeHandle::text_bound`].
    fn text(&mut self, rect: Rect, text: &str, class: TextClass, align: (Align, Align));

    /// Draw button sides, background and margin-area highlight
    fn button(&mut self, rect: Rect, state: InputState);

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
}

impl<S: SizeHandle> SizeHandle for Box<S> {
    fn scale_factor(&self) -> f32 {
        self.deref().scale_factor()
    }

    fn frame(&self) -> Size {
        self.deref().frame()
    }
    fn inner_margin(&self) -> Size {
        self.deref().inner_margin()
    }
    fn outer_margins(&self) -> Margins {
        self.deref().outer_margins()
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
    fn scrollbar(&self) -> (Size, u32) {
        self.deref().scrollbar()
    }
    fn slider(&self) -> (Size, u32) {
        self.deref().slider()
    }
}

#[cfg(feature = "stack_dst")]
impl<S> SizeHandle for stack_dst::ValueA<dyn SizeHandle, S>
where
    S: Default + Copy + AsRef<[usize]> + AsMut<[usize]>,
{
    fn scale_factor(&self) -> f32 {
        self.deref().scale_factor()
    }

    fn frame(&self) -> Size {
        self.deref().frame()
    }
    fn inner_margin(&self) -> Size {
        self.deref().inner_margin()
    }
    fn outer_margins(&self) -> Margins {
        self.deref().outer_margins()
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
    fn scrollbar(&self) -> (Size, u32) {
        self.deref().scrollbar()
    }
    fn slider(&self) -> (Size, u32) {
        self.deref().slider()
    }
}

impl<H: DrawHandle> DrawHandle for Box<H> {
    fn draw_device(&mut self) -> (Region, Coord, &mut dyn Draw) {
        self.deref_mut().draw_device()
    }
    fn clip_region(&mut self, rect: Rect, offset: Coord, f: &mut dyn FnMut(&mut dyn DrawHandle)) {
        self.deref_mut().clip_region(rect, offset, f)
    }
    fn target_rect(&self) -> Rect {
        self.deref().target_rect()
    }
    fn outer_frame(&mut self, rect: Rect) {
        self.deref_mut().outer_frame(rect);
    }
    fn separator(&mut self, rect: Rect) {
        self.deref_mut().separator(rect);
    }
    fn text(&mut self, rect: Rect, text: &str, class: TextClass, align: (Align, Align)) {
        self.deref_mut().text(rect, text, class, align)
    }
    fn button(&mut self, rect: Rect, state: InputState) {
        self.deref_mut().button(rect, state)
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
}

#[cfg(feature = "stack_dst")]
impl<S> DrawHandle for stack_dst::ValueA<dyn DrawHandle, S>
where
    S: Default + Copy + AsRef<[usize]> + AsMut<[usize]>,
{
    fn draw_device(&mut self) -> (Region, Coord, &mut dyn Draw) {
        self.deref_mut().draw_device()
    }
    fn clip_region(&mut self, rect: Rect, offset: Coord, f: &mut dyn FnMut(&mut dyn DrawHandle)) {
        self.deref_mut().clip_region(rect, offset, f)
    }
    fn target_rect(&self) -> Rect {
        self.deref().target_rect()
    }
    fn outer_frame(&mut self, rect: Rect) {
        self.deref_mut().outer_frame(rect);
    }
    fn separator(&mut self, rect: Rect) {
        self.deref_mut().separator(rect);
    }
    fn text(&mut self, rect: Rect, text: &str, class: TextClass, align: (Align, Align)) {
        self.deref_mut().text(rect, text, class, align)
    }
    fn button(&mut self, rect: Rect, state: InputState) {
        self.deref_mut().button(rect, state)
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
}
