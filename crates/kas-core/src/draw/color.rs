// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Colour types

#![allow(clippy::self_named_constructors)]

use std::ops::Add;

use crate::cast::{Conv, ConvFloat};
use thiserror::Error;

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

    /// Construct from grey-scale with alpha
    pub const fn ga(s: f32, a: f32) -> Self {
        Self::rgba(s, s, s, a)
    }

    /// Convert to [`Rgb`]
    pub const fn as_rgb(self) -> Rgb {
        let Rgba { r, g, b, .. } = self;
        Rgb { r, g, b }
    }

    /// Get the sum of the three colour components
    pub fn sum(self) -> f32 {
        self.r + self.g + self.b
    }

    /// Average three colour components (fully desaturate)
    #[must_use = "method does not modify self but returns a new value"]
    pub fn average(self) -> Self {
        Self::ga(self.sum() * (1.0 / 3.0), self.a)
    }

    /// Multiply and clamp three colour components
    ///
    /// Values outside the range `0..=1` could in theory be used but may result
    /// in components exceeding their valid range; be sure to call
    /// [`Self::clamp_to_01`] on the result in this case.
    #[must_use = "method does not modify self but returns a new value"]
    pub fn multiply(self, x: f32) -> Self {
        debug_assert!(x >= 0.0);
        Self {
            r: (self.r * x).min(1.0),
            g: (self.g * x).min(1.0),
            b: (self.b * x).min(1.0),
            a: self.a,
        }
    }

    /// Partially desaturate
    ///
    /// `x` should be in the range `0..=1` where 0 implies no desaturation and
    /// 1 implies full desaturation.
    ///
    /// Values outside the range `0..=1` could in theory be used but may result
    /// in components exceeding their valid range; be sure to call
    /// [`Self::clamp_to_01`] on the result in this case.
    #[must_use = "method does not modify self but returns a new value"]
    pub fn desaturate(self, x: f32) -> Self {
        self.as_rgb().desaturate(x).with_alpha(self.a)
    }

    /// Clamp each colour component to no more than `v`
    #[must_use = "method does not modify self but returns a new value"]
    pub fn min(self, v: f32) -> Self {
        debug_assert!(v >= 0.0);
        Self {
            r: self.r.min(v),
            g: self.g.min(v),
            b: self.b.min(v),
            a: self.a,
        }
    }

    /// Clamp each colour component to at least `v`
    #[must_use = "method does not modify self but returns a new value"]
    pub fn max(self, v: f32) -> Self {
        debug_assert!(v <= 1.0);
        Self {
            r: self.r.max(v),
            g: self.g.max(v),
            b: self.b.max(v),
            a: self.a,
        }
    }

    /// Clamp each colour component to `u..=v`
    #[must_use = "method does not modify self but returns a new value"]
    pub fn clamp(self, u: f32, v: f32) -> Self {
        debug_assert!(u <= v);
        self.min(v).max(u)
    }

    /// Clamp each component to `0..=1`
    #[must_use = "method does not modify self but returns a new value"]
    pub fn clamp_to_01(self) -> Self {
        self.as_rgb()
            .clamp(0.0, 1.0)
            .with_alpha(self.a.clamp(0.0, 1.0))
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

/// 3-part colour data, linear, sRGB colour space
///
/// Linear format must be used for colour data uploaded via uniforms or vertex
/// lists. Conversion from sRGB happens in user-space.
///
/// This is the expected type for shader inputs, encoded as three `f32` values
/// in RGB order.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Rgb {
    /// Opaque white
    pub const WHITE: Self = Self::grey(1.0);
    /// Opaque black
    pub const BLACK: Self = Self::grey(0.0);

    /// Construct from R-G-B components
    ///
    /// Values should be between 0 and 1 on a linear scale.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

    /// Construct from grey-scale
    pub const fn grey(s: f32) -> Self {
        Self::rgb(s, s, s)
    }

    /// Add an alpha component
    pub const fn with_alpha(self, alpha: f32) -> Rgba {
        let Rgb { r, g, b } = self;
        Rgba { r, g, b, a: alpha }
    }

    /// Get the sum of the three colour components
    pub fn sum(self) -> f32 {
        self.r + self.g + self.b
    }

    /// Average three colour components (desaturate)
    #[must_use = "method does not modify self but returns a new value"]
    pub fn average(self) -> Self {
        Self::grey(self.sum() * (1.0 / 3.0))
    }

    /// Multiply and clamp three colour components
    ///
    /// Values outside the range `0..=1` could in theory be used but may result
    /// in components exceeding their valid range; be sure to call
    /// [`Self::clamp_to_01`] on the result in this case.
    #[must_use = "method does not modify self but returns a new value"]
    pub fn multiply(self, x: f32) -> Self {
        debug_assert!(x >= 0.0);
        Self {
            r: (self.r * x).min(1.0),
            g: (self.g * x).min(1.0),
            b: (self.b * x).min(1.0),
        }
    }

    /// Partially desaturate
    ///
    /// `x` should be in the range `0..=1` where 0 implies no desaturation and
    /// 1 implies full desaturation.
    ///
    /// Values outside the range `0..=1` could in theory be used but may result
    /// in components exceeding their valid range; be sure to call
    /// [`Self::clamp_to_01`] on the result in this case.
    #[must_use = "method does not modify self but returns a new value"]
    pub fn desaturate(self, x: f32) -> Self {
        self.multiply(1.0 - x) + self.average().multiply(x)
    }

    /// Clamp each colour component to no more than `v`
    #[must_use = "method does not modify self but returns a new value"]
    pub fn min(self, v: f32) -> Self {
        debug_assert!(v >= 0.0);
        Self {
            r: self.r.min(v),
            g: self.g.min(v),
            b: self.b.min(v),
        }
    }

    /// Clamp each colour component to at least `v`
    #[must_use = "method does not modify self but returns a new value"]
    pub fn max(self, v: f32) -> Self {
        debug_assert!(v <= 1.0);
        Self {
            r: self.r.max(v),
            g: self.g.max(v),
            b: self.b.max(v),
        }
    }

    /// Clamp each colour component to `u..=v`
    #[must_use = "method does not modify self but returns a new value"]
    pub fn clamp(self, u: f32, v: f32) -> Self {
        debug_assert!(u <= v);
        self.min(v).max(u)
    }

    /// Clamp each colour component to `0..=1`
    #[must_use = "method does not modify self but returns a new value"]
    pub fn clamp_to_01(self) -> Self {
        self.clamp(0.0, 1.0)
    }
}

