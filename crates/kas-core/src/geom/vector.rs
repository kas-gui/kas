// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Vector types
//!
//! For drawing operations, all dimensions use the `f32` type.

use crate::cast::{CastFloat, Conv};
use crate::geom::{Coord, Offset, Rect, Size};
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub};

/// Axis-aligned 2D cuboid, specified via two corners `a` and `b`
///
/// Typically it is expected that `a.le(b)`, although this is not required.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Quad {
    pub a: Vec2,
    pub b: Vec2,
}

impl Quad {
    /// Construct with two coords
    #[inline]
    pub fn from_coords(a: Vec2, b: Vec2) -> Self {
        Quad { a, b }
    }

    /// Construct with position and size
    #[inline]
    pub fn from_pos_and_size(pos: Vec2, size: Vec2) -> Self {
        Quad {
            a: pos,
            b: pos + size,
        }
    }

    /// Get the size
    #[inline]
    pub fn size(&self) -> Vec2 {
        self.b - self.a
    }

    /// Swizzle coordinates: x from first, y from second point
    #[inline]
    pub fn ab(&self) -> Vec2 {
        Vec2(self.a.0, self.b.1)
    }

    /// Swizzle coordinates: x from second, y from first point
    #[inline]
    pub fn ba(&self) -> Vec2 {
        Vec2(self.b.0, self.a.1)
    }

    /// Shrink self in all directions by the given `value`
    ///
    /// In debug mode, this asserts `a.le(b)` after shrinking.
    #[inline]
    #[must_use = "method does not modify self but returns a new value"]
    pub fn shrink(&self, value: f32) -> Quad {
        let a = self.a + value;
        let b = self.b - value;
        debug_assert!(a.le(b));
        Quad { a, b }
    }

    /// Grow self in all directions by the given `value`
    ///
    /// In debug mode, this asserts `a.le(b)` after shrinking.
    #[inline]
    #[must_use = "method does not modify self but returns a new value"]
    pub fn grow(&self, value: f32) -> Quad {
        let a = self.a - value;
        let b = self.b + value;
        debug_assert!(a.le(b));
        Quad { a, b }
    }

    /// Shrink self in all directions by the given `value`
    ///
    /// In debug mode, this asserts `a.le(b)` after shrinking.
    #[inline]
    #[must_use = "method does not modify self but returns a new value"]
    pub fn shrink_vec(&self, value: Vec2) -> Quad {
        let a = self.a + value;
        let b = self.b - value;
        debug_assert!(a.le(b));
        Quad { a, b }
    }

    /// Calculate the intersection of two quads
    #[inline]
    pub fn intersection(&self, rhs: &Quad) -> Option<Quad> {
        let a = Vec2(self.a.0.max(rhs.a.0), self.a.1.max(rhs.a.1));
        let x = (self.b.0.min(rhs.b.0) - a.0).max(0.0);
        let y = (self.b.1.min(rhs.b.1) - a.1).max(0.0);
        if x * y > 0.0 {
            Some(Quad::from_pos_and_size(a, Vec2(x, y)))
        } else {
            None
        }
    }
}

impl AddAssign<Vec2> for Quad {
    #[inline]
    fn add_assign(&mut self, rhs: Vec2) {
        self.a += rhs;
        self.b += rhs;
    }
}

impl From<Rect> for Quad {
    #[inline]
    fn from(rect: Rect) -> Quad {
        let a = Vec2::from(rect.pos);
        let b = a + Vec2::from(rect.size);
        Quad { a, b }
    }
}

/// 2D vector
///
/// Usually used as either a coordinate or a difference of coordinates, but
/// may have some other uses.
///
/// Vectors are partially ordered and support component-wise comparison via
/// methods like `lhs.lt(rhs)`. The `PartialOrd` trait is not implemented since
/// it implements `lhs ≤ rhs` as `lhs < rhs || lhs == rhs` which is wrong for
/// vectors (consider for `lhs = (0, 1), rhs = (1, 0)`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vec2(pub f32, pub f32);

