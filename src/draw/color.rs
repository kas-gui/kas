// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Colour types

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
