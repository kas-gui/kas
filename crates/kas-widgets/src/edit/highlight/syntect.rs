// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Syntax highlighting using [`syntect`](https://crates.io/crates/syntect)

use super::Token;
use kas::draw::color::Rgba8Srgb;
use kas::text::LineIterator;
use kas::text::fonts::FontWeight;
use kas::text::format::{Color, DecorationType};
use std::sync::OnceLock;
use syntect::highlighting::{
    FontStyle, HighlightState, Highlighter, RangedHighlightIterator, ThemeSet,
};
use syntect::parsing::{ParseState, ParsingError, SyntaxReference, SyntaxSet};

fn syntaxes() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(|| SyntaxSet::load_defaults_newlines())
}

fn themes() -> &'static ThemeSet {
    static SET: OnceLock<ThemeSet> = OnceLock::new();
    SET.get_or_init(|| ThemeSet::load_defaults())
}

/// A highlighter using [`syntect`](https://crates.io/crates/syntect)
pub struct SyntectHighlighter {
    syntax: &'static SyntaxReference,
    highlighter: Highlighter<'static>,
}

impl SyntectHighlighter {
    /// Get a highlighter
    #[inline]
    pub fn new() -> Self {
        let syntaxes = syntaxes();
        let syntax = syntaxes
            .find_syntax_by_name("Rust")
            .unwrap_or_else(|| syntaxes.find_syntax_plain_text());

        let theme = themes().themes.get("InspiredGitHub").unwrap();
        let highlighter = Highlighter::new(theme);

        SyntectHighlighter {
            syntax,
            highlighter,
        }
    }
}

impl super::Highlighter for SyntectHighlighter {
    type Error = ParsingError;

    fn highlight_text(
        &mut self,
        text: &str,
        push_token: &mut dyn FnMut(usize, Token),
    ) -> Result<(), Self::Error> {
        let syntaxes = syntaxes();

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
