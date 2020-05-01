// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text processing
//!
//! The functionality here is deliberately a quick hack to get things working.
//! Hopefully it can be replaced with a real mark-up processor without too
//! much API breakage.

use std::ops::Deref;

use kas::event::{VirtualKeyCode, VirtualKeyCodes};

/// Convenience definition: `Cow<'a, str>`
pub type CowStringL<'a> = std::borrow::Cow<'a, str>;

/// Convenience definition: `Cow<'static, str>`
pub type CowString = CowStringL<'static>;

/// A label string
///
/// This is a label which supports markup.
///
/// Markup: `&&` translates to `&`; `&x` for any `x` translates to `x`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LabelString {
    label: CowString,
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

impl From<CowString> for LabelString {
    fn from(input: CowString) -> Self {
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
        CowString::from(input).into()
    }
}

impl From<String> for LabelString {
    fn from(input: String) -> Self {
        CowString::from(input).into()
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
    label: CowString,
    underlined: CowString,
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
        // NOTE: our text display doesn't yet support
        // underlining, but Unicode diacritics are good enough.
        let underline = '\u{0332}';
        let mut bufu = String::with_capacity(s.len() + underline.len_utf8() - "&".len());
        let mut keys = VirtualKeyCodes::new();

        while let Some(mut i) = s.find("&") {
            buf.push_str(&s[..i]);
            bufu.push_str(&s[..i]);
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
                    bufu.push(c);
                    if let Some(key) = find_vkey(c) {
                        bufu.push(underline);
                        keys.push(key);
                    }
                    let i = c.len_utf8();
                    s = &s[i..];
                }
            }
        }
        buf.push_str(s);
        bufu.push_str(s);
        AccelString {
            label: buf.into(),
            underlined: bufu.into(),
            keys,
        }
    }

    /// Get the key bindings
    pub fn keys(&self) -> &[VirtualKeyCode] {
        &self.keys
    }
}

impl From<CowString> for AccelString {
    fn from(input: CowString) -> Self {
        if input.as_bytes().contains(&b'&') {
            Self::parse(&input)
        } else {
            // fast path: we can use the raw input
            AccelString {
                label: input.clone(),
                underlined: input,
                keys: Default::default(),
            }
        }
    }
}

impl From<&'static str> for AccelString {
    fn from(input: &'static str) -> Self {
        CowString::from(input).into()
    }
}

impl From<String> for AccelString {
    fn from(input: String) -> Self {
        CowString::from(input).into()
    }
}

impl Deref for AccelString {
    type Target = str;
    fn deref(&self) -> &str {
        &self.underlined // TODO
    }
}

fn find_vkey(c: char) -> Option<VirtualKeyCode> {
    Some(match c.to_ascii_uppercase() {
        '0' => VirtualKeyCode::Key0,
        '1' => VirtualKeyCode::Key1,
        '2' => VirtualKeyCode::Key2,
        '3' => VirtualKeyCode::Key3,
        '4' => VirtualKeyCode::Key4,
        '5' => VirtualKeyCode::Key5,
        '6' => VirtualKeyCode::Key6,
        '7' => VirtualKeyCode::Key7,
        '8' => VirtualKeyCode::Key8,
        '9' => VirtualKeyCode::Key9,
        'A' => VirtualKeyCode::A,
        'B' => VirtualKeyCode::B,
        'C' => VirtualKeyCode::C,
        'D' => VirtualKeyCode::D,
        'E' => VirtualKeyCode::E,
        'F' => VirtualKeyCode::F,
        'G' => VirtualKeyCode::G,
        'H' => VirtualKeyCode::H,
        'I' => VirtualKeyCode::I,
        'J' => VirtualKeyCode::J,
        'K' => VirtualKeyCode::K,
        'L' => VirtualKeyCode::L,
        'M' => VirtualKeyCode::M,
        'N' => VirtualKeyCode::N,
        'O' => VirtualKeyCode::O,
        'P' => VirtualKeyCode::P,
        'Q' => VirtualKeyCode::Q,
        'R' => VirtualKeyCode::R,
        'S' => VirtualKeyCode::S,
        'T' => VirtualKeyCode::T,
        'U' => VirtualKeyCode::U,
        'V' => VirtualKeyCode::V,
        'W' => VirtualKeyCode::W,
        'X' => VirtualKeyCode::X,
        'Y' => VirtualKeyCode::Y,
        'Z' => VirtualKeyCode::Z,
        _ => return None,
    })
}
