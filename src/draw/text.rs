// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text-drawing API

use super::{Colour, Pass};
use crate::geom::{Rect, Vec2};
use crate::text::{FontId, PreparedText};
use crate::Align;

// TODO: remove PxScale when removing TextPart
pub use kas_text::FontScale as PxScale;

/// A part of a text section
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct TextPart {
    /// The start (inclusive) of this text part within the whole text
    pub start: u32,
    /// The end (exclusive) of this text part within the whole text
    pub end: u32,
    /// Font scale
    ///
    /// This is approximately the pixel-height of a line of text or double the
    /// "pt" size. Usually you want to use the same scale for both components,
    /// e.g. `PxScale::from(18.0)`.
    pub scale: PxScale,
    /// The font
    pub font: FontId,
    /// Font colour
    pub col: Colour,
}

impl TextPart {
    /// Byte-length of part (`end - start`)
    #[inline]
    pub fn len(&self) -> usize {
        (self.end - self.start) as usize
    }

    /// Byte-range
    #[inline]
    pub fn range(&self) -> std::ops::Range<usize> {
        (self.start as usize)..(self.end as usize)
    }
}

/// A text section, as drawn by [`DrawText::text`]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct TextSection<'a> {
    /// The whole text
    ///
    /// Note: each [`TextPart`] references a sub-set of this. Sub-sets may
    /// overlap and are not required to cover the whole of this text.
    pub text: &'a str,
    /// The rect within which the text is drawn
    pub rect: Rect,
    /// Text alignment in horizontal and vertical directions
    pub align: (Align, Align),
    /// True if text should automatically be line-wrapped
    pub line_wrap: bool,
    /// Text parts to draw
    pub parts: &'a [TextPart],
}

/// Text properties for use by [`DrawText::text`]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct TextProperties {
    /// The font
    pub font: FontId,
    /// Font scale
    ///
    /// This is approximately the pixel-height of a line of text or double the
    /// "pt" size. Usually you want to use the same scale for both components,
    /// e.g. `PxScale::from(18.0)`.
    pub scale: PxScale,
    /// Font colour
    pub col: Colour,
    /// Text alignment in horizontal and vertical directions
    pub align: (Align, Align),
    /// True if text should automatically be line-wrapped
    pub line_wrap: bool,
}

impl Default for TextProperties {
    fn default() -> Self {
        TextProperties {
            font: Default::default(),
            scale: 18.0.into(),
            col: Default::default(),
            align: Default::default(),
            line_wrap: Default::default(),
        }
    }
}

/// Abstraction over text rendering
///
/// Note: the current API is designed to meet only current requirements since
/// changes are expected to support external font shaping libraries.
pub trait DrawText {
    /// Draw text
    fn text(&mut self, pass: Pass, pos: Vec2, col: Colour, text: &PreparedText);

    /// Find the starting position (top-left) of the glyph at the given index
    ///
    /// May panic on invalid byte index.
    ///
    /// This method is only partially compatible with mult-line text.
    /// Ideally an external line-breaker should be used.
    fn text_glyph_pos(&mut self, text: TextSection, byte: usize) -> Vec2;

    /// Find the text index for the glyph nearest the given `pos`
    ///
    /// This includes the index immediately after the last glyph, thus
    /// `result ≤ text.len()`.
    ///
    /// This method is only partially compatible with mult-line text.
    /// Ideally an external line-breaker should be used.
    fn text_index_nearest(&mut self, text: TextSection, pos: Vec2) -> usize;
}
