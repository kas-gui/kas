// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Supporting elements for syntax highlighting

mod cache;
#[cfg(feature = "syntect")] mod syntect;

pub(crate) use cache::Cache;
use kas::impl_scope;
#[cfg(feature = "syntect")]
pub use syntect::{
    SyntaxReference as SyntectSyntax, SyntaxSet as SyntectSyntaxSet, SyntectHighlighter,
};

use kas::event::ConfigCx;
use kas::text::fonts::{FontStyle, FontWeight};
use kas::text::format::{Color, Colors, Decoration};

impl_scope! {
    /// Colors provided by the highlighter's color scheme
    #[impl_default]
    #[derive(Debug)]
    pub struct SchemeColors {
        /// The default text color
        pub foreground: Color,
        /// The default background color
        pub background: Color,
        /// The color of the text cursor (sometimes called caret)
        pub cursor: Color,
        /// The color of selected text
        ///
        /// Note that the default value is [`Color::SELECTION`].
        pub selection_foreground: Color = Color::SELECTION,
        /// The background color of selected text
        ///
        /// Note that the default value is [`Color::SELECTION`].
        pub selection_background: Color = Color::SELECTION,
    }
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
    ///
    /// The background color should be `None` unless highlighting is desired.
    /// Specify the default background color using [`SchemeColors::background`].
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

    /// State used to save/resume highlighting
    type State: Clone + Eq;

    /// Configure the highlighter
    ///
    /// This is called when the widget is configured. It may be used to set the
    /// theme / color scheme.
    ///
    /// The method should return `true` when the highlighter should be re-run.
    #[must_use]
    fn configure(&mut self, cx: &mut ConfigCx) -> bool;

    /// Get scheme colors
    ///
    /// This method allows usage of the highlighter's colors by the editor.
    fn scheme_colors(&self) -> SchemeColors;

    /// Construct a new highlighting state
    fn new_state(&self) -> Self::State;

    /// Highlight a `line` of text using a `state`
    ///
    /// The `state` used tracks the parse state and highlighting scope across
    /// lines. At the start of a document a [new state](Self::new_state) must be
    /// used; in other cases the input `state` must be the output `state` from
    /// highlighting the previous line.
    ///
    /// The `line` passed must represent a single whole line of text (including
    /// terminating line-break characters) for correct parsing.
    ///
    /// The method should yield a sequence of tokens each with a text index
    /// (within `line`) using `push_token`. These must be yielded in order (i.e.
    /// `index` must be strictly increasing).
    ///
    /// # Error handling
    ///
    /// In debug builds errors returned by this method or errors in the order of
    /// tokens' `index` value will result in a panic, while in release builds
    /// these will merely result in a log error and interrupt highlighting.
    fn highlight_line(
        &self,
        state: &mut Self::State,
        line: &str,
        push_token: impl FnMut(usize, Token),
    ) -> Result<(), Self::Error>;
}

/// An implementation of [`Highlighter`] which doesn't highlight anything
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Plain;
impl Highlighter for Plain {
    type Error = std::convert::Infallible;
    type State = ();

    #[inline]
    fn configure(&mut self, _: &mut ConfigCx) -> bool {
        false
    }

    #[inline]
    fn scheme_colors(&self) -> SchemeColors {
        SchemeColors::default()
    }

    #[inline]
    fn new_state(&self) -> Self::State {
        ()
    }

    #[inline]
    fn highlight_line(
        &self,
        _: &mut Self::State,
        _: &str,
        _: impl FnMut(usize, Token),
    ) -> Result<(), Self::Error> {
        Ok::<(), std::convert::Infallible>(())
    }
}
