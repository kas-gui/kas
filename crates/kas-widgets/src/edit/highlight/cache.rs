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
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Cache {
    fonts: Vec<Fmt>,
    colors: Vec<(u32, Colors)>,
    decorations: Vec<(u32, Decoration)>,
}

impl Default for Cache {
    #[inline]
    fn default() -> Self {
        Cache {
            fonts: vec![Fmt::default()],
            colors: vec![],
            decorations: vec![],
        }
    }
}

impl Cache {
    /// Highlight a whole `text`, returning errors
    pub fn try_highlight<H: Highlighter>(
        &mut self,
        text: &str,
        highlighter: &mut H,
    ) -> Result<(), H::Error> {
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

        highlighter.highlight_text(text, &mut push_token)
    }

    /// Highlight a whole `text`, logging errors
    pub fn highlight<H: Highlighter>(&mut self, text: &str, highlighter: &mut H) {
        if let Err(err) = self.try_highlight(text, highlighter) {
            log::error!("Highlighting failed: {err}");
            debug_assert!(false, "Highlighter: {err}");
        }
    }

    pub fn font_tokens(&self, dpem: f32, font: FontSelector) -> impl Iterator<Item = FontToken> {
        self.fonts.iter().map(move |fmt| FontToken {
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
