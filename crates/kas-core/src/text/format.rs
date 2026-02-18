// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Formatted text traits and types

use super::fonts::FontSelector;
pub use kas_text::format::FontToken;

/// Effect formatting marker: text and background color
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Colors {
    /// User-specified value
    ///
    /// Usage is not specified by `kas-text`, but typically this field will be
    /// used as an index into a colour palette or not used at all.
    pub color: u16,
}

/// Decoration types
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DecorationType {
    /// No decoration
    #[default]
    None,
    /// Glyph is underlined
    Underline,
    /// Glyph is crossed through by a center-line
    Strikethrough,
}

/// Decoration styles
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LineStyle {
    /// A single solid line
    #[default]
    Solid,
}

/// Effect formatting marker: strikethrough and underline decorations
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Decoration {
    /// Type of decoration
    pub dec: DecorationType,
    /// Line style
    pub style: LineStyle,
    /// Line color
    pub color: u16,
}

/// Text, optionally with formatting data
pub trait FormattableText: std::cmp::PartialEq {
    /// Access whole text as contiguous `str`
    fn as_str(&self) -> &str;

    /// Return an iterator of font tokens
    ///
    /// These tokens are used to select the font and font size.
    /// Each text object has a configured
    /// [font size][crate::theme::Text::set_font_size] and [`FontSelector`]; these
    /// values are passed as a reference (`dpem` and `font`).
    ///
    /// The iterator is expected to yield a stream of tokens such that
    /// [`FontToken::start`] values are strictly increasing, less than
    /// `self.as_str().len()` and at `char` boundaries (i.e. an index value
    /// returned by [`str::char_indices`]. In case the returned iterator is
    /// empty or the first [`FontToken::start`] value is greater than zero the
    /// reference `dpem` and `font` values are used.
    ///
    /// Any changes to the result of this method require full re-preparation of
    /// text since this affects run breaking and font resolution.
    fn font_tokens(&self, dpem: f32, font: FontSelector) -> impl Iterator<Item = FontToken>;

    /// Return the sequence of color effect tokens
    ///
    /// These tokens may be used for rendering effects: glyph color and
    /// background color.
    ///
    /// Use `&[]` to use the default `Colors` everywhere, or use a sequence such
    /// that `tokens[i].0` values are strictly increasing. A glyph for index `j`
    /// in the source text will use the colors `tokens[i].1` where `i` is the
    /// largest value such that `tokens[i].0 <= j`, or the default `Colors` if
    /// no such `i` exists.
    ///
    /// Changes to the result of this method do not require any re-preparation
    /// of text.
    ///
    /// The default implementation returns `&[]`.
    #[inline]
    fn color_tokens(&self) -> &[(u32, Colors)] {
        &[]
    }

    /// Return optional sequences of decoration tokens
    ///
    /// These tokens may be used for rendering effects: strike-through and
    /// underline decorations.
    ///
    /// Use `&[]` for no decorations, or use a sequence such that `tokens[i].0`
    /// values are strictly increasing. A glyph for index `j` in the source text
    /// will use the decoration `tokens[i].1` where `i` is the largest value
    /// such that `tokens[i].0 <= j`, or no decoration if no such `i` exists.
    ///
    /// Changes to the result of this method do not require any re-preparation
    /// of text.
    ///
    /// The default implementation returns `&[]`.
    #[inline]
    fn decorations(&self) -> &[(u32, Decoration)] {
        &[]
    }
}

impl FormattableText for str {
    #[inline]
    fn as_str(&self) -> &str {
        self
    }

    #[inline]
    fn font_tokens(&self, dpem: f32, font: FontSelector) -> impl Iterator<Item = FontToken> {
        let start = 0;
        std::iter::once(FontToken { start, dpem, font })
    }
}

impl FormattableText for String {
    #[inline]
    fn as_str(&self) -> &str {
        self
    }

    #[inline]
    fn font_tokens(&self, dpem: f32, font: FontSelector) -> impl Iterator<Item = FontToken> {
        let start = 0;
        std::iter::once(FontToken { start, dpem, font })
    }
}

impl<F: FormattableText + ?Sized> FormattableText for &F {
    fn as_str(&self) -> &str {
        F::as_str(self)
    }

    fn font_tokens(&self, dpem: f32, font: FontSelector) -> impl Iterator<Item = FontToken> {
        F::font_tokens(self, dpem, font)
    }

    fn color_tokens(&self) -> &[(u32, Colors)] {
        F::color_tokens(self)
    }

    fn decorations(&self) -> &[(u32, Decoration)] {
        F::decorations(self)
    }
}

#[cfg(test)]
#[test]
fn sizes() {
    use std::mem::size_of;

    assert_eq!(size_of::<Colors>(), 2);
    assert_eq!(size_of::<DecorationType>(), 1);
    assert_eq!(size_of::<LineStyle>(), 0);
    assert_eq!(size_of::<Decoration>(), 4);
}
