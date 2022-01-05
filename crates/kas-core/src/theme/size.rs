// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! "Handle" types used by themes

use std::ops::Deref;

#[allow(unused)]
use super::DrawHandle;
use super::TextClass;
use crate::geom::Size;
use crate::layout::{AxisInfo, FrameRules, Margins, SizeRules};
use crate::text::TextApi;
// for doc use
#[allow(unused)]
use crate::text::TextApiExt;

/// A handle to the active theme, used for sizing
///
/// The shell provides widgets a `&dyn SizeHandle` in [`crate::Layout::size_rules`].
/// It may also be accessed through [`crate::event::Manager::size_handle`] and
/// [`DrawHandle::size_handle`].
///
/// All methods get or calculate the size of some feature.
///
/// See also [`DrawHandle`].
pub trait SizeHandle {
    /// Get the scale (DPI) factor
    ///
    /// "Traditional" PC screens have a scale factor of 1; high-DPI screens
    /// may have a factor of 2 or higher; this may be fractional. It is
    /// recommended to calculate sizes as follows:
    /// ```
    /// use kas_core::cast::*;
    /// # let scale_factor = 1.5f32;
    /// let size: i32 = (100.0 * scale_factor).cast_ceil();
    /// ```
    ///
    /// This value may change during a program's execution (e.g. when a window
    /// is moved to a different monitor); in this case all widgets will be
    /// resized via [`crate::Layout::size_rules`].
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
    ///
    /// This already includes the margins specified by [`Self::frame_margins`].
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

    /// The margin around frames and separators
    fn frame_margins(&self) -> Margins;

    /// The margin around text elements
    ///
    /// Similar to [`Self::outer_margins`], but intended for things like text
    /// labels which do not have a visible hard edge.
    fn text_margins(&self) -> Margins;

    /// The height of a line of text
    fn line_height(&self, class: TextClass) -> i32;

    /// Update a [`crate::text::Text`] and get a size bound
    ///
    /// First, this method updates the text's [`Environment`]: `bounds`, `dpp`
    /// and `pt_size` are set. Second, the text is prepared (which is necessary
    /// to calculate size requirements). Finally, this converts the requirements
    /// to a [`SizeRules`] value and returns it.
    ///
    /// Usually this method is used in [`Layout::size_rules`], then
    /// [`TextApiExt::update_env`] is used in [`Layout::set_rect`].
    ///
    /// [`Environment`]: crate::text::Environment
    /// [`Layout::set_rect`]: crate::Layout::set_rect
    /// [`Layout::size_rules`]: crate::Layout::size_rules
    fn text_bound(&self, text: &mut dyn TextApi, class: TextClass, axis: AxisInfo) -> SizeRules;

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

macro_rules! impl_ {
    (($($args:tt)*) SizeHandle for $ty:ty) => {
        impl<$($args)*> SizeHandle for $ty {
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
            fn frame_margins(&self) -> Margins {
                self.deref().frame_margins()
            }
            fn text_margins(&self) -> Margins {
                self.deref().text_margins()
            }

            fn line_height(&self, class: TextClass) -> i32 {
                self.deref().line_height(class)
            }
            fn text_bound(&self, text: &mut dyn TextApi, class: TextClass, axis: AxisInfo) -> SizeRules {
                self.deref().text_bound(text, class, axis)
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
    };
}

impl_! { (S: SizeHandle + ?Sized, R: Deref<Target = S>) SizeHandle for R }
