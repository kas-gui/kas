// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Vector types
//!
//! For drawing operations, all dimensions use the `f32` type.

use kas::geom::{Coord, Rect, Size};
use std::ops::{Add, Div, Mul, Neg, Sub};

/// Axis-aligned 2D cuboid, specified via two corners `a` and `b`
///
/// Typically it is expected that `a.le(b)`, although this is not required.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub a: Vec2,
    pub b: Vec2,
}

impl Quad {
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
    pub fn shrink(&self, value: f32) -> Quad {
        let a = self.a + value;
        let b = self.b - value;
        debug_assert!(a.le(b));
        Quad { a, b }
    }

    /// Shrink self in all directions by the given `value`
    ///
    /// In debug mode, this asserts `a.le(b)` after shrinking.
    #[inline]
    pub fn shrink_vec(&self, value: Vec2) -> Quad {
        let a = self.a + value;
        let b = self.b - value;
        debug_assert!(a.le(b));
        Quad { a, b }
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
/// vectors (consider for `lhs = (0, 1), rhs = (1, 1)`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2(pub f32, pub f32);

impl Vec2 {
    /// Zero
    pub const ZERO: Vec2 = Vec2(0.0, 0.0);

    /// Constructs a new instance with each element initialized to `value`.
    #[inline]
    pub const fn splat(value: f32) -> Self {
        Vec2(value, value)
    }

    /// Take the minimum component
    #[inline]
    pub fn min_comp(self) -> f32 {
        self.0.min(self.1)
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

    /// Multiply two vectors as if they are complex numbers
    #[inline]
    pub fn complex_mul(self, rhs: Self) -> Self {
        Vec2(
            self.0 * rhs.0 - self.1 * rhs.1,
            self.0 * rhs.1 + self.1 * rhs.0,
        )
    }

    /// Divide by a second vector as if they are complex numbers
    #[inline]
    pub fn complex_div(self, rhs: Self) -> Self {
        self.complex_mul(rhs.complex_inv())
    }

    /// Take the complex reciprocal
    #[inline]
    pub fn complex_inv(self) -> Self {
        let ssi = 1.0 / self.sum_square();
        Vec2(self.0 * ssi, -self.1 * ssi)
    }

    /// Return the sum of the terms
    #[inline]
    pub fn sum(self) -> f32 {
        self.0 + self.1
    }

    /// Return the sum of the square of the terms
    #[inline]
    pub fn sum_square(self) -> f32 {
        self.0 * self.0 + self.1 * self.1
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

impl Mul<Vec2> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn mul(self, rhs: Vec2) -> Self::Output {
        Vec2(self.0 * rhs.0, self.1 * rhs.1)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn mul(self, rhs: f32) -> Self::Output {
        Vec2(self.0 * rhs, self.1 * rhs)
    }
}

impl Div<Vec2> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn div(self, rhs: Vec2) -> Self::Output {
        Vec2(self.0 / rhs.0, self.1 / rhs.1)
    }
}

impl Div<f32> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn div(self, rhs: f32) -> Self::Output {
        Vec2(self.0 / rhs, self.1 / rhs)
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

impl From<Coord> for Vec2 {
    #[inline]
    fn from(arg: Coord) -> Self {
        Vec2(arg.0 as f32, arg.1 as f32)
    }
}

impl From<Size> for Vec2 {
    #[inline]
    fn from(arg: Size) -> Self {
        Vec2(arg.0 as f32, arg.1 as f32)
    }
}

impl From<Vec2> for Coord {
    #[inline]
    fn from(arg: Vec2) -> Self {
        Coord(arg.0.round() as i32, arg.1.round() as i32)
    }
}

impl From<Vec2> for Size {
    #[inline]
    fn from(arg: Vec2) -> Self {
        Size(arg.0.round() as u32, arg.1.round() as u32)
    }
}

/// 2D vector (double precision)
///
/// Usually used as either a coordinate or a difference of coordinates, but
/// may have some other uses.
///
/// Vectors are partially ordered and support component-wise comparison via
/// methods like `lhs.lt(rhs)`. The `PartialOrd` trait is not implemented since
/// it implements `lhs ≤ rhs` as `lhs < rhs || lhs == rhs` which is wrong for
/// vectors (consider for `lhs = (0, 1), rhs = (1, 1)`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DVec2(pub f64, pub f64);

impl DVec2 {
    /// Zero
    pub const ZERO: DVec2 = DVec2(0.0, 0.0);

    /// Constructs a new instance with each element initialized to `value`.
    #[inline]
    pub const fn splat(value: f64) -> Self {
        DVec2(value, value)
    }

    /// For each component, return `±1` with the same sign as `self`.
    #[inline]
    pub fn sign(self) -> Self {
        let one = 1f64;
        DVec2(one.copysign(self.0), one.copysign(self.1))
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
    pub fn complex_mul(self, rhs: Self) -> Self {
        DVec2(
            self.0 * rhs.0 - self.1 * rhs.1,
            self.0 * rhs.1 + self.1 * rhs.0,
        )
    }

    /// Divide by a second vector as if they are complex numbers
    #[inline]
    pub fn complex_div(self, rhs: Self) -> Self {
        self.complex_mul(rhs.complex_inv())
    }

    /// Take the complex reciprocal
    #[inline]
    pub fn complex_inv(self) -> Self {
        let ssi = 1.0 / self.sum_square();
        DVec2(self.0 * ssi, -self.1 * ssi)
    }

    /// Return the sum of the terms
    #[inline]
    pub fn sum(self) -> f64 {
        self.0 + self.1
    }

    /// Return the sum of the square of the terms
    #[inline]
    pub fn sum_square(self) -> f64 {
        self.0 * self.0 + self.1 * self.1
    }
}

impl Neg for DVec2 {
    type Output = DVec2;
    #[inline]
    fn neg(self) -> Self::Output {
        DVec2(-self.0, -self.1)
    }
}

impl Add<DVec2> for DVec2 {
    type Output = DVec2;
    #[inline]
    fn add(self, rhs: DVec2) -> Self::Output {
        DVec2(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl Add<f64> for DVec2 {
    type Output = DVec2;
    #[inline]
    fn add(self, rhs: f64) -> Self::Output {
        DVec2(self.0 + rhs, self.1 + rhs)
    }
}

impl Sub<DVec2> for DVec2 {
    type Output = DVec2;
    #[inline]
    fn sub(self, rhs: DVec2) -> Self::Output {
        DVec2(self.0 - rhs.0, self.1 - rhs.1)
    }
}

impl Sub<f64> for DVec2 {
    type Output = DVec2;
    #[inline]
    fn sub(self, rhs: f64) -> Self::Output {
        DVec2(self.0 - rhs, self.1 - rhs)
    }
}

impl Mul<DVec2> for DVec2 {
    type Output = DVec2;
    #[inline]
    fn mul(self, rhs: DVec2) -> Self::Output {
        DVec2(self.0 * rhs.0, self.1 * rhs.1)
    }
}

impl Mul<f64> for DVec2 {
    type Output = DVec2;
    #[inline]
    fn mul(self, rhs: f64) -> Self::Output {
        DVec2(self.0 * rhs, self.1 * rhs)
    }
}

impl Div<DVec2> for DVec2 {
    type Output = DVec2;
    #[inline]
    fn div(self, rhs: DVec2) -> Self::Output {
        DVec2(self.0 / rhs.0, self.1 / rhs.1)
    }
}

impl Div<f64> for DVec2 {
    type Output = DVec2;
    #[inline]
    fn div(self, rhs: f64) -> Self::Output {
        DVec2(self.0 / rhs, self.1 / rhs)
    }
}

impl From<(f64, f64)> for DVec2 {
    #[inline]
    fn from(arg: (f64, f64)) -> Self {
        DVec2(arg.0, arg.1)
    }
}

impl From<DVec2> for (f64, f64) {
    #[inline]
    fn from(v: DVec2) -> Self {
        (v.0, v.1)
    }
}

impl From<Coord> for DVec2 {
    #[inline]
    fn from(arg: Coord) -> Self {
        DVec2(arg.0 as f64, arg.1 as f64)
    }
}

impl From<Size> for DVec2 {
    #[inline]
    fn from(arg: Size) -> Self {
        DVec2(arg.0 as f64, arg.1 as f64)
    }
}

impl From<Vec2> for DVec2 {
    #[inline]
    fn from(arg: Vec2) -> Self {
        DVec2(arg.0 as f64, arg.1 as f64)
    }
}

impl From<DVec2> for Coord {
    #[inline]
    fn from(arg: DVec2) -> Self {
        Coord(arg.0.round() as i32, arg.1.round() as i32)
    }
}

impl From<DVec2> for Size {
    #[inline]
    fn from(arg: DVec2) -> Self {
        Size(arg.0.round() as u32, arg.1.round() as u32)
    }
}
