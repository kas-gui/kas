// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text processing
//!
//! The functionality here is deliberately a quick hack to get things working.
//! Hopefully it can be replaced with a real mark-up processor without too
//! much API breakage.

use smallvec::SmallVec;

use crate::cast::Conv;
use crate::event::Key;
use crate::text::format::{FontToken, FormattableText};
use crate::text::{Effect, EffectFlags};

/// An access key string
///
/// This is a label which supports highlighting of access keys (sometimes called
/// "mnemonics"). This type represents both the
/// displayed text (via [`FormattableText`] implementation)
/// and the shortcut (via [`AccessString::key`]).
///
/// Markup: `&&` translates to `&`; `&x` for any `x` translates to `x` and
/// identifies `x` as an "access key"; this may be drawn underlined and
/// may support keyboard access via e.g. `Alt+X`
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccessString {
    text: String,
    effects: SmallVec<[Effect<()>; 2]>,
    key: Option<Key>,
}

impl AccessString {
    /// Parse a `&str`
    ///
    /// Since we require `'static` for references and don't yet have
    /// specialisation, this parser always allocates. Prefer to use `from`.
    fn parse(mut s: &str) -> Self {
        let mut buf = String::with_capacity(s.len());
        let mut effects = SmallVec::<[Effect<()>; 2]>::default();
        let mut key = None;

        while let Some(mut i) = s.find('&') {
            buf.push_str(&s[..i]);
            i += "&".len();
            s = &s[i..];
            let mut chars = s.char_indices();

            match chars.next() {
                None => {
                    // Ending with '&' is an error, but we can ignore it
                    s = &s[0..0];
                    break;
                }
                Some((j, c)) => {
                    // TODO(opt): we can simplify if we don't support multiple mnemonic keys
                    let pos = u32::conv(buf.len());
                    buf.push(c);
                    if effects.last().map(|e| e.start == pos).unwrap_or(false) {
                        effects.last_mut().unwrap().flags = EffectFlags::UNDERLINE;
                    } else {
                        effects.push(Effect {
                            start: pos,
                            flags: EffectFlags::UNDERLINE,
                            aux: (),
                        });
                    }
                    if key.is_none() {
                        let mut kbuf = [0u8; 4];
                        let s = c.to_ascii_lowercase().encode_utf8(&mut kbuf);
                        key = Some(Key::Character(s.into()));
                    }
                    let i = c.len_utf8();
                    s = &s[i..];

                    if let Some((k, _)) = chars.next() {
                        effects.push(Effect {
                            start: pos + u32::conv(k - j),
                            flags: EffectFlags::empty(),
                            aux: (),
                        });
                    }
                }
            }
        }
        buf.push_str(s);
        AccessString {
            text: buf,
            effects,
            key,
        }
    }

    /// Get the key binding, if any
    pub fn key(&self) -> Option<Key> {
        self.key.clone()
    }

    /// Get the text
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl FormattableText for AccessString {
    type FontTokenIter<'a> = std::iter::Empty<FontToken>;

    #[inline]
    fn as_str(&self) -> &str {
        &self.text
    }

    #[inline]
    fn font_tokens(&self, _: f32) -> Self::FontTokenIter<'_> {
        std::iter::empty()
    }

    fn effect_tokens(&self) -> &[Effect<()>] {
        &self.effects
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
