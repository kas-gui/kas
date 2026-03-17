// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Formatted text traits and types

use std::num::NonZeroU32;

use super::fonts::FontSelector;
use crate::draw::color::{Rgba, Rgba8Srgb};
use crate::theme::ColorsLinear;
pub use kas_text::format::FontToken;

#[cfg(feature = "markdown")] mod markdown;
#[cfg(feature = "markdown")] pub use markdown::Markdown;

/// Rgba or theme-provided color value
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Color(NonZeroU32);

impl Default for Color {
    /// Use a theme-defined color (automatic)
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Color {
    /// Use the default theme-defined color
    ///
    /// As a foreground color, this maps to [`ColorsLinear::text`] or
    /// [`ColorsLinear::text_invert`] depending on the background.
    ///
    /// As a background color, this maps to [`ColorsLinear::edit_bg`].
    pub const DEFAULT: Self =
        Color(NonZeroU32::new(u32::from_ne_bytes(Rgba8Srgb::rgba(1, 0, 0, 0).0)).unwrap());

    /// Use the text-selection color
    ///
    /// As a foreground color this is identical to [`Self::DEFAULT`].
    ///
    /// As a background color, this maps to [`ColorsLinear::text_sel_bg`].
    pub const SELECTION: Self =
        Color(NonZeroU32::new(u32::from_ne_bytes(Rgba8Srgb::rgba(1, 1, 0, 0).0)).unwrap());

    /// Use an RGBA sRGB color
    ///
    /// This will resolve to the default theme color if `color.a() == 0`.
    #[inline]
    pub fn from_rgba_srgb(color: Rgba8Srgb) -> Self {
        if color.a() == 0 {
            return Self::default();
        }
        Color(NonZeroU32::new(u32::from_ne_bytes(color.0)).unwrap())
    }

    /// Use an RGBA color
    ///
    /// Note that this converts to [`Rgba8Srgb`] internally, thus some color
    /// information may be lost.
    ///
    /// This will resolve to the default theme color if `Rgba8Srgb::from(color).a() == 0`.
    #[inline]
    pub fn from_rgba(color: Rgba) -> Self {
        Self::from_rgba_srgb(color.into())
    }

    /// Get the RGBA sRGB color, if any
    #[inline]
    pub fn as_rgba_srgb(self) -> Option<Rgba8Srgb> {
        let col = Rgba8Srgb(u32::to_ne_bytes(self.0.get()));
        (col.a() != 0).then_some(col)
    }

    /// Get the RGBA color, if any
    #[inline]
    pub fn as_rgba(self) -> Option<Rgba> {
        self.as_rgba_srgb().map(|c| c.into())
    }

    /// Resolve as (foreground) text color
    #[inline]
    pub fn resolve_foreground(self, theme: &ColorsLinear, bg: Option<Rgba>) -> Rgba {
        if let Some(col) = self.as_rgba() {
            col
        } else if let Some(bg) = bg {
            theme.text_over(bg)
        } else {
            theme.text
        }
    }

    /// Resolve as background color
    #[inline]
    pub fn resolve_background(self, theme: &ColorsLinear) -> Rgba {
        if let Some(col) = self.as_rgba() {
            col
        } else if self == Self::SELECTION {
            theme.text_sel_bg
        } else {
            theme.edit_bg
        }
    }
}

/// Effect formatting marker: text and background color
///
/// By default, this uses the theme's text color without a background.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Colors {
    /// The default text color
    pub foreground: Color,
    /// The text background (highlight) color
    pub background: Option<Color>,
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
    pub color: Color,
}

/// Text, optionally with formatting data
pub trait FormattableText: std::cmp::PartialEq {
    /// Access whole text as contiguous `str`
    fn as_str(&self) -> &str;

    /// Return an iterator of font tokens
    ///
    /// These tokens are used to select the font and font size.
    /// Each text object has a configured
    /// [font size][super::ConfiguredDisplay::set_font_size] and [`FontSelector`]; these
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

    assert_eq!(size_of::<Colors>(), 8);
    assert_eq!(size_of::<DecorationType>(), 1);
    assert_eq!(size_of::<LineStyle>(), 0);
    assert_eq!(size_of::<Decoration>(), 8);
}
