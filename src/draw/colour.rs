// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Colour type and theming

/// Standard colour description
///
/// NOTE: spelling standardisation is omitted for this type on the basis that
/// is expected to be replaced in the near future.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Colour {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Colour {
    /// Transparent black
    pub const TRANSPARENT: Colour = Colour {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    /// Opaque white
    pub const WHITE: Colour = Colour::grey(1.0);
    /// Opaque black
    pub const BLACK: Colour = Colour::grey(0.0);

    /// Constructor
    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Colour { r, g, b, a: 1.0 }
    }

    /// Construct from grey-scale
    pub const fn grey(s: f32) -> Self {
        Colour::new(s, s, s)
    }
}

impl From<Colour> for [f32; 4] {
    fn from(c: Colour) -> Self {
        [c.r, c.g, c.b, c.a]
    }
}

impl From<[f32; 4]> for Colour {
    fn from(c: [f32; 4]) -> Self {
        Colour {
            r: c[0],
            g: c[1],
            b: c[2],
            a: c[3],
        }
    }
}
