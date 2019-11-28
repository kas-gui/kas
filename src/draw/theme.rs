// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget styling
//!
//! Widget size and appearance can be modified through themes.

use rusttype::Font;

use crate::layout::{AxisInfo, SizeRules};
use kas::draw::*;
use kas::geom::Size;
use kas::{event, Widget};

/// A *theme* provides widget sizing and drawing implementations.
///
/// Objects of this type are copied within each window's data structure. For
/// large resources (e.g. fonts and icons) consider using external storage.
pub trait Theme: Clone {
    /// Implementor of draw API
    type Draw;

    /// Set the DPI factor.
    ///
    /// This method is called after constructing a window and each time the DPI
    /// changes (e.g. via system settings or with monitor-specific DPI factors).
    ///
    /// On "standard" monitors, the factor is 1. High-DPI screens may have a
    /// factor of 2 or higher. The factor may not be an integer; e.g.
    /// `9/8 = 1.125` works well with many 1440p screens. It is recommended to
    /// round dimensions to the nearest integer, and cache the result:
    /// ```notest
    /// self.margin = (MARGIN * factor).round();
    /// ```
    fn set_dpi_factor(&mut self, factor: f32);

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

    /// Margin between child widgets
    ///
    /// This is only applicable to parents with multiple child widgets.
    fn inner_margin(&self, widget: &dyn Widget, axis_is_vertical: bool) -> u32;

    /// Margins around child widgets
    ///
    /// Returns three components: size of the top-left margin, size of
    /// inter-widget margins, size of the bottom-right margin.
    ///
    /// These must match the margins returned by `size_rules` and `inner_margin`
    /// for correct operation.
    fn child_margins(&self, widget: &dyn Widget) -> (Size, Size, Size);

    /// Widget dimensions
    ///
    /// Used to specify the dimension of a widget, based on class and contents.
    /// This should include margins (except inter-child margins).
    ///
    /// On parent widgets, this should only return the size of margins and any
    /// area dedicated to self.
    fn size_rules(&self, draw: &mut Self::Draw, widget: &dyn Widget, axis: AxisInfo) -> SizeRules;

    /// Draw a widget
    ///
    /// This method is called to draw each visible widget (and should not
    /// attempt recursion on child widgets).
    fn draw(&self, draw: &mut Self::Draw, ev_mgr: &event::Manager, widget: &dyn kas::Widget);
}
