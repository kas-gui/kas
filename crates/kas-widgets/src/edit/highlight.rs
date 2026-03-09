// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Supporting elements for syntax highlighting

#[cfg(feature = "syntect")] mod syntect;
mod text;

#[cfg(feature = "syntect")]
pub use syntect::{
    SyntaxReference as SyntectSyntax, SyntaxSet as SyntectSyntaxSet, SyntectHighlighter,
};
pub use text::Text;

use kas::event::ConfigCx;
use kas::text::fonts::{FontStyle, FontWeight};
use kas::text::format::{Color, Colors, Decoration};

/// Colors provided by the highlighter's color scheme
///
/// Note that in each case [`Color::default()`] will resolve to the UI color
/// scheme's color for this property.
#[derive(Debug, Default)]
pub struct SchemeColors {
    /// The default text color
    pub foreground: Color,
    /// The color of selected text
    pub selection_foreground: Color,
    /// The background color of selected text
    pub selection_background: Color,
}

/// A highlighting token
///
/// This token is designed to support all capabilities required by syntax
/// highlighters.
///
/// Some extensions could be possible: font width, (relative) font size,
/// generic font family (Serif/Sans-Serif/Monospace).
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct Token {
    /// Text (foreground) and background color
    pub colors: Colors,
    /// Text weight (bold/medium/light)
    pub weight: FontWeight,
    /// Text style (normal/italic/oblique)
    pub style: FontStyle,
    /// Text decorations (strikethrough, underline)
    pub decoration: Decoration,
}

pub trait Highlighter {
    /// Error type
    ///
    /// TODO(associated_type_defaults): default to [`std::convert::Infallible`]
    type Error: std::error::Error;

    /// Configure the highlighter
    ///
    /// This is called when the widget is configured. It may be used to set the
    /// theme / color scheme.
    ///
    /// The method should return `true` when the highlighter should be re-run.
    fn configure(&mut self, cx: &mut ConfigCx) -> bool;

    /// Get scheme colors
    ///
    /// This method allows usage of the highlighter's colors by the editor.
    fn scheme_colors(&self) -> SchemeColors;

    /// Highlight a `text` as a single item
    ///
    /// The method should yield a sequence of tokens each with a text index
    /// using `push_token`. These must be yielded in order (i.e. `index` must be
    /// strictly increasing).
    ///
    /// # Error handling
    ///
    /// In debug builds errors returned by this method or errors in the order of
    /// tokens' `index` value will result in a panic, while in release builds
    /// these will merely result in a log error and interrupt highlighting.
    fn highlight_text(
        &mut self,
        text: &str,
        push_token: &mut dyn FnMut(usize, Token),
    ) -> Result<(), Self::Error>;
}

/// An implementation of [`Highlighter`] which doesn't highlight anything
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Plain;
impl Highlighter for Plain {
    type Error = std::convert::Infallible;

    #[inline]
    fn configure(&mut self, _: &mut ConfigCx) -> bool {
        false
    }

    #[inline]
    fn scheme_colors(&self) -> SchemeColors {
        SchemeColors::default()
    }

    #[inline]
    fn highlight_text(
        &mut self,
        _: &str,
        _: &mut dyn FnMut(usize, Token),
    ) -> Result<(), Self::Error> {
        Ok::<(), std::convert::Infallible>(())
    }
}