/// 2D vector (double precision)
///
/// Usually used as either a coordinate or a difference of coordinates, but
/// may have some other uses.
///
/// Vectors are partially ordered and support component-wise comparison via
/// methods like `lhs.lt(rhs)`. The `PartialOrd` trait is not implemented since
/// it implements `lhs ≤ rhs` as `lhs < rhs || lhs == rhs` which is wrong for
/// vectors (consider for `lhs = (0, 1), rhs = (1, 0)`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DVec2(pub f64, pub f64);

macro_rules! impl_vec2 {
    ($T:ident, $f:ty) => {
        impl $T {
            /// Zero
            pub const ZERO: $T = $T::splat(0.0);

            /// One
            pub const ONE: $T = $T::splat(1.0);

            /// Positive infinity
            pub const INFINITY: $T = $T::splat(<$f>::INFINITY);

            /// Constructs a new instance with each element initialized to `value`.
            #[inline]
            pub const fn splat(value: $f) -> Self {
                $T(value, value)
            }

            /// Take the minimum component
            #[inline]
            pub fn min_comp(self) -> $f {
                self.0.min(self.1)
            }

            /// Return the minimum, componentwise
            #[inline]
            #[must_use = "method does not modify self but returns a new value"]
            pub fn min(self, other: Self) -> Self {
                $T(self.0.min(other.0), self.1.min(other.1))
            }

            /// Return the maximum, componentwise
            #[inline]
            #[must_use = "method does not modify self but returns a new value"]
            pub fn max(self, other: Self) -> Self {
                $T(self.0.max(other.0), self.1.max(other.1))
            }

            /// Take the absolute value of each component
            #[inline]
            #[must_use = "method does not modify self but returns a new value"]
            pub fn abs(self) -> Self {
                $T(self.0.abs(), self.1.abs())
            }

            /// Take the floor of each component
            #[inline]
            #[must_use = "method does not modify self but returns a new value"]
            pub fn floor(self) -> Self {
                $T(self.0.floor(), self.1.floor())
            }

            /// Take the ceiling of each component
            #[inline]
            #[must_use = "method does not modify self but returns a new value"]
            pub fn ceil(self) -> Self {
                $T(self.0.ceil(), self.1.ceil())
            }

            /// Round each component to the nearest integer
            #[inline]
            #[must_use = "method does not modify self but returns a new value"]
            pub fn round(self) -> Self {
                $T(self.0.round(), self.1.round())
            }

            /// For each component, return `±1` with the same sign as `self`.
            #[inline]
            #[must_use = "method does not modify self but returns a new value"]
            pub fn sign(self) -> Self {
                let one: $f = 1.0;
                $T(one.copysign(self.0), one.copysign(self.1))
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

            /// Multiply two vectors as if they are complex numbers
            #[inline]
            #[must_use = "method does not modify self but returns a new value"]
            pub fn complex_mul(self, rhs: Self) -> Self {
                $T(
                    self.0 * rhs.0 - self.1 * rhs.1,
                    self.0 * rhs.1 + self.1 * rhs.0,
                )
            }

            /// Divide by a second vector as if they are complex numbers
            #[inline]
            #[must_use = "method does not modify self but returns a new value"]
            pub fn complex_div(self, rhs: Self) -> Self {
                self.complex_mul(rhs.complex_inv())
            }

            /// Take the complex reciprocal
            #[inline]
            #[must_use = "method does not modify self but returns a new value"]
            pub fn complex_inv(self) -> Self {
                let ssi = 1.0 / self.sum_square();
                $T(self.0 * ssi, -self.1 * ssi)
            }

            /// Return the sum of the terms
            #[inline]
            pub fn sum(self) -> $f {
                self.0 + self.1
            }

            /// Return the sum of the square of the terms
            #[inline]
            pub fn sum_square(self) -> $f {
                self.0 * self.0 + self.1 * self.1
            }
        }

        impl Neg for $T {
            type Output = $T;
            #[inline]
            fn neg(self) -> Self::Output {
                $T(-self.0, -self.1)
            }
        }

        impl Add<$T> for $T {
            type Output = $T;
            #[inline]
            fn add(self, rhs: $T) -> Self::Output {
                $T(self.0 + rhs.0, self.1 + rhs.1)
            }
        }

        impl Add<$f> for $T {
            type Output = $T;
            #[inline]
            fn add(self, rhs: $f) -> Self::Output {
                $T(self.0 + rhs, self.1 + rhs)
            }
        }

        impl AddAssign<$T> for $T {
            #[inline]
            fn add_assign(&mut self, rhs: $T) {
                self.0 += rhs.0;
                self.1 += rhs.1;
            }
        }

        impl AddAssign<$f> for $T {
            #[inline]
            fn add_assign(&mut self, rhs: $f) {
                self.0 += rhs;
                self.1 += rhs;
            }
        }

        impl Sub<$T> for $T {
            type Output = $T;
            #[inline]
            fn sub(self, rhs: $T) -> Self::Output {
                $T(self.0 - rhs.0, self.1 - rhs.1)
            }
        }

        impl Sub<$f> for $T {
            type Output = $T;
            #[inline]
            fn sub(self, rhs: $f) -> Self::Output {
                $T(self.0 - rhs, self.1 - rhs)
            }
        }

        impl Mul<$T> for $T {
            type Output = $T;
            #[inline]
            fn mul(self, rhs: $T) -> Self::Output {
                $T(self.0 * rhs.0, self.1 * rhs.1)
            }
        }

        impl Mul<$f> for $T {
            type Output = $T;
            #[inline]
            fn mul(self, rhs: $f) -> Self::Output {
                $T(self.0 * rhs, self.1 * rhs)
            }
        }

        impl Div<$T> for $T {
            type Output = $T;
            #[inline]
            fn div(self, rhs: $T) -> Self::Output {
                $T(self.0 / rhs.0, self.1 / rhs.1)
            }
        }

        impl Div<$f> for $T {
            type Output = $T;
            #[inline]
            fn div(self, rhs: $f) -> Self::Output {
                $T(self.0 / rhs, self.1 / rhs)
            }
        }

        impl Div<$T> for $f {
            type Output = $T;
            #[inline]
            fn div(self, rhs: $T) -> Self::Output {
                $T(self / rhs.0, self / rhs.1)
            }
        }

        impl From<($f, $f)> for $T {
            #[inline]
            fn from(arg: ($f, $f)) -> Self {
                $T(arg.0, arg.1)
            }
        }

        impl From<$T> for ($f, $f) {
            #[inline]
            fn from(v: $T) -> Self {
                (v.0, v.1)
            }
        }

        impl From<Coord> for $T {
            #[inline]
            fn from(arg: Coord) -> Self {
                $T(<$f>::conv(arg.0), <$f>::conv(arg.1))
            }
        }

        impl From<Size> for $T {
            #[inline]
            fn from(arg: Size) -> Self {
                $T(<$f>::conv(arg.0), <$f>::conv(arg.1))
            }
        }

        impl From<Offset> for $T {
            #[inline]
            fn from(arg: Offset) -> Self {
                $T(<$f>::conv(arg.0), <$f>::conv(arg.1))
            }
        }

        impl From<$T> for Coord {
            #[inline]
            fn from(arg: $T) -> Self {
                Coord(arg.0.cast_nearest(), arg.1.cast_nearest())
            }
        }

        impl From<$T> for Size {
            #[inline]
            fn from(arg: $T) -> Self {
                Offset::from(arg).into()
            }
        }

        impl From<$T> for Offset {
            #[inline]
            fn from(arg: $T) -> Self {
                Offset(arg.0.cast_nearest(), arg.1.cast_nearest())
            }
        }

        impl From<kas_text::Vec2> for $T {
            #[inline]
            fn from(size: kas_text::Vec2) -> Self {
                $T(size.0.into(), size.1.into())
            }
        }
    };
}

impl From<Vec2> for kas_text::Vec2 {
    fn from(size: Vec2) -> kas_text::Vec2 {
        kas_text::Vec2(size.0, size.1)
    }
}

impl From<DVec2> for Vec2 {
    fn from(size: DVec2) -> Vec2 {
        Vec2(size.0 as f32, size.1 as f32)
    }
}

impl_vec2!(Vec2, f32);
impl_vec2!(DVec2, f64);

/// 3D vector
///
/// Usually used for a 2D coordinate with a depth value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vec3(pub f32, pub f32, pub f32);

impl Vec3 {
    /// Construct from a [`Vec2`] and third value
    #[inline]
    pub fn from2(v: Vec2, z: f32) -> Self {
        Vec3(v.0, v.1, z)
    }
}
