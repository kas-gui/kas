// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Supporting elements for syntax highlighting

use super::*;
use kas::cast::Cast;
use kas::text::fonts::{FontSelector, FontStyle, FontWeight};
use kas::text::format::{Colors, Decoration, FontToken};

#[derive(Clone, Debug, Default, PartialEq)]
struct Fmt {
    start: u32,
    weight: FontWeight,
    style: FontStyle,
}

/// A highlighted text
///
/// Two `Text` objects compare equal if their formatted text is equal regardless
/// of the embedded highlighter.
#[derive(Clone, Debug)]
#[kas::autoimpl(PartialEq ignore self.highlighter)]
pub struct Text<H: Highlighter> {
    highlighter: H,
    fonts: Vec<Fmt>,
    colors: Vec<(u32, Colors)>,
    decorations: Vec<(u32, Decoration)>,
}

impl<H: Highlighter + Default> Default for Text<H> {
    fn default() -> Self {
        Self::new(H::default())
    }
}

impl<H: Highlighter> Text<H> {
    /// Construct a new instance
    #[inline]
    pub fn new(highlighter: H) -> Self {
        Text {
            highlighter,
            fonts: vec![Fmt::default()],
            colors: vec![],
            decorations: vec![],
        }
    }

    /// Configure the highlighter
    ///
    /// This is called when the widget is configured. It may be used to set the
    /// theme / color scheme.
    ///
    /// Returns `true` when the highlighter must be re-run.
    #[inline]
    #[must_use]
    pub fn configure(&mut self, cx: &mut ConfigCx) -> bool {
        self.highlighter.configure(cx)
    }

    /// Get scheme colors
    ///
    /// This method allows usage of the highlighter's colors by the editor.
    #[inline]
    pub fn scheme_colors(&self) -> SchemeColors {
        self.highlighter.scheme_colors()
    }

    /// Highlight the text (from scratch)
    pub fn highlight(&mut self, text: &str) {
        self.fonts.clear();
        self.fonts.push(Fmt::default());
        self.colors.clear();
        self.decorations.clear();

        let mut last_index = None;
        let mut state = Token::default();
        let mut push_token = |index: usize, token: Token| {
            if let Some(last) = last_index
                && index <= last
            {
                log::error!("Highlighting failed: token start indices are not strictly increasing");
                debug_assert!(false, "Highlighter: token start index order");
                return;
            }

            if token.weight != state.weight || token.style != state.style {
                if index == 0 {
                    self.fonts.clear();
                }

                self.fonts.push(Fmt {
                    start: index.cast(),
                    weight: token.weight,
                    style: token.style,
                });
            }

            if token.colors != state.colors {
                self.colors.push((index.cast(), token.colors));
            }

            if token.decoration != state.decoration {
                self.decorations.push((index.cast(), token.decoration));
            }

            last_index = Some(index);
            state = token;
        };

        if let Err(err) = self.highlighter.highlight_text(text, &mut push_token) {
            log::error!("Highlighting failed: {err}");
            debug_assert!(false, "Highlighter: {err}");
        }
    }

    pub fn font_tokens(&self, dpem: f32, font: FontSelector) -> impl Iterator<Item = FontToken> {
        self.fonts.iter().cloned().map(move |fmt| FontToken {
            start: fmt.start,
            dpem,
            font: FontSelector {
                family: font.family,
                weight: fmt.weight,
                width: font.width,
                style: fmt.style,
            },
        })
    }

    /// The default implementation returns `&[]`.
    #[inline]
    pub fn color_tokens(&self) -> &[(u32, Colors)] {
        &self.colors
    }

    #[inline]
    pub fn decorations(&self) -> &[(u32, Decoration)] {
        &self.decorations
    }
}
