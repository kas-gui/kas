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

use kas::event::{VirtualKeyCode as VK, VirtualKeyCodes};
use kas::text::format::{FontToken, FormattableText};
use kas::text::{fonts::FontId, Environment};

/// An accelerator key string
///
/// This is a label which supports highlighting of accelerator keys.
///
/// Markup: `&&` translates to `&`; `&x` for any `x` translates to `x` and
/// identifies `x` as an "accelerator key"; this may be drawn underlined and
/// may support keyboard access via e.g. `Alt+X`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AccelString {
    label: String,
    /// Even entries: position to start underline; odd entries: stop pos
    ulines: SmallVec<[u32; 2]>,
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
        let mut ulines = SmallVec::<[u32; 2]>::default();
        let mut keys = VirtualKeyCodes::new();

        while let Some(mut i) = s.find("&") {
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
                    let pos = buf.len() as u32;
                    buf.push(c);
                    ulines.push(pos);
                    let vkeys = find_vkeys(c);
                    if !vkeys.is_empty() {
                        keys.extend(vkeys);
                    }
                    let i = c.len_utf8();
                    s = &s[i..];

                    if let Some((k, _)) = chars.next() {
                        ulines.push(pos + (k - j) as u32);
                    }
                }
            }
        }
        buf.push_str(s);
        AccelString {
            label: buf.into(),
            ulines,
            keys,
        }
    }

    /// Get the key bindings
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

    /// Get the glyph to be underlined
    pub fn underline(&self) -> usize {
        // TODO: this is not the intended way to pass on this information!
        self.ulines
            .get(0)
            .map(|pos| *pos as usize)
            .unwrap_or(usize::MAX)
    }
}

impl FormattableText for AccelString {
    #[inline]
    fn clone_boxed(&self) -> Box<dyn FormattableText> {
        Box::new(self.clone())
    }

    #[inline]
    fn as_str(&self) -> &str {
        &self.label
    }

    #[inline]
    fn font_tokens<'a>(&'a self, env: &'a Environment) -> Box<dyn Iterator<Item = FontToken> + 'a> {
        Box::new(UlinesIter::new(&self.ulines, env))
    }
}

pub struct UlinesIter<'a> {
    index: usize,
    ulines: &'a [u32],
    dpem: f32,
}

impl<'a> UlinesIter<'a> {
    fn new(ulines: &'a [u32], env: &Environment) -> Self {
        UlinesIter {
            index: 0,
            ulines,
            dpem: env.dpp * env.pt_size,
        }
    }
}

impl<'a> Iterator for UlinesIter<'a> {
    type Item = FontToken;

    fn next(&mut self) -> Option<FontToken> {
        if self.index < self.ulines.len() {
            // TODO: if index is even, this starts an underline; if odd, it ends one
            let pos = self.ulines[self.index];
            self.index += 1;
            Some(FontToken {
                start: pos,
                font_id: FontId::default(),
                dpem: self.dpem,
            })
        } else {
            None
        }
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

impl From<&'static str> for AccelString {
    fn from(input: &'static str) -> Self {
        Self::parse(input)
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
