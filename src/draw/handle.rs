// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! "Handle" types used by themes

use std::ops::{Bound, Deref, DerefMut, Range, RangeBounds};

use kas::draw::{Draw, Pass};
use kas::geom::{Coord, Rect, Size};
use kas::layout::{AxisInfo, Margins, SizeRules};
use kas::text::PreparedText;
use kas::Direction;

/// Classification of a clip region
pub enum ClipRegion {
    Popup,
    Scroll,
}

/// Input and highlighting state of a widget
///
/// This struct is used to adjust the appearance of [`DrawHandle`]'s primitives.
///
/// Multiple instances can be combined via [`std::ops::BitOr`]: `lhs | rhs`.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct InputState {
    /// Disabled widgets are not responsive to input and usually drawn in grey.
    ///
    /// All other states should be ignored when disabled.
    pub disabled: bool,
    /// Some widgets, such as `EditBox`, use a red background on error
    pub error: bool,
    /// "Hover" is true if the mouse is over this element
    pub hover: bool,
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
    /// Single-line label which does not want to stretch vertically
    LabelSingle,
    /// Button text is drawn over a button
    Button,
    /// Class of text drawn in a single-line edit box
    Edit,
    /// Class of text drawn in a multi-line edit box
    EditMulti,
}

impl TextClass {
    /// True if text should be automatically line-wrapped
    pub fn line_wrap(self) -> bool {
        self == TextClass::Label || self == TextClass::EditMulti
    }
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
    /// is moved to a different monitor); in this case all widgets will be
    /// resized via [`kas::Layout::size_rules`].
    fn scale_factor(&self) -> f32;

    /// Size of a frame around child widget(s)
    ///
    /// Returns dimensions of the frame on each side.
    fn frame(&self) -> Size;

    /// Menu frame
    ///
    /// Menu items have a larger-than-usual margin / invisible frame around
    /// them. This should be drawn with [`DrawHandle::menu_frame`],
    /// though likely the theme will only draw when highlighted.
    ///
    /// Like [`SizeHandle::frame`] this method returns the frame on each side.
    fn menu_frame(&self) -> Size;

    /// The margin around content within a widget
    ///
    /// This area may be used to draw focus indicators.
    fn inner_margin(&self) -> Size;

    /// The margin between UI elements, where desired
    fn outer_margins(&self) -> Margins;

    /// The height of a line of text
    fn line_height(&self, class: TextClass) -> u32;

    /// Update a [`PreparedText`] and get a size bound
    ///
    /// First, this method updates the text's [`Environment`]: `bounds`, `dpp`
    /// and `pt_size` are set. Second, the text is prepared (which is necessary
    /// to calculate size requirements). Finally, this converts the requirements
    /// to a [`SizeRules`] value and returns it.
    ///
    /// Usually this method is used in [`Layout::size_rules`], then
    /// [`PreparedText::update_env`] is used in [`Layout::set_rect`].
    ///
    /// [`Environment`]: kas::text::Environment
    /// [`Layout::set_rect`]: kas::Layout::set_rect
    /// [`Layout::size_rules`]: kas::Layout::size_rules
    fn text_bound(
        &mut self,
        text: &mut PreparedText,
        class: TextClass,
        axis: AxisInfo,
    ) -> SizeRules;

    /// Width of an edit marker
    fn edit_marker_width(&self) -> f32;

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
/// as a high-level drawing interface. See also the extension trait,
/// [`DrawHandleExt`], and the companion trait, [`SizeHandle`].
pub trait DrawHandle {
    /// Access a [`SizeHandle`] (object-safe version)
    ///
    /// Users may prefer to use [`DrawHandleExt::size_handle`] instead. If using
    /// this method directly, note that there is no guarantee that `f` gets
    /// called exactly once, or even that it gets called at all.
    ///
    /// Implementations *should* call the given function argument once.
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    fn size_handle_dyn(&mut self, f: &mut dyn FnMut(&mut dyn SizeHandle));

    /// Access the low-level draw device
    ///
    /// Returns `(pass, offset, draw)`.
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
    fn draw_device(&mut self) -> (Pass, Coord, &mut dyn Draw);

    /// Construct a new draw-handle on a given region and pass to a callback.
    ///
    /// This new region uses coordinates relative to `offset` (i.e. coordinates
    /// are subtracted by `offset`).
    ///
    /// All content drawn by the new region is clipped to the given `rect`
    /// (in the current coordinate space, i.e. not translated by `offset`).
    fn clip_region(
        &mut self,
        rect: Rect,
        offset: Coord,
        class: ClipRegion,
        f: &mut dyn FnMut(&mut dyn DrawHandle),
    );

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

