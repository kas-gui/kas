// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text processing
//!
//! The functionality here is deliberately a quick hack to get things working.
//! Hopefully it can be replaced with a real mark-up processor without too
//! much API breakage.

use crate::cast::Conv;
use crate::event::Key;
use crate::text::format::{FontToken, FormattableText};
use crate::text::{Effect, EffectFlags, fonts::FontSelector};

/// An access key string
///
/// This is a label which supports highlighting of access keys (sometimes called
/// "mnemonics").
///
/// Drawing this text using the inherent [`FormattableText`] implementation will
/// not underline the access key. To do that, use the effect tokens returned by
/// [`Self::key`].
///
/// Markup: `&&` translates to `&`; `&x` for any `x` translates to `x` and
/// identifies `x` as an "access key"; this may be drawn underlined and
/// may support keyboard access via e.g. `Alt+X`
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccessString {
    text: String,
    key: Option<(Key, [(u32, Effect); 2])>,
}

impl AccessString {
    /// Parse a `&str`
    ///
    /// Since we require `'static` for references and don't yet have
    /// specialisation, this parser always allocates. Prefer to use `from`.
    fn parse(mut s: &str) -> Self {
        let mut text = String::with_capacity(s.len());
        let mut key = None;

        while let Some(mut i) = s.find('&') {
            text.push_str(&s[..i]);
            i += "&".len();
            s = &s[i..];

            match s.chars().next() {
                None => {
                    // Ending with '&' is an error, but we can ignore it
                    s = &s[0..0];
                    break;
                }
                Some(c) if key.is_none() => {
                    let start = u32::conv(text.len());
                    text.push(c);

                    let mut kbuf = [0u8; 4];
                    let k = c.to_ascii_lowercase().encode_utf8(&mut kbuf);
                    let k = Key::Character(k.into());

                    let e0 = (start, Effect {
                        color: 0,
                        flags: EffectFlags::UNDERLINE,
                    });

                    let i = c.len_utf8();
                    s = &s[i..];

                    let e1 = (start + u32::conv(i), Effect {
                        color: 0,
                        flags: EffectFlags::empty(),
                    });

                    key = Some((k, [e0, e1]));
                }
                Some(c) => {
                    text.push(c);
                    let i = c.len_utf8();
                    s = &s[i..];
                }
            }
        }

        text.push_str(s);
        AccessString { text, key }
    }

    /// Get the key bindings and associated effects, if any
    pub fn key(&self) -> Option<&(Key, [(u32, Effect); 2])> {
        self.key.as_ref()
    }

    /// Get the text
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl FormattableText for AccessString {
    #[inline]
    fn as_str(&self) -> &str {
        &self.text
    }

    #[inline]
    fn font_tokens(&self, dpem: f32, font: FontSelector) -> impl Iterator<Item = FontToken> {
        std::iter::once(FontToken {
            start: 0,
            dpem,
            font,
        })
    }

    fn effect_tokens(&self) -> &[(u32, Effect)] {
        &[]
    }
}

impl From<String> for AccessString {
    fn from(text: String) -> Self {
        if text.as_bytes().contains(&b'&') {
            Self::parse(&text)
        } else {
            // fast path: we can use the raw input
            AccessString {
                text,
                ..Default::default()
            }
        }
    }
}

impl From<&str> for AccessString {
    fn from(input: &str) -> Self {
        Self::parse(input)
    }
}

impl<T: Into<AccessString> + Copy> From<&T> for AccessString {
    fn from(input: &T) -> Self {
        (*input).into()
    }
}
