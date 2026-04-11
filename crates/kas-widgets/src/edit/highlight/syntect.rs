// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Syntax highlighting using [`syntect`](https://crates.io/crates/syntect)

use super::{ActionRestart, SchemeColors, Token};
use kas::draw::color::Rgba8Srgb;
use kas::event::ConfigCx;
use kas::text::fonts::FontWeight;
use kas::text::format::{Color, DecorationType};
use std::sync::OnceLock;
use syntect::highlighting::{
    FontStyle, HighlightState, Highlighter, RangedHighlightIterator, Theme, ThemeSet,
};
use syntect::parsing::{ParseState, ParsingError};

pub use syntect::parsing::{SyntaxReference, SyntaxSet};

fn themes() -> &'static ThemeSet {
    static SET: OnceLock<ThemeSet> = OnceLock::new();
    SET.get_or_init(|| {
        let mut set = ThemeSet::load_defaults();
        for theme in set.themes.values_mut() {
            if let Some(c) = theme.settings.background.as_mut() {
                c.a = 0;
            }
        }
        set
    })
}

/// A highlighter using [`syntect`](https://crates.io/crates/syntect)
pub struct SyntectHighlighter {
    syntax: &'static SyntaxReference,
    dark: bool,
    theme: &'static Theme,
    highlighter: Highlighter<'static>,
}

impl SyntectHighlighter {
    /// Access the compiled-in syntax set
    ///
    /// See [`SyntaxSet::load_defaults_newlines`] documentation.
    pub fn syntaxes() -> &'static SyntaxSet {
        static SET: OnceLock<SyntaxSet> = OnceLock::new();
        SET.get_or_init(|| SyntaxSet::load_defaults_newlines())
    }

    /// Construct a new highlighter for the given [`SyntaxReference`]
    #[inline]
    pub fn new(syntax: &'static SyntaxReference) -> Self {
        let theme = themes().themes.get("InspiredGitHub").unwrap();

        SyntectHighlighter {
            syntax,
            dark: false,
            theme,
            highlighter: Highlighter::new(theme),
        }
    }

    /// Construct a new "highlighter" for plain text
    #[inline]
    pub fn new_plain() -> Self {
        Self::new(Self::syntaxes().find_syntax_plain_text())
    }

    /// Construct a new highlighter for a given language by name
    ///
    /// Falls back to plain text mode if `name` is not found.
    #[inline]
    pub fn new_by_name(name: &str) -> Self {
        let syntaxes = Self::syntaxes();
        let syntax = syntaxes
            .find_syntax_by_name(name)
            .unwrap_or_else(|| syntaxes.find_syntax_plain_text());

        Self::new(syntax)
    }

    /// Construct a new highlighter for a given language by extension
    ///
    /// Falls back to plain text mode if `ext` is not found.
    #[inline]
    pub fn new_by_extension(ext: &str) -> Self {
        let syntaxes = Self::syntaxes();
        let syntax = syntaxes
            .find_syntax_by_extension(ext)
            .unwrap_or_else(|| syntaxes.find_syntax_plain_text());

        Self::new(syntax)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State(HighlightState, ParseState);

impl super::Highlighter for SyntectHighlighter {
    type Error = ParsingError;
    type State = State;

    fn configure(&mut self, cx: &mut ConfigCx) -> Option<ActionRestart> {
        let dark = cx.config().theme().get_active_scheme().is_dark;
        if dark == self.dark {
            return None;
        }

        self.dark = dark;
        let name = if dark { "base16-ocean.dark" } else { "InspiredGitHub" };
        self.theme = themes().themes.get(name).unwrap();
        self.highlighter = Highlighter::new(self.theme);
        Some(ActionRestart)
    }

    fn scheme_colors(&self) -> SchemeColors {
        SchemeColors {
            foreground: self
                .theme
                .settings
                .foreground
                .map(|c| into_kas_text_color(c))
                .unwrap_or_default(),
            background: self
                .theme
                .settings
                .background
                .map(|mut c| {
                    c.a = 255;
                    into_kas_text_color(c)
                })
                .unwrap_or_default(),
            cursor: self
                .theme
                .settings
                .caret
                .map(|c| into_kas_text_color(c))
                .unwrap_or_default(),
            selection_foreground: self
                .theme
                .settings
                .selection_foreground
                .map(|c| into_kas_text_color(c))
                .unwrap_or(Color::SELECTION),
            selection_background: self
                .theme
                .settings
                .selection
                .map(|c| into_kas_text_color(c))
                .unwrap_or(Color::SELECTION),
        }
    }

    #[inline]
    fn new_state(&self) -> Self::State {
        let state = HighlightState::new(&self.highlighter, Default::default());
        let parse_state = ParseState::new(&self.syntax);
        State(state, parse_state)
    }

    #[inline]
    fn highlight_line(
        &self,
        state: &mut Self::State,
        line: &str,
        mut push_token: impl FnMut(usize, Token),
    ) -> Result<(), Self::Error> {
        let changes = state.1.parse_line(line, Self::syntaxes())?;
        let line_highlighter =
            RangedHighlightIterator::new(&mut state.0, &changes, line, &self.highlighter);

        for (style, _, range) in line_highlighter {
            let mut token = Token::default();
            token.colors.foreground = into_kas_text_color(style.foreground);
            token.colors.background = if style.background.a == 0 {
                None
            } else {
                Some(into_kas_text_color(style.background))
            };
            if style.font_style.contains(FontStyle::BOLD) {
                token.weight = FontWeight::BOLD;
            }
            if style.font_style.contains(FontStyle::UNDERLINE) {
                token.decoration.dec = DecorationType::Underline;
            }
            if style.font_style.contains(FontStyle::ITALIC) {
                token.style = kas::text::fonts::FontStyle::Italic;
            }
            push_token(range.start, token);
        }

        Ok(())
    }
}

/// Convert to `Color`, even if transparent
fn into_kas_text_color(c: ::syntect::highlighting::Color) -> Color {
    Color::from_rgba_srgb(Rgba8Srgb::rgba(c.r, c.g, c.b, c.a))
}
