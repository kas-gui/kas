// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Supporting elements for syntax highlighting

mod text;

pub use text::Text;

use kas::text::fonts::{FontStyle, FontWeight};
use kas::text::format::{Colors, Decoration};

/// A highlighting token
///
/// This token is designed to support all capabilities required by syntax
/// highlighters.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct Token {
    pub colors: Colors,
    pub weight: FontWeight,
    pub style: FontStyle,
    pub decoration: Decoration,
}

pub trait Highlighter {
    /// Error type
    ///
    /// TODO(associated_type_defaults): default to [`std::convert::Infallible`]
    type Error: std::error::Error;

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

    fn highlight_text(
        &mut self,
        _: &str,
        _: &mut dyn FnMut(usize, Token),
    ) -> Result<(), Self::Error> {
        Ok::<(), std::convert::Infallible>(())
    }
}
