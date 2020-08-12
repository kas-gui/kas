// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text processing
//!
//! The functionality here is deliberately a quick hack to get things working.
//! Hopefully it can be replaced with a real mark-up processor without too
//! much API breakage.

use smallvec::smallvec;
use std::ops::Deref;

use kas::event::{VirtualKeyCode as VK, VirtualKeyCodes};

/// A label string
///
/// This is a label which supports markup.
///
/// Markup: `&&` translates to `&`; `&x` for any `x` translates to `x`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LabelString {
    label: String,
}

impl LabelString {
    /// Parse a `&str`
    ///
    /// Since we require `'static` for references and don't yet have
    /// specialisation, this parser always allocates. Prefer to use `from`.
    pub fn parse(mut s: &str) -> Self {
        let mut buf = String::with_capacity(s.len());
        while let Some(mut i) = s.find("&") {
            buf.push_str(&s[..i]);
            i += "&".len();
            s = &s[i..];

            match s.chars().next() {
                None => {
                    // Ending with '&' is an error, but we can ignore it
                    s = &s[0..0];
                    break;
                }
                Some(c) => {
                    buf.push(c);
                    let i = c.len_utf8();
                    s = &s[i..];
                }
            }
        }
        buf.push_str(s);
        LabelString { label: buf.into() }
    }
}

impl From<String> for LabelString {
    fn from(input: String) -> Self {
        if input.as_bytes().contains(&b'&') {
            Self::parse(&input)
        } else {
            // fast path: we can use the raw input
            LabelString { label: input }
        }
    }
}

impl From<&'static str> for LabelString {
    fn from(input: &'static str) -> Self {
        input.to_string().into()
    }
}

impl Deref for LabelString {
    type Target = str;
    fn deref(&self) -> &str {
        &self.label
    }
}

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
    underline: usize, // glyph to underline
    // TODO: is it worth using such a large structure here instead of Option?
    keys: VirtualKeyCodes,
}

impl AccelString {
    /// Parse a `&str`
    ///
    /// Since we require `'static` for references and don't yet have
    /// specialisation, this parser always allocates. Prefer to use `from`.
    pub fn parse(mut s: &str) -> Self {
        let mut buf = String::with_capacity(s.len());
        let mut count = 0;
        let mut underline = usize::MAX;
        let mut keys = VirtualKeyCodes::new();

        while let Some(mut i) = s.find("&") {
            count += 1;
            buf.push_str(&s[..i]);
            i += "&".len();
            s = &s[i..];

            match s.char_indices().next() {
                None => {
                    // Ending with '&' is an error, but we can ignore it
                    s = &s[0..0];
                    break;
                }
                Some((j, c)) => {
                    buf.push(c);
                    underline = i + j - count;
                    let vkeys = find_vkeys(c);
                    if !vkeys.is_empty() {
                        keys.extend(vkeys);
                    }
                    let i = c.len_utf8();
                    s = &s[i..];
                }
            }
        }
        buf.push_str(s);
        AccelString {
            label: buf.into(),
            underline,
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
        self.underline
    }
}

impl From<String> for AccelString {
    fn from(input: String) -> Self {
        if input.as_bytes().contains(&b'&') {
            Self::parse(&input)
        } else {
            // fast path: we can use the raw input
            AccelString {
                label: input.clone(),
                underline: usize::MAX,
                keys: Default::default(),
            }
        }
    }
}

impl From<&'static str> for AccelString {
    fn from(input: &'static str) -> Self {
        input.to_string().into()
    }
}

fn find_vkeys(c: char) -> VirtualKeyCodes {
    // TODO: lots of keys aren't yet available in VirtualKeyCode!
    // NOTE: some of these bindings are a little inaccurate. It isn't obvious
    // whether prefer strict or more flexible bindings here.
    // NOTE: we add a couple of non-unicode bindings. How many should we add?
    match c.to_ascii_uppercase() {
        '\'' => smallvec![VK::Apostrophe],
        '+' => smallvec![VK::Add],
        ',' => smallvec![VK::Comma],
        '-' => smallvec![VK::Minus],
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
        '÷' => smallvec![VK::Divide],
        '×' => smallvec![VK::Multiply],
        '−' => smallvec![VK::Subtract],
        _ => smallvec![],
    }
}