impl From<Rgb> for [f32; 3] {
    fn from(c: Rgb) -> Self {
        [c.r, c.g, c.b]
    }
}

impl From<[f32; 3]> for Rgb {
    fn from(c: [f32; 3]) -> Self {
        Self::rgb(c[0], c[1], c[2])
    }
}

impl From<Rgb> for Rgba {
    fn from(c: Rgb) -> Self {
        Self::rgb(c.r, c.g, c.b)
    }
}

impl Add for Rgb {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Rgb {
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
        }
    }
}

/// 8-bit-per-channel sRGB colour + alpha component
///
/// This is a common format for inputs (alpha component defaults to opaque).
///
/// This type may be parsed from a string:
/// ```
/// use kas_core::draw::color::Rgba8Srgb;
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
pub struct Rgba8Srgb(pub [u8; 4]);

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

    /// Construct from grey-scale with alpha
    pub const fn ga(s: u8, a: u8) -> Self {
        Self::rgba(s, s, s, a)
    }

    /// Get the red component
    pub const fn r(self) -> u8 {
        self.0[0]
    }

    /// Get the green component
    pub const fn g(self) -> u8 {
        self.0[1]
    }

    /// Get the blue component
    pub const fn b(self) -> u8 {
        self.0[2]
    }

    /// Get the alpha component
    pub const fn a(self) -> u8 {
        self.0[3]
    }

    /// Format to a string
    ///
    /// This looks like `#123456` if the alpha component is opaque, otherwise
    /// like `#12345678`.
    pub fn format_html(self) -> String {
        if self.0[3] == 255 {
            format!("#{:02X}{:02X}{:02X}", self.0[0], self.0[1], self.0[2])
        } else {
            format!(
                "#{:02X}{:02X}{:02X}{:02X}",
                self.0[0], self.0[1], self.0[2], self.0[3]
            )
        }
    }

    /// Compile-time parser for sRGB and sRGBA colours
    pub const fn try_parse(s: &str) -> Result<Rgba8Srgb, ParseError> {
        let s = s.as_bytes();
        if s.len() != 6 && s.len() != 8 {
            return Err(ParseError::Length);
        }

        // `val` is copied from the hex crate:
        // Copyright (c) 2013-2014 The Rust Project Developers.
        // Copyright (c) 2015-2020 The rust-hex Developers.
        const fn val(c: u8) -> Result<u8, ()> {
            match c {
                b'A'..=b'F' => Ok(c - b'A' + 10),
                b'a'..=b'f' => Ok(c - b'a' + 10),
                b'0'..=b'9' => Ok(c - b'0'),
                _ => Err(()),
            }
        }

        const fn byte(a: u8, b: u8) -> Result<u8, ()> {
            match (val(a), val(b)) {
                (Ok(hi), Ok(lo)) => Ok((hi << 4) | lo),
                _ => Err(()),
            }
        }

        let r = byte(s[0], s[1]);
        let g = byte(s[2], s[3]);
        let b = byte(s[4], s[5]);
        let a = if s.len() == 8 { byte(s[6], s[7]) } else { Ok(0xFF) };

        match (r, g, b, a) {
            (Ok(r), Ok(g), Ok(b), Ok(a)) => Ok(Rgba8Srgb([r, g, b, a])),
            _ => Err(ParseError::InvalidHex),
        }
    }

    /// Compile-time parser for sRGB and sRGBA colours
    ///
    /// This method has worse diagnostics on error due to limited error handling in `const fn`.
    pub const fn parse(s: &str) -> Rgba8Srgb {
        match Self::try_parse(s) {
            Ok(result) => result,
            Err(ParseError::Length) => panic!("invalid length (expected 6 or 8 bytes)"),
            Err(ParseError::InvalidHex) => panic!("invalid hex byte (expected 0-9, a-f or A-F)"),
        }
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

#[derive(Copy, Clone, Debug, Error)]
pub enum ParseError {
    /// Incorrect input length
    #[error("invalid length (expected 6 or 8 bytes)")]
    Length,
    /// Invalid hex byte
    #[error("invalid hex byte (expected 0-9, a-f or A-F)")]
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

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("#") {
            let a;
            (a, s) = s.split_at(1);
            debug_assert_eq!(a, "#");
        }
        Rgba8Srgb::try_parse(s)
    }
}

impl std::str::FromStr for Rgba {
    type Err = ParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Rgba8Srgb::from_str(s).map(|c| c.into())
    }
}

impl std::str::FromStr for Rgb {
    type Err = ParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Rgba8Srgb::from_str(s).map(|c| c.into())
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Rgba8Srgb {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.format_html())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Rgba8Srgb {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Rgba8Srgb;

            fn expecting(&self, fmtr: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(
                    fmtr,
                    "an HTML color code with optional '#' prefix then 6 or 8 hex digits"
                )
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.parse().map_err(E::custom)
            }
        }

        deserializer.deserialize_str(Visitor)
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

impl From<Rgba8Srgb> for Rgb {
    fn from(c: Rgba8Srgb) -> Self {
        Rgb {
            r: into_linear(c.0[0]),
            g: into_linear(c.0[1]),
            b: into_linear(c.0[2]),
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

impl From<Rgb> for Rgba8Srgb {
    fn from(c: Rgb) -> Self {
        Rgba8Srgb([from_linear(c.r), from_linear(c.g), from_linear(c.b), 255])
    }
}
