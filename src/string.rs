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
                    bufu.push(underline);
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
        }
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