    /// Draw a menu frame and background inside the given `rect`
    ///
    /// The frame dimensions equal those of [`SizeHandle::frame`] on each side.
    fn menu_frame(&mut self, rect: Rect);

    /// Draw a separator in the given `rect`
    fn separator(&mut self, rect: Rect);

    /// Draw some text using the standard font
    ///
    /// The `text` is drawn within the rect from `pos` to `text.env().bounds`,
    /// but offset by subtracting `offset` (allowing scrolling).
    ///
    /// The dimensions required for this text may be queried with [`SizeHandle::text_bound`].
    fn text_offset(&mut self, pos: Coord, offset: Coord, text: &PreparedText, class: TextClass);

    /// Draw some text, with an underlined glyph
    ///
    /// This is identical to [`DrawHandle::text_offset`] except that the glyph
    /// starting at the given index is underlined.
    fn text_with_underline(
        &mut self,
        pos: Coord,
        offset: Coord,
        text: &PreparedText,
        class: TextClass,
        underline: usize,
    );

    /// Method used to implement [`DrawHandleExt::text_selected`]
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    fn text_selected_range(
        &mut self,
        pos: Coord,
        offset: Coord,
        text: &PreparedText,
        range: Range<usize>,
        class: TextClass,
    );

    /// Draw an edit marker at the given `byte` index on this `text`
    fn edit_marker(
        &mut self,
        pos: Coord,
        offset: Coord,
        text: &PreparedText,
        class: TextClass,
        byte: usize,
    );

    /// Draw the background of a menu entry
    fn menu_entry(&mut self, rect: Rect, state: InputState);

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

/// Extension trait over [`DrawHandle`]
///
/// Importing this trait allows use of additional methods (which cannot be
/// defined directly on [`DrawHandle`]).
pub trait DrawHandleExt: DrawHandle {
    /// Access a [`SizeHandle`]
    ///
    /// The given closure is called with a reference to a [`SizeHandle`], and
    /// the closure's result is returned.
    ///
    /// This method will panic if the implementation fails to call the closure.
    fn size_handle<F: Fn(&mut dyn SizeHandle) -> T, T>(&mut self, f: F) -> T {
        let mut result = None;
        self.size_handle_dyn(&mut |size_handle| {
            result = Some(f(size_handle));
        });
        result.expect("DrawHandle::size_handle_dyn impl failed to call function argument")
    }

    /// Draw some text using the standard font
    ///
    /// The `text` is drawn within the rect from `pos` to `text.env().bounds`.
    ///
    /// The dimensions required for this text may be queried with [`SizeHandle::text_bound`].
    fn text(&mut self, pos: Coord, text: &PreparedText, class: TextClass) {
        self.text_offset(pos, Coord::ZERO, text, class);
    }

    /// Draw some text using the standard font, with a subset selected
    ///
    /// Other than visually highlighting the selection, this method behaves
    /// identically to [`DrawHandleExt::text`]. It is likely to be replaced in the
    /// future by a higher-level API.
    fn text_selected<R: RangeBounds<usize>>(
        &mut self,
        pos: Coord,
        offset: Coord,
        text: &PreparedText,
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
            Bound::Unbounded => text.text_len(),
        };
        let range = Range { start, end };
        self.text_selected_range(pos, offset, text, range, class);
    }
}

impl<D: DrawHandle + ?Sized> DrawHandleExt for D {}

impl<S: SizeHandle> SizeHandle for Box<S> {
    fn scale_factor(&self) -> f32 {
        self.deref().scale_factor()
    }

