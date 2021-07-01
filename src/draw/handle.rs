// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! "Handle" types used by themes

use std::convert::AsRef;
use std::ops::{Bound, Deref, DerefMut, Range, RangeBounds};

use kas::dir::Direction;
use kas::draw::{Draw, DrawSharedT, Drawable, ImageId};
use kas::geom::{Coord, Offset, Rect, Size};
use kas::layout::{AxisInfo, FrameRules, Margins, SizeRules};
use kas::text::{AccelString, Text, TextApi, TextDisplay};

// for doc use
#[allow(unused)]
use kas::text::TextApiExt;

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
    /// "Selection focus" allows things such as text to be selected. Selection
    /// focus implies that the widget also has character focus.
    pub sel_focus: bool,
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
            sel_focus: self.sel_focus || rhs.sel_focus,
        }
    }
}

/// Class of text drawn
///
/// Themes choose font, font size, colour, and alignment based on this.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TextClass {
    /// Label text is drawn over the background colour
    Label,
    /// Scrollable label (same as label except that min height is limited)
    LabelScroll,
    /// Button text is drawn over a button
    Button,
    /// Class of text drawn in a single-line edit box
    Edit,
    /// Class of text drawn in a multi-line edit box
    EditMulti,
    /// Menu label (single line, does not stretch)
    MenuLabel,
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

/// Region class
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum RegionClass {
    ScrollRegion,
    Overlay,
}

/// Handle passed to objects during sizing operations
///
/// Themes must implement both [`SizeHandle`] and [`DrawHandle`].
///
/// The toolkit provides a `&dyn SizeHandle` value when resizing widgets. The
/// handle may also be accessed via [`kas::event::Manager::size_handle`].
pub trait SizeHandle {
    /// Access [`DrawSharedT`] trait object
    fn draw_shared(&mut self) -> &mut dyn DrawSharedT;

    /// Get the scale (DPI) factor
    ///
    /// "Traditional" PC screens have a scale factor of 1; high-DPI screens
    /// may have a factor of 2 or higher; this may be fractional. It is
    /// recommended to calculate sizes as follows:
    /// ```
    /// use kas::cast::*;
    /// # let scale_factor = 1.5f32;
    /// let size: i32 = (100.0 * scale_factor).cast_ceil();
    /// ```
    ///
    /// This value may change during a program's execution (e.g. when a window
    /// is moved to a different monitor); in this case all widgets will be
    /// resized via [`kas::Layout::size_rules`].
    fn scale_factor(&self) -> f32;

    /// Convert a size in virtual pixels to physical pixels
    fn pixels_from_virtual(&self, px: f32) -> f32 {
        px * self.scale_factor()
    }

    /// Convert a size in font Points to physical pixels
    fn pixels_from_points(&self, pt: f32) -> f32;

    /// Convert a size in font Em to physical pixels
    ///
    /// (This depends on the font size.)
    fn pixels_from_em(&self, em: f32) -> f32;

    /// Size of a frame around child widget(s)
    fn frame(&self, vert: bool) -> FrameRules;

    /// Frame/margin around a menu entry
    fn menu_frame(&self, vert: bool) -> FrameRules;

    /// Size of a separator frame between items
    fn separator(&self) -> Size;

    /// Size of a navigation highlight margin around a child widget
    fn nav_frame(&self, vert: bool) -> FrameRules;

    /// The margin around content within a widget
    ///
    /// Though inner margins are *usually* empty, they are sometimes drawn to,
    /// for example focus indicators.
    fn inner_margin(&self) -> Size;

    /// The margin between UI elements, where desired
    ///
    /// Widgets must not draw in outer margins.
    fn outer_margins(&self) -> Margins;

    /// The margin around text elements
    ///
    /// Similar to [`Self::outer_margins`], but intended for things like text
    /// labels which do not have a visible hard edge.
    fn text_margins(&self) -> Margins;

    /// The height of a line of text
    fn line_height(&self, class: TextClass) -> i32;

