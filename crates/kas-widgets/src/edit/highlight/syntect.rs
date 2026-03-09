// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Syntax highlighting using [`syntect`](https://crates.io/crates/syntect)

use super::{SchemeColors, Token};
use kas::draw::color::Rgba8Srgb;
use kas::event::ConfigCx;
use kas::text::LineIterator;
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

impl super::Highlighter for SyntectHighlighter {
    type Error = ParsingError;

    fn configure(&mut self, cx: &mut ConfigCx) -> bool {
        let dark = cx.config().theme().get_active_scheme().is_dark;
        if dark == self.dark {
            return false;
        }

        self.dark = dark;
        let name = if dark { "base16-ocean.dark" } else { "InspiredGitHub" };
        self.theme = themes().themes.get(name).unwrap();
        self.highlighter = Highlighter::new(self.theme);
        true
    }

    fn scheme_colors(&self) -> SchemeColors {
        SchemeColors {
            foreground: self
                .theme
                .settings
                .foreground
                .and_then(|c| into_kas_text_color(c))
                .unwrap_or_default(),
            background: self
                .theme
                .settings
                .background
                .and_then(|mut c| {
                    c.a = 255;
                    into_kas_text_color(c)
                })
                .unwrap_or_default(),
            cursor: self
                .theme
                .settings
                .caret
                .and_then(|c| into_kas_text_color(c))
                .unwrap_or_default(),
            selection_foreground: self
                .theme
                .settings
                .selection_foreground
                .and_then(|c| into_kas_text_color(c))
                .unwrap_or_default(),
            selection_background: self
                .theme
                .settings
                .selection
                .and_then(|c| into_kas_text_color(c))
                .unwrap_or_default(),
        }
    }

    fn highlight_text(
        &mut self,
        text: &str,
        push_token: &mut dyn FnMut(usize, Token),
    ) -> Result<(), Self::Error> {
        let syntaxes = Self::syntaxes();

        let mut state = HighlightState::new(&self.highlighter, Default::default());
        let mut parse_state = ParseState::new(&self.syntax);

        for line_range in LineIterator::new(text) {
            let line_start = line_range.start;
            let line = &text[line_range];
            let changes = parse_state.parse_line(line, &syntaxes)?;
            let line_highlighter =
                RangedHighlightIterator::new(&mut state, &changes, line, &self.highlighter);

            for (style, _, range) in line_highlighter {
                let mut token = Token::default();
                token.colors.color =
                    into_kas_text_color(style.foreground).unwrap_or(Default::default());
                token.colors.background = into_kas_text_color(style.background);
                if style.font_style.contains(FontStyle::BOLD) {
                    token.weight = FontWeight::BOLD;
                }
                if style.font_style.contains(FontStyle::UNDERLINE) {
                    token.decoration.dec = DecorationType::Underline;
                }
                if style.font_style.contains(FontStyle::ITALIC) {
                    token.style = kas::text::fonts::FontStyle::Italic;
                }
                push_token(line_start + range.start, token);
            }
        }

        Ok(())
    }
}

fn into_kas_text_color(c: ::syntect::highlighting::Color) -> Option<Color> {
    if c.a == 0 {
        return None;
    }

    Some(Color::from_rgba_srgb(Rgba8Srgb::rgba(c.r, c.g, c.b, c.a)))
}
