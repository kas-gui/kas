// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text-drawing API

use super::Colour;
use crate::geom::Rect;
use crate::Align;

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

/// Abstraction over text rendering
///
/// This trait is an extension over [`Draw`] providing basic text rendering.
/// Rendering makes use of transparency and should occur last in
/// implementations which buffer draw commands.
///
/// Note: the current API is designed to meet only current requirements since
/// changes are expected to support external font shaping libraries.
///
/// [`Draw`]: super::Draw
pub trait DrawText {
    /// Simple text drawing
    ///
    /// This allows text to be drawn according to a high-level API, and should
    /// satisfy most uses.
    fn text(&mut self, rect: Rect, text: &str, font_scale: f32, props: TextProperties, col: Colour);

    /// Calculate size bound on text
    ///
    /// This may be used with [`DrawText::text`] to calculate size requirements
    /// within [`kas::Layout::size_rules`].
    ///
    /// Bounds of `(f32::INFINITY, f32::INFINITY)` may be used if there are no
    /// constraints. This parameter allows forcing line-wrapping behaviour
    /// within the given bounds.
    fn text_bound(
        &mut self,
        text: &str,
        font_scale: f32,
        bounds: (f32, f32),
        line_wrap: bool,
    ) -> (f32, f32);
}