    /// Update a [`Text`] and get a size bound
    ///
    /// First, this method updates the text's [`Environment`]: `bounds`, `dpp`
    /// and `pt_size` are set. Second, the text is prepared (which is necessary
    /// to calculate size requirements). Finally, this converts the requirements
    /// to a [`SizeRules`] value and returns it.
    ///
    /// Usually this method is used in [`Layout::size_rules`], then
    /// [`TextApiExt::update_env`] is used in [`Layout::set_rect`].
    ///
    /// [`Environment`]: kas::text::Environment
    /// [`Layout::set_rect`]: kas::Layout::set_rect
    /// [`Layout::size_rules`]: kas::Layout::size_rules
    fn text_bound(&mut self, text: &mut dyn TextApi, class: TextClass, axis: AxisInfo)
        -> SizeRules;

    /// Width of an edit marker
    fn edit_marker_width(&self) -> f32;

    /// Size of the sides of a button.
    fn button_surround(&self, vert: bool) -> FrameRules;

    /// Size of the frame around an edit box, including margin
    ///
    /// Note: though text should not be drawn in the margin, the edit cursor
    /// may be. The margin included here should be large enough!
    fn edit_surround(&self, vert: bool) -> FrameRules;

    /// Size of the element drawn by [`DrawHandle::checkbox`].
    fn checkbox(&self) -> Size;

    /// Size of the element drawn by [`DrawHandle::radiobox`].
    fn radiobox(&self) -> Size;

    /// Dimensions for a scrollbar
    ///
    /// Returns:
    ///
    /// -   `size`: minimum size of handle in horizontal orientation;
    ///     `size.1` is also the width of the scrollbar
    /// -   `min_len`: minimum length for the whole bar
    ///
    /// Required bound: `min_len >= size.0`.
    fn scrollbar(&self) -> (Size, i32);

    /// Dimensions for a slider
    ///
    /// Returns:
    ///
    /// -   `size`: minimum size of handle in horizontal orientation;
    ///     `size.1` is also the width of the slider
    /// -   `min_len`: minimum length for the whole bar
    ///
    /// Required bound: `min_len >= size.0`.
    fn slider(&self) -> (Size, i32);

    /// Dimensions for a progress bar
    ///
    /// Returns the minimum size for a horizontal progress bar. It is assumed
    /// that the width is adjustable while the height is (preferably) not.
    /// For a vertical bar, the values are swapped.
    fn progress_bar(&self) -> Size;
}

/// Handle passed to objects during draw operations
///
/// Themes must implement both [`SizeHandle`] and [`DrawHandle`].
///
/// The toolkit provides a `&mut dyn DrawHandle` value when drawing widgets.
/// The extension trait [`DrawHandleExt`] provides some additional methods on
/// draw handles.
pub trait DrawHandle {
    /// Access a [`SizeHandle`] (object-safe version)
    ///
    /// Users may prefer to use [`DrawHandleExt::size_handle`] instead. If using
    /// this method directly, note that there is no guarantee that `f` gets
    /// called exactly once, or even that it gets called at all.
    ///
    /// Implementations *should* call the given function argument once.
    fn size_handle_dyn(&mut self, f: &mut dyn FnMut(&mut dyn SizeHandle));

