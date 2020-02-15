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
//! It is also responsible for creating a per-window [`Window`] object.
//!
//! Finally, the [`SizeHandle`] and [`DrawHandle`] traits provide actual sizing
//! and drawing information for widgets. Widgets are provided implementations of
//! these traits within calls to the appropriate [`Widget`] methods.
//!
//! [`Widget`]: crate::Widget

use std::any::Any;

use rusttype::Font;

use kas::draw::Colour;
use kas::event::HighlightState;
use kas::geom::{Coord, Rect, Size};
use kas::layout::{AxisInfo, SizeRules};
use kas::{Align, Direction};

/// Class of text drawn
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

/// Text alignment, class, etc.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextProperties {
    /// Class of text
    pub class: TextClass,
    /// Horizontal alignment
    pub horiz: Align,
    /// Vertical alignment
    pub vert: Align,
    // Note: do we want to add HighlightState?
}

/// Toolkit actions needed after theme adjustment, if any
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum ThemeAction {
    /// No action needed
    None,
    /// All windows require redrawing
    RedrawAll,
    /// Theme sizes changed: must call [`Theme::update_window`] and resize
    ThemeResize,
}

/// Interface through which a theme can be adjusted at run-time
///
/// All methods return a [`ThemeAction`] to enable correct action when a theme
/// is updated via [`Manager::adjust_theme`]. When adjusting a theme before
/// the UI is started, this return value can be safely ignored.
pub trait ThemeApi {
    /// Set font size. Default is 18. Units are unknown.
    fn set_font_size(&mut self, size: f32) -> ThemeAction;

    /// Change the colour scheme
    ///
    /// If no theme by this name is found, the theme is unchanged.
    // TODO: revise scheme identification and error handling?
    fn set_colours(&mut self, _scheme: &str) -> ThemeAction;

    /// Change the theme itself
    ///
    /// Themes may do nothing, or may react according to their own
    /// interpretation of this method.
    fn set_theme(&mut self, _theme: &str) -> ThemeAction {
        ThemeAction::None
    }
}

/// A *theme* provides widget sizing and drawing implementations.
///
/// The theme is generic over some `Draw` type.
///
/// Objects of this type are copied within each window's data structure. For
/// large resources (e.g. fonts and icons) consider using external storage.
pub trait Theme<Draw>: ThemeApi {
    /// The associated [`Window`] implementation.
    type Window: Window<Draw> + 'static;

    /// The associated [`DrawHandle`] implementation.
    type DrawHandle<'a>: DrawHandle;

    /// Construct per-window storage
    ///
    /// On "standard" monitors, the `dpi_factor` is 1. High-DPI screens may
    /// have a factor of 2 or higher. The factor may not be an integer; e.g.
    /// `9/8 = 1.125` works well with many 1440p screens. It is recommended to
    /// round dimensions to the nearest integer, and cache the result:
    /// ```notest
    /// self.margin = (MARGIN * factor).round() as u32;
    /// ```
    ///
    /// A reference to the draw backend is provided allowing configuration.
    fn new_window(&self, draw: &mut Draw, dpi_factor: f32) -> Self::Window;

    /// Update a window created by [`Theme::new_window`]
    ///
    /// This is called when the DPI factor changes or theme dimensions change.
    fn update_window(&self, window: &mut Self::Window, dpi_factor: f32);

    /// Construct a [`DrawHandle`] object
    ///
    /// Drawing via this [`DrawHandle`] is restricted to the specified `rect`.
    ///
    /// The `theme_window` is guaranteed to be one created by a call to
    /// [`Theme::new_window`] on `self`, and the `draw` reference is guaranteed
    /// to be identical to the one passed to [`Theme::new_window`].
    fn draw_handle<'a>(
        &'a self,
        draw: &'a mut Draw,
        theme_window: &'a mut Self::Window,
        rect: Rect,
    ) -> Self::DrawHandle<'a>;

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
