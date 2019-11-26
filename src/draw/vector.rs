// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Vector types

use std::ops::{Add, Neg, Sub};

/// 2D vector
///
/// Usually used as either a coordinate or a difference of coordinates, but
/// may have some other uses.
///
/// Vectors are partially ordered and support component-wise comparison via
/// methods like `lhs.lt(rhs)`. The `PartialOrd` trait is not implemented since
/// it implements `lhs ≤ rhs` as `lhs < rhs || lhs == rhs` which is wrong for
/// vectors (consider for `lhs = (0, 1), rhs = (1, 1)`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec2(pub f32, pub f32);

impl Vec2 {
    /// Constructs a new instance with each element initialized to `value`.
    #[inline]
    pub const fn splat(value: f32) -> Self {
        Vec2(value, value)
    }

    /// For each component, return `±1` with the same sign as `self`.
    #[inline]
    pub fn sign(self) -> Self {
        let one = 1f32;
        Vec2(one.copysign(self.0), one.copysign(self.1))
    }

    /// True when for all components, `lhs < rhs`
    #[inline]
    pub fn lt(self, rhs: Self) -> bool {
        self.0 < rhs.0 && self.1 < rhs.1
    }

    /// True when for all components, `lhs ≤ rhs`
    #[inline]
    pub fn le(self, rhs: Self) -> bool {
        self.0 <= rhs.0 && self.1 <= rhs.1
    }

    /// True when for all components, `lhs ≥ rhs`
    #[inline]
    pub fn ge(self, rhs: Self) -> bool {
        self.0 >= rhs.0 && self.1 >= rhs.1
    }

    /// True when for all components, `lhs > rhs`
    #[inline]
    pub fn gt(self, rhs: Self) -> bool {
        self.0 > rhs.0 && self.1 > rhs.1
    }
}

impl Neg for Vec2 {
    type Output = Vec2;
    #[inline]
    fn neg(self) -> Self::Output {
        Vec2(-self.0, -self.1)
    }
}

impl Add<Vec2> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn add(self, rhs: Vec2) -> Self::Output {
        Vec2(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl Add<f32> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn add(self, rhs: f32) -> Self::Output {
        Vec2(self.0 + rhs, self.1 + rhs)
    }
}

impl Sub<Vec2> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn sub(self, rhs: Vec2) -> Self::Output {
        Vec2(self.0 - rhs.0, self.1 - rhs.1)
    }
}

impl Sub<f32> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn sub(self, rhs: f32) -> Self::Output {
        Vec2(self.0 - rhs, self.1 - rhs)
    }
}

impl From<(f32, f32)> for Vec2 {
    #[inline]
    fn from(arg: (f32, f32)) -> Self {
        Vec2(arg.0, arg.1)
    }
}

impl From<Vec2> for (f32, f32) {
    #[inline]
    fn from(v: Vec2) -> Self {
        (v.0, v.1)
    }
}
