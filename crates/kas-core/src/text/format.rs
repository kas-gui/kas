// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Formatted text traits and types

use super::fonts::FontSelector;
pub use kas_text::format::FontToken;

/// Effect formatting marker
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Effect {
    /// User-specified value
    ///
    /// Usage is not specified by `kas-text`, but typically this field will be
    /// used as an index into a colour palette or not used at all.
    pub color: u16,
    /// Effect flags
    pub flags: EffectFlags,
}

bitflags::bitflags! {
    /// Text effects
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct EffectFlags: u16 {
        /// Glyph is underlined
        const UNDERLINE = 1 << 0;
        /// Glyph is crossed through by a center-line
        const STRIKETHROUGH = 1 << 1;
    }
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

    /// Return the sequence of effect tokens
    ///
    /// The `effects` sequence may be used for rendering effects: glyph color,
    /// background color, strike-through, underline. Use `&[]` for no effects
    /// (effectively using the default value of `Self::Effect` everywhere), or
    /// use a sequence such that `effects[i].0` values are strictly increasing.
    /// A glyph for index `j` in the source text will use effect `effects[i].1`
    /// where `i` is the largest value such that `effects[i].0 <= j`, or the
    /// default value of `Self::Effect` if no such `i` exists.
    ///
    /// Changes to the result of this method do not require any re-preparation
    /// of text.
    fn effect_tokens(&self) -> &[(u32, Effect)];
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

    #[inline]
    fn effect_tokens(&self) -> &[(u32, Effect)] {
        &[]
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

    #[inline]
    fn effect_tokens(&self) -> &[(u32, Effect)] {
        &[]
    }
}

impl<F: FormattableText + ?Sized> FormattableText for &F {
    fn as_str(&self) -> &str {
        F::as_str(self)
    }

    fn font_tokens(&self, dpem: f32, font: FontSelector) -> impl Iterator<Item = FontToken> {
        F::font_tokens(self, dpem, font)
    }

    fn effect_tokens(&self) -> &[(u32, Effect)] {
        F::effect_tokens(self)
    }
}
