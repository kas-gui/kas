// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! High-level drawing interface
//!
//! A [`Theme`] provides a high-level drawing interface. It may be provided by
//! the toolkit or separately (but dependent on a toolkit's drawing API).
//!
//! A theme is implemented in multiple parts: the [`Theme`] object is shared
//! by all windows and may provide shared resources (e.g. fonts and textures).
//! It is also responsible for creating a per-window [`ThemeWindow`] object and
//! draw handles ([`DrawHandle`]).

use std::any::Any;

use rusttype::Font;

use kas::class::Align;
use kas::draw::Colour;
use kas::event::HighlightState;
use kas::geom::{Coord, Rect, Size};
use kas::layout::{AxisInfo, SizeRules};

/// Class of text drawn
pub enum TextClass {
    /// Label text is drawn over the background colour
    Label,
    /// Button text is drawn over a button
    Button,
    /// Class of text drawn in an edit (entry) box
    Edit,
}

/// Text alignment, class, etc.
pub struct TextProperties {
    /// Class of text
    pub class: TextClass,
    /// Does this use line breaks?
    pub multi_line: bool,
    /// Horizontal alignment
    pub horiz: Align,
    /// Vertical alignment
    pub vert: Align,
    // Note: do we want to add HighlightState?
}

/// A *theme* provides widget sizing and drawing implementations.
///
/// The theme is generic over some `Draw` type.
///
/// Objects of this type are copied within each window's data structure. For
/// large resources (e.g. fonts and icons) consider using external storage.
pub trait Theme<Draw> {
    /// The associated [`ThemeWindow`] implementation.
    type Window: Window<Draw> + 'static;

    /// The associated [`DrawHandle`] implementation.
    type DrawHandle: DrawHandle;

    /// Construct per-window storage
    ///
    /// A reference to the draw backend is provided allowing configuration.
    ///
    /// See also documentation on [`ThemeWindow::set_dpi_factor`].
    fn new_window(&self, draw: &mut Draw, dpi_factor: f32) -> Self::Window;

    /// Construct a [`DrawHandle`] object
    ///
    /// The `theme_window` is guaranteed to be one created by a call to
    /// [`Theme::new_window`] on `self`, and the `draw` reference is guaranteed
    /// to be identical to the one passed to [`Theme::new_window`].
    ///
    /// Note: this function is marked **unsafe** because the returned object
    /// requires a lifetime bound not exceeding that of all three pointers
    /// passed in. This ought to be expressible using generic associated types
    /// but currently is not: https://github.com/rust-lang/rust/issues/67089
    unsafe fn draw_handle(
        &self,
        draw: &mut Draw,
        theme_window: &mut Self::Window,
    ) -> Self::DrawHandle;

    /// Get the list of available fonts
    ///
    /// Currently, all fonts used must be specified up front by this method.
    /// (Dynamic addition of fonts may be enabled in the future.)
    ///
    /// This is considered a "getter" rather than a "constructor" method since
    /// the `Font` type is cheap to copy, and each window requires its own copy.
    /// It may also be useful to retain a `Font` handle for access to its
    /// methods.
    ///
    /// Corresponding `FontId`s may be created from the index into this list.
    /// The first font in the list will be the default font.
    ///
    /// TODO: this part of the API is dependent on `rusttype::Font`. We should
    /// build an abstraction over this, or possibly just pass the font bytes
    /// (although this makes re-use of fonts between windows difficult).
    fn get_fonts<'a>(&self) -> Vec<Font<'a>>;

    /// Light source
    ///
    /// This affects shadows on frames, etc. The light source has neutral colour
    /// and intensity such that the colour of flat surfaces is unaffected.
    ///
    /// Return value: `(a, b)` where `0 ≤ a < pi/2` is the angle to the screen
    /// normal (i.e. `a = 0` is straight at the screen) and `b` is the bearing
    /// (from UP, clockwise), both in radians.
    ///
    /// Currently this is not updated after initial set-up.
    fn light_direction(&self) -> (f32, f32);

    /// Background colour
    fn clear_colour(&self) -> Colour;
}

/// Per-window storage for the theme
///
/// Constructed via [`Theme::new_window`].
///
/// The main reason for this separation is to allow proper handling of
/// multi-window applications across screens with differing DPIs.
pub trait Window<Draw> {
    /// The associated [`SizeHandle`] implementation.
    type SizeHandle: SizeHandle;

    /// Construct a [`SizeHandle`] object
    ///
    /// The `draw` reference is guaranteed to be identical to the one used to
    /// construct this object.
    ///
    /// Note: this function is marked **unsafe** because the returned object
    /// requires a lifetime bound not exceeding that of all three pointers
    /// passed in. This ought to be expressible using generic associated types
    /// but currently is not: https://github.com/rust-lang/rust/issues/67089
    unsafe fn size_handle(&mut self, draw: &mut Draw) -> Self::SizeHandle;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Set the DPI factor.
    ///
    /// This method is called when the DPI changes (e.g. via system settings or
    /// when a window is moved to a different screen).
    ///
    /// On "standard" monitors, the factor is 1. High-DPI screens may have a
    /// factor of 2 or higher. The factor may not be an integer; e.g.
    /// `9/8 = 1.125` works well with many 1440p screens. It is recommended to
    /// round dimensions to the nearest integer, and cache the result:
    /// ```notest
    /// self.margin = (MARGIN * factor).round();
    /// ```
    fn set_dpi_factor(&mut self, factor: f32);
}

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
    fn text_bound(
        &mut self,
        text: &str,
        class: TextClass,
        multi_line: bool,
        axis: AxisInfo,
    ) -> SizeRules;

    /// Size of the sides of a button.
    ///
    /// Includes each side (as in `outer_frame`), minus the content area (to be added separately).
    fn button_surround(&self) -> (Size, Size);

    /// Size of the sides of an edit box.
    ///
    /// Includes each side (as in `outer_frame`), minus the content area (to be added separately).
    fn edit_surround(&self) -> (Size, Size);

    /// Size of the element drawn by [`DrawHandle::draw_checkbox`].
    ///
    /// This element is not scalable (except by DPI).
    fn checkbox(&self) -> Size;
}

/// Handle passed to objects during draw and sizing operations
pub trait DrawHandle {
    /// Draw a frame in the given [`Rect`]
    ///
    /// The frame dimensions should equal those of [`SizeHandle::frame_size`].
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
    ///
    /// Size is fixed as [`SizeHandle::size_of_checkbox`], thus only the `pos`
    /// and state are needed here.
    fn checkbox(&mut self, pos: Coord, checked: bool, highlights: HighlightState);
}
