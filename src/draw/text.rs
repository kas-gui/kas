// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text-drawing API

use super::{Colour, Draw, DrawShared, Pass};
use crate::geom::{Rect, Vec2};
use crate::Align;

/// Font scale
///
/// This is approximately the pixel-height of a line of text or double the
/// "pt" size. Usually you want to use the same scale for both components,
/// e.g. `PxScale::from(18.0)`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PxScale {
    pub x: f32,
    pub y: f32,
}

impl Default for PxScale {
    fn default() -> Self {
        PxScale::from(18.0)
    }
}

impl From<f32> for PxScale {
    fn from(scale: f32) -> Self {
        PxScale { x: scale, y: scale }
    }
}

/// Font identifier
///
/// A default font may be obtained with `FontId(0)`, which refers to the
/// first font loaded by the (first) theme.
///
/// Other than this, users should treat this type as an opaque handle.
/// An instance may be obtained by [`DrawTextShared::load_font`].
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct FontId(pub usize);

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

/// Abstraction over type shared by [`DrawText`] implementations
pub trait DrawTextShared: DrawShared {
    /// Load a font
    ///
    /// For font collections, the `index` is used to identify the font;
    /// otherwise it is expected to be 0.
    fn load_font_static_ref(&mut self, data: &'static [u8], index: u32) -> FontId;

    /// Load a font
    ///
    /// For font collections, the `index` is used to identify the font;
    /// otherwise it is expected to be 0.
    fn load_font_vec(&mut self, data: Vec<u8>, index: u32) -> FontId;
}

/// Abstraction over text rendering
///
/// This trait is an extension over [`Draw`] providing basic text rendering.
/// Rendering makes use of transparency and should occur last in
/// implementations which buffer draw commands.
///
/// Note: the current API is designed to meet only current requirements since
/// changes are expected to support external font shaping libraries.
pub trait DrawText: Draw {
    /// Text section (uniform)
    ///
    /// This method provides a simpler API around [`DrawText::text_section`].
    fn text(&mut self, pass: Pass, rect: Rect, text: &str, props: TextProperties) {
        let end = text.len() as u32;
        self.text_section(
            pass,
            TextSection {
                text,
                rect,
                align: props.align,
                line_wrap: props.line_wrap,
                parts: &[TextPart {
                    start: 0,
                    end,
                    scale: props.scale,
                    font: props.font,
                    col: props.col,
                }],
            },
        );
    }

    /// Text section (varying)
    ///
    /// A "text section" represents a block of text (e.g. a line or paragraph)
    /// with common layout, but potentially varying properties (including
    /// colour, size and font).
    fn text_section(&mut self, pass: Pass, text: TextSection);

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
        bounds: (f32, f32),
        line_wrap: bool,
        text: &str,
        parts: &[TextPart],
    ) -> (f32, f32);

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
