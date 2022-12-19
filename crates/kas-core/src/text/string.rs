// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text processing
//!
//! The functionality here is deliberately a quick hack to get things working.
//! Hopefully it can be replaced with a real mark-up processor without too
//! much API breakage.

use smallvec::{smallvec, SmallVec};

use crate::cast::Conv;
use crate::event::{VirtualKeyCode as VK, VirtualKeyCodes};
use crate::text::format::{FontToken, FormattableText};
use crate::text::{Effect, EffectFlags};

/// An accelerator key string
///
/// This is a label which supports highlighting of accelerator keys (elsewhere
/// called "access keys" or "mnemonics"). This type represents both the
/// displayed text (via [`FormattableText`] implementation)
/// and the shortcut (via [`AccelString::keys`]).
///
/// Markup: `&&` translates to `&`; `&x` for any `x` translates to `x` and
/// identifies `x` as an "accelerator key"; this may be drawn underlined and
/// may support keyboard access via e.g. `Alt+X`
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccelString {
    label: String,
    effects: SmallVec<[Effect<()>; 2]>,
    // TODO: is it worth using such a large structure here instead of Option?
    keys: VirtualKeyCodes,
}

impl AccelString {
    /// Parse a `&str`
    ///
    /// Since we require `'static` for references and don't yet have
    /// specialisation, this parser always allocates. Prefer to use `from`.
    fn parse(mut s: &str) -> Self {
        let mut buf = String::with_capacity(s.len());
        let mut effects = SmallVec::<[Effect<()>; 2]>::default();
        let mut keys = VirtualKeyCodes::new();

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
                    let vkeys = find_vkeys(c);
                    if !vkeys.is_empty() {
                        keys.extend(vkeys);
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
        AccelString {
            label: buf,
            effects,
            keys,
        }
    }

    /// Get the key bindings
    ///
    /// Usually this list has length zero or one, but nothing prevents the use
    /// multiple mnemonic key bindings.
    pub fn keys(&self) -> &[VK] {
        &self.keys
    }

    /// Take the key bindings, destroying self
    pub fn take_keys(self) -> VirtualKeyCodes {
        self.keys
    }

    /// Get the text
    pub fn text(&self) -> &str {
        &self.label
    }
}

impl FormattableText for AccelString {
    type FontTokenIter<'a> = std::iter::Empty<FontToken>;

    #[inline]
    fn as_str(&self) -> &str {
        &self.label
    }

    #[inline]
    fn font_tokens(&self, _: f32) -> Self::FontTokenIter<'_> {
        std::iter::empty()
    }

    fn effect_tokens(&self) -> &[Effect<()>] {
        &self.effects
    }
}

impl From<String> for AccelString {
    fn from(input: String) -> Self {
        if input.as_bytes().contains(&b'&') {
            Self::parse(&input)
        } else {
            // fast path: we can use the raw input
            AccelString {
                label: input,
                ..Default::default()
            }
        }
    }
}

impl From<&str> for AccelString {
    fn from(input: &str) -> Self {
        Self::parse(input)
    }
}

impl<T: Into<AccelString> + Copy> From<&T> for AccelString {
    fn from(input: &T) -> Self {
        (*input).into()
    }
}

fn find_vkeys(c: char) -> VirtualKeyCodes {
    // TODO: lots of keys aren't yet available in VirtualKeyCode!
    // NOTE: some of these bindings are a little inaccurate. It isn't obvious
    // whether prefer strict or more flexible bindings here.
    match c.to_ascii_uppercase() {
        '\'' => smallvec![VK::Apostrophe],
        '+' => smallvec![VK::Plus, VK::NumpadAdd],
        ',' => smallvec![VK::Comma],
        '-' => smallvec![VK::Minus, VK::NumpadSubtract],
        '.' => smallvec![VK::Period],
        '/' => smallvec![VK::Slash],
        '0' => smallvec![VK::Key0, VK::Numpad0],
        '1' => smallvec![VK::Key1, VK::Numpad1],
        '2' => smallvec![VK::Key2, VK::Numpad2],
        '3' => smallvec![VK::Key3, VK::Numpad3],
        '4' => smallvec![VK::Key4, VK::Numpad4],
        '5' => smallvec![VK::Key5, VK::Numpad5],
        '6' => smallvec![VK::Key6, VK::Numpad6],
        '7' => smallvec![VK::Key7, VK::Numpad7],
        '8' => smallvec![VK::Key8, VK::Numpad8],
        '9' => smallvec![VK::Key9, VK::Numpad9],
        ':' => smallvec![VK::Colon],
        ';' => smallvec![VK::Semicolon],
        '=' => smallvec![VK::Equals, VK::NumpadEquals],
        '`' => smallvec![VK::Grave],
        'A' => smallvec![VK::A],
        'B' => smallvec![VK::B],
        'C' => smallvec![VK::C],
        'D' => smallvec![VK::D],
        'E' => smallvec![VK::E],
        'F' => smallvec![VK::F],
        'G' => smallvec![VK::G],
        'H' => smallvec![VK::H],
        'I' => smallvec![VK::I],
        'J' => smallvec![VK::J],
        'K' => smallvec![VK::K],
        'L' => smallvec![VK::L],
        'M' => smallvec![VK::M],
        'N' => smallvec![VK::N],
        'O' => smallvec![VK::O],
        'P' => smallvec![VK::P],
        'Q' => smallvec![VK::Q],
        'R' => smallvec![VK::R],
        'S' => smallvec![VK::S],
        'T' => smallvec![VK::T],
        'U' => smallvec![VK::U],
        'V' => smallvec![VK::V],
        'W' => smallvec![VK::W],
        'X' => smallvec![VK::X],
        'Y' => smallvec![VK::Y],
        'Z' => smallvec![VK::Z],
        '[' => smallvec![VK::LBracket],
        ']' => smallvec![VK::RBracket],
        '^' => smallvec![VK::Caret],
        '÷' => smallvec![VK::NumpadDivide],
        '×' => smallvec![VK::NumpadMultiply],
        '−' => smallvec![VK::Minus, VK::NumpadSubtract],
        _ => smallvec![],
    }
}
