// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Colour types

use crate::cast::{Conv, ConvFloat};

/// 4-part colour data, linear, sRGB colour space
///
/// Linear format must be used for colour data uploaded via uniforms or vertex
/// lists. Conversion from sRGB happens in user-space.
///
/// This is the expected type for shader inputs, encoded as four `f32` values
/// in RGBA order.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    /// Transparent black
    pub const TRANSPARENT: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);
    /// Opaque white
    pub const WHITE: Self = Self::grey(1.0);
    /// Opaque black
    pub const BLACK: Self = Self::grey(0.0);

    /// Construct from R-G-B-A components
    ///
    /// Values should be between 0 and 1 on a linear scale.
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Construct from opaque R-G-B
    ///
    /// Values should be between 0 and 1 on a linear scale.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Construct from grey-scale
    pub const fn grey(s: f32) -> Self {
        Self::rgb(s, s, s)
    }
}

impl From<Rgba> for [f32; 4] {
    fn from(c: Rgba) -> Self {
        [c.r, c.g, c.b, c.a]
    }
}

impl From<[f32; 4]> for Rgba {
    fn from(c: [f32; 4]) -> Self {
        Self::rgba(c[0], c[1], c[2], c[3])
    }
}

/// 8-bit-per-channel sRGB colour + alpha component
///
/// This is a common format for inputs (alpha component defaults to opaque).
///
/// This type may be parsed from a string:
/// ```
/// use kas::draw::color::Rgba8Srgb;
///
/// let red: Rgba8Srgb = "#FF0000".parse().unwrap();
/// assert_eq!(red, Rgba8Srgb::rgb(255, 0, 0));
///
/// // The leading `#` is optional and lowercase is fine
/// let green: Rgba8Srgb = "00ff00".parse().unwrap();
/// assert_eq!(green, Rgba8Srgb::rgb(0, 255, 0));
///
/// // An optional fourth byte is interpreted as alpha component
/// let translucent_blue: Rgba8Srgb = "0000FF7F".parse().unwrap();
/// assert_eq!(translucent_blue, Rgba8Srgb::rgba(0, 0, 255, 127));
/// ```
///
/// This is incoded as an array of four bytes: `[r, g, b, a]`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Rgba8Srgb([u8; 4]);

impl Rgba8Srgb {
    /// Transparent black
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    /// Opaque white
    pub const WHITE: Self = Self::grey(255);
    /// Opaque black
    pub const BLACK: Self = Self::grey(0);

    /// Construct from R-G-B-A components
    ///
    /// Values should be between 0 and 255 with sRGB gamma scaling.
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self([r, g, b, a])
    }

    /// Construct from opaque R-G-B
    ///
    /// Values should be between 0 and 1 on a linear scale.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b, 255])
    }

    /// Construct from grey-scale
    pub const fn grey(s: u8) -> Self {
        Self::rgb(s, s, s)
    }
}

impl From<Rgba8Srgb> for [u8; 4] {
    fn from(c: Rgba8Srgb) -> Self {
        [c.0[0], c.0[1], c.0[2], c.0[2]]
    }
}

impl From<[u8; 4]> for Rgba8Srgb {
    fn from(c: [u8; 4]) -> Self {
        Self::rgba(c[0], c[1], c[2], c[3])
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ParseError {
    /// Incorrect input length
    Length,
    /// Invalid hex byte
    InvalidHex,
}

/// Parse sRGB colour designator from a string
///
/// Accepts:
///
/// -   optional `#` prefix
/// -   upper and lower case hex digits
/// -   six (RGB) or eight (RGBA) digits
impl std::str::FromStr for Rgba8Srgb {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = s.as_bytes();
        if s[0] == b'#' {
            s = &s[1..];
        }
        if s.len() != 6 && s.len() != 8 {
            return Err(ParseError::Length);
        }

        // `val` is copied from the hex crate:
        // Copyright (c) 2013-2014 The Rust Project Developers.
        // Copyright (c) 2015-2020 The rust-hex Developers.
        fn val(c: u8) -> Result<u8, ParseError> {
            match c {
                b'A'..=b'F' => Ok(c - b'A' + 10),
                b'a'..=b'f' => Ok(c - b'a' + 10),
                b'0'..=b'9' => Ok(c - b'0'),
                _ => Err(ParseError::InvalidHex),
            }
        }

        fn byte(s: &[u8]) -> Result<u8, ParseError> {
            Ok(val(s[0])? << 4 | val(s[1])?)
        }

        let r = byte(&s[0..2])?;
        let g = byte(&s[2..4])?;
        let b = byte(&s[4..6])?;
        let a = if s.len() == 8 { byte(&s[6..8])? } else { 0xFF };

        Ok(Rgba8Srgb([r, g, b, a]))
    }
}

// Linear conversions are copied from the palette crate, and adapted to remove
// float generics and include byte to/from float conversion.
fn into_linear(x: u8) -> f32 {
    let x = f32::conv(x) * (1.0 / 255.0);
    // Recip call shows performance benefits in benchmarks for this function
    if x <= 0.04045 {
        x * (1.0 / 12.92)
    } else {
        ((x + 0.055) * (1.0 / 1.055)).powf(2.4)
    }
}

fn from_linear(x: f32) -> u8 {
    let x = if x <= 0.0031308 {
        x * 12.92
    } else {
        x.powf(1.0 / 2.4) * 1.055 - 0.055
    };
    u8::conv_nearest(x * 255.0)
}

impl From<Rgba8Srgb> for Rgba {
    fn from(c: Rgba8Srgb) -> Self {
        // We are still working in the sRGB colour space, so the white point is the same.
        Rgba {
            r: into_linear(c.0[0]),
            g: into_linear(c.0[1]),
            b: into_linear(c.0[2]),
            a: f32::conv(c.0[3]) * (1.0 / 255.0),
        }
    }
}

impl From<Rgba> for Rgba8Srgb {
    fn from(c: Rgba) -> Self {
        Rgba8Srgb([
            from_linear(c.r),
            from_linear(c.g),
            from_linear(c.b),
            u8::conv_nearest(c.a * 255.0),
        ])
    }
}