    fn frame(&self) -> Size {
        self.deref().frame()
    }
    fn menu_frame(&self) -> Size {
        self.deref().menu_frame()
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
    fn text_bound(
        &mut self,
        text: &mut PreparedText,
        class: TextClass,
        axis: AxisInfo,
    ) -> SizeRules {
        self.deref_mut().text_bound(text, class, axis)
    }
    fn edit_marker_width(&self) -> f32 {
        self.deref().edit_marker_width()
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
    fn menu_frame(&self) -> Size {
        self.deref().menu_frame()
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
    fn text_bound(
        &mut self,
        text: &mut PreparedText,
        class: TextClass,
        axis: AxisInfo,
    ) -> SizeRules {
        self.deref_mut().text_bound(text, class, axis)
    }
    fn edit_marker_width(&self) -> f32 {
        self.deref().edit_marker_width()
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
    fn size_handle_dyn(&mut self, f: &mut dyn FnMut(&mut dyn SizeHandle)) {
        self.deref_mut().size_handle_dyn(f)
    }
    fn draw_device(&mut self) -> (Pass, Coord, &mut dyn Draw) {
        self.deref_mut().draw_device()
    }
    fn clip_region(
        &mut self,
        rect: Rect,
        offset: Coord,
        class: ClipRegion,
        f: &mut dyn FnMut(&mut dyn DrawHandle),
    ) {
        self.deref_mut().clip_region(rect, offset, class, f)
    }
    fn target_rect(&self) -> Rect {
        self.deref().target_rect()
    }
    fn outer_frame(&mut self, rect: Rect) {
        self.deref_mut().outer_frame(rect);
    }
    fn menu_frame(&mut self, rect: Rect) {
        self.deref_mut().menu_frame(rect);
    }
    fn separator(&mut self, rect: Rect) {
        self.deref_mut().separator(rect);
    }
    fn text_offset(&mut self, pos: Coord, offset: Coord, text: &PreparedText, class: TextClass) {
        self.deref_mut().text_offset(pos, offset, text, class)
    }
    fn text_with_underline(
        &mut self,
        pos: Coord,
        offset: Coord,
        text: &PreparedText,
        class: TextClass,
        underline: usize,
    ) {
        self.deref_mut()
            .text_with_underline(pos, offset, text, class, underline)
    }
    fn text_selected_range(
        &mut self,
        pos: Coord,
        offset: Coord,
        text: &PreparedText,
        range: Range<usize>,
        class: TextClass,
    ) {
        self.deref_mut()
            .text_selected_range(pos, offset, text, range, class);
    }
    fn edit_marker(
        &mut self,
        pos: Coord,
        offset: Coord,
        text: &PreparedText,
        class: TextClass,
        byte: usize,
    ) {
        self.deref_mut().edit_marker(pos, offset, text, class, byte)
    }
    fn menu_entry(&mut self, rect: Rect, state: InputState) {
        self.deref_mut().menu_entry(rect, state)
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
    fn size_handle_dyn(&mut self, f: &mut dyn FnMut(&mut dyn SizeHandle)) {
        self.deref_mut().size_handle_dyn(f)
    }
    fn draw_device(&mut self) -> (Pass, Coord, &mut dyn Draw) {
        self.deref_mut().draw_device()
    }
    fn clip_region(
        &mut self,
        rect: Rect,
        offset: Coord,
        class: ClipRegion,
        f: &mut dyn FnMut(&mut dyn DrawHandle),
    ) {
        self.deref_mut().clip_region(rect, offset, class, f)
    }
    fn target_rect(&self) -> Rect {
        self.deref().target_rect()
    }
    fn outer_frame(&mut self, rect: Rect) {
        self.deref_mut().outer_frame(rect);
    }
    fn menu_frame(&mut self, rect: Rect) {
        self.deref_mut().menu_frame(rect);
    }
    fn separator(&mut self, rect: Rect) {
        self.deref_mut().separator(rect);
    }
    fn text_offset(&mut self, pos: Coord, offset: Coord, text: &PreparedText, class: TextClass) {
        self.deref_mut().text_offset(pos, offset, text, class)
    }
    fn text_with_underline(
        &mut self,
        pos: Coord,
        offset: Coord,
        text: &PreparedText,
        class: TextClass,
        underline: usize,
    ) {
        self.deref_mut()
            .text_with_underline(pos, offset, text, class, underline)
    }
    fn text_selected_range(
        &mut self,
        pos: Coord,
        offset: Coord,
        text: &PreparedText,
        range: Range<usize>,
        class: TextClass,
    ) {
        self.deref_mut()
            .text_selected_range(pos, offset, text, range, class);
    }
    fn edit_marker(
        &mut self,
        pos: Coord,
        offset: Coord,
        text: &PreparedText,
        class: TextClass,
        byte: usize,
    ) {
        self.deref_mut().edit_marker(pos, offset, text, class, byte)
    }
    fn menu_entry(&mut self, rect: Rect, state: InputState) {
        self.deref_mut().menu_entry(rect, state)
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

#[cfg(test)]
mod test {
    use super::*;

    fn _draw_handle_ext(draw_handle: &mut dyn DrawHandle) {
        // We can't call this method without constructing an actual DrawHandle.
        // But we don't need to: we just want to test that methods are callable.

        let _size = draw_handle.size_handle(|h| h.frame());

        let zero = Coord::ZERO;
        let text = PreparedText::new_single("sample".into());
        draw_handle.text_selected(zero, zero, &text, .., TextClass::Label)
    }
}