    /// Access the low-level draw device
    ///
    /// Returns `(offset, draw, shared)`.
    ///
    /// All local coordinates must be adjusted by `offset` (i.e. `new_pos = pos + offset`).
    ///
    /// The `draw` object is over the [`Drawable`] interface which exposes only
    /// minimal functionality. [`Draw::downcast`] will likely be of use.
    fn draw_device(&mut self) -> (Offset, Draw<'_, dyn Drawable>, &mut dyn DrawSharedT);

    /// Add a clip region
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    fn with_clip_region(
        &mut self,
        rect: Rect,
        offset: Offset,
        class: RegionClass,
        f: &mut dyn FnMut(&mut dyn DrawHandle),
    );

    /// Target area for drawing
    ///
    /// Drawing is restricted to this [`Rect`], which may be the whole window, a
    /// [clip region](DrawHandleExt::clip_region) or an [overlay](DrawHandleExt::overlay).
    /// This may be used to cull hidden items from lists inside a scrollable view.
    fn target_rect(&self) -> Rect;

    /// Draw a frame inside the given `rect`
    ///
    /// The frame dimensions equal those of [`SizeHandle::frame`] on each side.
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

    /// Draw some text using the standard font
    ///
    /// The dimensions required for this text may be queried with [`SizeHandle::text_bound`].
    fn text(&mut self, pos: Coord, text: &TextDisplay, class: TextClass);

    /// Draw text with effects
    ///
    /// [`DrawHandle::text`] already supports *font* effects: bold,
    /// emphasis, text size. In addition, this method supports underline and
    /// strikethrough effects.
    fn text_effects(&mut self, pos: Coord, text: &dyn TextApi, class: TextClass);

    /// Draw an `AccelString` text
    ///
    /// The `text` is drawn within the rect from `pos` to `text.env().bounds`.
    ///
    /// The dimensions required for this text may be queried with [`SizeHandle::text_bound`].
    fn text_accel(&mut self, pos: Coord, text: &Text<AccelString>, state: bool, class: TextClass);

    /// Method used to implement [`DrawHandleExt::text_selected`]
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    fn text_selected_range(
        &mut self,
        pos: Coord,
        text: &TextDisplay,
        range: Range<usize>,
        class: TextClass,
    );

    /// Draw an edit marker at the given `byte` index on this `text`
    fn edit_marker(&mut self, pos: Coord, text: &TextDisplay, class: TextClass, byte: usize);

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

    /// Draw to a sub-region with offset (e.g. for scrolling)
    ///
    /// This new region uses coordinates relative to `offset` (i.e. coordinates
    /// are subtracted by `offset`).
    ///
    /// All content drawn by the new region is clipped to the intersection of
    /// `rect` and the current target ([`DrawHandle::target_rect`]).
    fn clip_region(&mut self, rect: Rect, offset: Offset, f: &mut dyn FnMut(&mut dyn DrawHandle)) {
        self.with_clip_region(rect, offset, RegionClass::ScrollRegion, f);
    }

    /// Draw to an overlay (e.g. for pop-up menus)
    ///
    /// This new region uses coordinates relative to `rect` (i.e. `Coord::ZERO`
    /// is the first pixel inside `rect`).
    ///
    /// All content drawn via this handle is clipped to the given `rect`.
    ///
    /// The new `rect` may extend beyond the current draw region. If it extends
    /// beyond the bounds of the window, it will be silently reduced to that of
    /// the window.
    fn overlay(&mut self, rect: Rect, f: &mut dyn FnMut(&mut dyn DrawHandle)) {
        self.with_clip_region(rect, Offset::ZERO, RegionClass::Overlay, f);
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
        self.text_selected_range(pos, text.as_ref(), range, class);
    }
}

impl<D: DrawHandle + ?Sized> DrawHandleExt for D {}

impl<S: SizeHandle> SizeHandle for Box<S> {
    fn draw_shared(&mut self) -> &mut dyn DrawSharedT {
        self.deref_mut().draw_shared()
    }

    fn scale_factor(&self) -> f32 {
        self.deref().scale_factor()
    }
    fn pixels_from_points(&self, pt: f32) -> f32 {
        self.deref().pixels_from_points(pt)
    }
    fn pixels_from_em(&self, em: f32) -> f32 {
        self.deref().pixels_from_em(em)
    }

    fn frame(&self, vert: bool) -> FrameRules {
        self.deref().frame(vert)
    }
    fn menu_frame(&self, vert: bool) -> FrameRules {
        self.deref().menu_frame(vert)
    }
    fn separator(&self) -> Size {
        self.deref().separator()
    }
    fn nav_frame(&self, vert: bool) -> FrameRules {
        self.deref().nav_frame(vert)
    }
    fn inner_margin(&self) -> Size {
        self.deref().inner_margin()
    }
    fn outer_margins(&self) -> Margins {
        self.deref().outer_margins()
    }
    fn text_margins(&self) -> Margins {
        self.deref().text_margins()
    }

    fn line_height(&self, class: TextClass) -> i32 {
        self.deref().line_height(class)
    }
    fn text_bound(
        &mut self,
        text: &mut dyn TextApi,
        class: TextClass,
        axis: AxisInfo,
    ) -> SizeRules {
        self.deref_mut().text_bound(text, class, axis)
    }
    fn edit_marker_width(&self) -> f32 {
        self.deref().edit_marker_width()
    }

    fn button_surround(&self, vert: bool) -> FrameRules {
        self.deref().button_surround(vert)
    }
    fn edit_surround(&self, vert: bool) -> FrameRules {
        self.deref().edit_surround(vert)
    }

    fn checkbox(&self) -> Size {
        self.deref().checkbox()
    }
    fn radiobox(&self) -> Size {
        self.deref().radiobox()
    }
    fn scrollbar(&self) -> (Size, i32) {
        self.deref().scrollbar()
    }
    fn slider(&self) -> (Size, i32) {
        self.deref().slider()
    }
    fn progress_bar(&self) -> Size {
        self.deref().progress_bar()
    }
}

#[cfg(feature = "stack_dst")]
impl<'a, S> SizeHandle for stack_dst::ValueA<dyn SizeHandle + 'a, S>
where
    S: Default + Copy + AsRef<[usize]> + AsMut<[usize]>,
{
    fn draw_shared(&mut self) -> &mut dyn DrawSharedT {
        self.deref_mut().draw_shared()
    }

    fn scale_factor(&self) -> f32 {
        self.deref().scale_factor()
    }
    fn pixels_from_points(&self, pt: f32) -> f32 {
        self.deref().pixels_from_points(pt)
    }
    fn pixels_from_em(&self, em: f32) -> f32 {
        self.deref().pixels_from_em(em)
    }

    fn frame(&self, vert: bool) -> FrameRules {
        self.deref().frame(vert)
    }
    fn menu_frame(&self, vert: bool) -> FrameRules {
        self.deref().menu_frame(vert)
    }
    fn separator(&self) -> Size {
        self.deref().separator()
    }
    fn nav_frame(&self, vert: bool) -> FrameRules {
        self.deref().nav_frame(vert)
    }
    fn inner_margin(&self) -> Size {
        self.deref().inner_margin()
    }
    fn outer_margins(&self) -> Margins {
        self.deref().outer_margins()
    }
    fn text_margins(&self) -> Margins {
        self.deref().text_margins()
    }

    fn line_height(&self, class: TextClass) -> i32 {
        self.deref().line_height(class)
    }
    fn text_bound(
        &mut self,
        text: &mut dyn TextApi,
        class: TextClass,
        axis: AxisInfo,
    ) -> SizeRules {
        self.deref_mut().text_bound(text, class, axis)
    }
    fn edit_marker_width(&self) -> f32 {
        self.deref().edit_marker_width()
    }

    fn button_surround(&self, vert: bool) -> FrameRules {
        self.deref().button_surround(vert)
    }
    fn edit_surround(&self, vert: bool) -> FrameRules {
        self.deref().edit_surround(vert)
    }

    fn checkbox(&self) -> Size {
        self.deref().checkbox()
    }
    fn radiobox(&self) -> Size {
        self.deref().radiobox()
    }
    fn scrollbar(&self) -> (Size, i32) {
        self.deref().scrollbar()
    }
    fn slider(&self) -> (Size, i32) {
        self.deref().slider()
    }
    fn progress_bar(&self) -> Size {
        self.deref().progress_bar()
    }
}

impl<H: DrawHandle> DrawHandle for Box<H> {
    fn size_handle_dyn(&mut self, f: &mut dyn FnMut(&mut dyn SizeHandle)) {
        self.deref_mut().size_handle_dyn(f)
    }
    fn draw_device(&mut self) -> (Offset, Draw<'_, dyn Drawable>, &mut dyn DrawSharedT) {
        self.deref_mut().draw_device()
    }
    fn with_clip_region(
        &mut self,
        rect: Rect,
        offset: Offset,
        class: RegionClass,
        f: &mut dyn FnMut(&mut dyn DrawHandle),
    ) {
        self.deref_mut().with_clip_region(rect, offset, class, f);
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
    fn nav_frame(&mut self, rect: Rect, state: InputState) {
        self.deref_mut().nav_frame(rect, state);
    }
    fn selection_box(&mut self, rect: Rect) {
        self.deref_mut().selection_box(rect);
    }
    fn text(&mut self, pos: Coord, text: &TextDisplay, class: TextClass) {
        self.deref_mut().text(pos, text, class)
    }
    fn text_effects(&mut self, pos: Coord, text: &dyn TextApi, class: TextClass) {
        self.deref_mut().text_effects(pos, text, class);
    }
    fn text_accel(&mut self, pos: Coord, text: &Text<AccelString>, state: bool, class: TextClass) {
        self.deref_mut().text_accel(pos, text, state, class);
    }
    fn text_selected_range(
        &mut self,
        pos: Coord,
        text: &TextDisplay,
        range: Range<usize>,
        class: TextClass,
    ) {
        self.deref_mut()
            .text_selected_range(pos, text, range, class);
    }
    fn edit_marker(&mut self, pos: Coord, text: &TextDisplay, class: TextClass, byte: usize) {
        self.deref_mut().edit_marker(pos, text, class, byte)
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
    fn progress_bar(&mut self, rect: Rect, dir: Direction, state: InputState, value: f32) {
        self.deref_mut().progress_bar(rect, dir, state, value);
    }
    fn image(&mut self, id: ImageId, rect: Rect) {
        self.deref_mut().image(id, rect);
    }
}

#[cfg(feature = "stack_dst")]
impl<'a, S> DrawHandle for stack_dst::ValueA<dyn DrawHandle + 'a, S>
where
    S: Default + Copy + AsRef<[usize]> + AsMut<[usize]>,
{
    fn size_handle_dyn(&mut self, f: &mut dyn FnMut(&mut dyn SizeHandle)) {
        self.deref_mut().size_handle_dyn(f)
    }
    fn draw_device(&'_ mut self) -> (Offset, Draw<'_, dyn Drawable>, &'_ mut dyn DrawSharedT) {
        self.deref_mut().draw_device()
    }
    fn with_clip_region(
        &mut self,
        rect: Rect,
        offset: Offset,
        class: RegionClass,
        f: &mut dyn FnMut(&mut dyn DrawHandle),
    ) {
        self.deref_mut().with_clip_region(rect, offset, class, f);
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
    fn nav_frame(&mut self, rect: Rect, state: InputState) {
        self.deref_mut().nav_frame(rect, state);
    }
    fn selection_box(&mut self, rect: Rect) {
        self.deref_mut().selection_box(rect);
    }
    fn text(&mut self, pos: Coord, text: &TextDisplay, class: TextClass) {
        self.deref_mut().text(pos, text, class)
    }
    fn text_effects(&mut self, pos: Coord, text: &dyn TextApi, class: TextClass) {
        self.deref_mut().text_effects(pos, text, class);
    }
    fn text_accel(&mut self, pos: Coord, text: &Text<AccelString>, state: bool, class: TextClass) {
        self.deref_mut().text_accel(pos, text, state, class);
    }
    fn text_selected_range(
        &mut self,
        pos: Coord,
        text: &TextDisplay,
        range: Range<usize>,
        class: TextClass,
    ) {
        self.deref_mut()
            .text_selected_range(pos, text, range, class);
    }
    fn edit_marker(&mut self, pos: Coord, text: &TextDisplay, class: TextClass, byte: usize) {
        self.deref_mut().edit_marker(pos, text, class, byte)
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
    fn progress_bar(&mut self, rect: Rect, dir: Direction, state: InputState, value: f32) {
        self.deref_mut().progress_bar(rect, dir, state, value);
    }
    fn image(&mut self, id: ImageId, rect: Rect) {
        self.deref_mut().image(id, rect);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn _draw_handle_ext(draw_handle: &mut dyn DrawHandle) {
        // We can't call this method without constructing an actual DrawHandle.
        // But we don't need to: we just want to test that methods are callable.

        let _scale = draw_handle.size_handle(|h| h.scale_factor());

        let text = kas::text::Text::new_single("sample");
        let class = TextClass::Label;
        draw_handle.text_selected(Coord::ZERO, &text, .., class)
    }
}
