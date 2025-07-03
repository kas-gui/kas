// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Affine transformation

use super::DVec2;
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// Linear transformation: scaling and rotation
///
/// This represents scaling and rotation transformations (i.e. the linear
/// mappings on [`DVec2`] in the mathematical sense, excluding skew).
///
/// A `Linear` transform `a` may be applied to a vector `v` via multiplication:
/// `a * v`. Multiple transforms can be combined: `a * (b * v) == (a * b) * v`.
///
/// `Linear` transforms are a [ring algebra](https://en.wikipedia.org/wiki/Ring_(mathematics))
/// with commutative operations. Both addition and multiplication operations are associative
/// and commutative, e.g. `(a * b) * c == a * (b * c)` and `a * b == b * c`. The operators are also
/// distributive: `a * (b + c) == a * b + a * c`.
/// (Subject to the limitations of floating-point numbers.)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Linear(DVec2);

impl Linear {
    /// The multiplicitive identity
    pub const IDENTITY: Linear = Linear(DVec2(1.0, 0.0));

    /// The additive identity
    pub const ZERO: Linear = Linear(DVec2::ZERO);

    /// Construct as a complex number
    ///
    /// The state is treated as a complex number of the form
    /// `u + v*i = a * e^{i*t}` where `a` is the scale component and `t` is the
    /// angle of rotation.
    #[inline]
    pub fn new(u: f64, v: f64) -> Self {
        Linear(DVec2(u, v))
    }

    /// Construct a scaling transform
    #[inline]
    pub fn scale(scale: f64) -> Self {
        Linear(DVec2(scale, 0.0))
    }

    /// Construct a rotating transform
    ///
    /// In case two vectors are available, it is preferable to use
    /// `Linear::from_vectors(a, b).normalize()`.
    ///
    /// To also scale, use `Linear::rotate(radians) * scale`.
    pub fn rotate(radians: f64) -> Self {
        let (s, c) = radians.sin_cos();
        Linear(DVec2(c, s))
    }

    /// Construct a scaling and rotation transform taking `a` to `b`
    ///
    /// This may be used to construct from two-finger touch motions. In this
    /// case, `a = old.finger1 - old.finger2` and
    /// `b = new.finger1 - new.finger2`.
    #[inline]
    pub fn from_vectors(a: DVec2, b: DVec2) -> Self {
        Linear(b.complex_div(a))
    }

    /// Construct from two vectors with optional scaling and rotation
    ///
    /// This is a multiplexer over [`Self::from_vectors`] and other methods,
    /// making scaling and rotation optional (though unless both are true,
    /// the transform won't map `a` to `b`).
    #[inline]
    pub fn pan(a: DVec2, b: DVec2, (scale, rotate): (bool, bool)) -> Self {
        match (scale, rotate) {
            (false, false) => Self::IDENTITY,
            (true, false) => Self::scale((b.sum_square() / a.sum_square()).sqrt()),
            (false, true) => Linear::from_vectors(a, b).normalize(),
            (true, true) => Linear::from_vectors(a, b),
        }
    }

    /// Get the internal representation
    ///
    /// The state is treated as a complex number of the form
    /// `u + v*i = a * e^{i*t}` where `a` is the scale component and `t` is the
    /// angle of rotation. These components can be calculated as follows:
    /// ```
    /// # let alpha = kas_core::geom::Linear::IDENTITY;
    /// let (u, v) = alpha.get_complex();
    /// let a = (u*u + v*v).sqrt();
    /// let t = v.atan2(a);
    /// ```
    ///
    /// The matrix form of this transform is:
    /// ```none
    ///     u  -v
    ///     v   u
    /// ```
    #[inline]
    pub fn get_complex(self) -> (f64, f64) {
        (self.0.0, self.0.1)
    }

    /// Get the internal representation as a [`DVec2`]
    #[inline]
    pub fn get_vec2(self) -> DVec2 {
        self.0
    }

    /// Calculate the change in scale (non-trivial)
    #[inline]
    pub fn get_scale(self) -> f64 {
        self.0.sum_square().sqrt()
    }

    /// Calculate the rotation angle (non-trivial)
    #[inline]
    pub fn get_angle(self) -> f64 {
        self.0.1.atan2(self.0.0)
    }

    /// True if the transform scales its input
    ///
    /// This is a non-trivial operation.
    #[inline]
    pub fn has_scale(self) -> bool {
        self.0.sum_square() != 1.0
    }

    /// True if the transform rotates its input
    ///
    /// This is a trivial operation.
    #[inline]
    pub fn has_rotation(self) -> bool {
        self.0.1 != 0.0
    }

    /// True if all components are finite
    #[inline]
    pub fn is_finite(self) -> bool {
        self.0.is_finite()
    }

    /// True if the transform has an inverse
    ///
    /// This test corresponds *approximately* but not exactly to `self.inverse().is_finite()`.
    /// Further, `self.is_bijective()` *approximately* implies `self.inverse().is_bijective()`.
    #[inline]
    pub fn is_bijective(self) -> bool {
        self.0.0.is_normal() && (self.0.1.is_normal() || self.0.1 == 0.0)
    }

    /// Remove the scaling component
    #[inline]
    pub fn normalize(self) -> Self {
        Linear(self.0 / self.0.sum_square().sqrt())
    }

    /// Calculate the inverse
    ///
    /// This is the reciprical: `Self::IDENTITY / self`.
    /// Due to the limitations of floating-point numbers, it is not guaranteed
    /// that `self * self.inverse() == Self::IDENTITY` in general.
    #[inline]
    pub fn inverse(self) -> Self {
        Linear(self.0.complex_inv())
    }
}

impl Neg for Linear {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Linear(-self.0)
    }
}

impl Mul<f64> for Linear {
    type Output = Linear;

    #[inline]
    fn mul(self, rhs: f64) -> Linear {
        Linear(self.0 * rhs)
    }
}

impl MulAssign<f64> for Linear {
    #[inline]
    fn mul_assign(&mut self, rhs: f64) {
        self.0 *= rhs;
    }
}

impl Div<f64> for Linear {
    type Output = Linear;

    #[inline]
    fn div(self, rhs: f64) -> Linear {
        Linear(self.0 / rhs)
    }
}

impl DivAssign<f64> for Linear {
    #[inline]
    fn div_assign(&mut self, rhs: f64) {
        self.0 /= rhs;
    }
}

impl Add<Linear> for Linear {
    type Output = Linear;

    #[inline]
    fn add(self, rhs: Linear) -> Linear {
        Linear(self.0 + rhs.0)
    }
}

impl AddAssign<Linear> for Linear {
    #[inline]
    fn add_assign(&mut self, rhs: Linear) {
        self.0 += rhs.0;
    }
}

impl Sub<Linear> for Linear {
    type Output = Linear;

    #[inline]
    fn sub(self, rhs: Linear) -> Linear {
        Linear(self.0 - rhs.0)
    }
}

impl SubAssign<Linear> for Linear {
    #[inline]
    fn sub_assign(&mut self, rhs: Linear) {
        self.0 -= rhs.0;
    }
}

impl Mul<Linear> for Linear {
    type Output = Linear;

    #[inline]
    fn mul(self, rhs: Linear) -> Linear {
        Linear(self.0.complex_mul(rhs.0))
    }
}

impl MulAssign<Linear> for Linear {
    #[inline]
    fn mul_assign(&mut self, rhs: Linear) {
        *self = *self * rhs;
    }
}

impl Div<Linear> for Linear {
    type Output = Linear;

    #[inline]
    fn div(self, rhs: Linear) -> Linear {
        Linear(self.0.complex_div(rhs.0))
    }
}

impl DivAssign<Linear> for Linear {
    #[inline]
    fn div_assign(&mut self, rhs: Linear) {
        self.0 = self.0.complex_div(rhs.0);
    }
}

impl Mul<DVec2> for Linear {
    type Output = DVec2;

    #[inline]
    fn mul(self, rhs: DVec2) -> DVec2 {
        self.0.complex_mul(rhs)
    }
}

/// Affine transformations: move/scale/rotate
///
/// Note that the current representation is limited to transformations which
/// preserve the angle: it cannot represent reflection or shear transformations.
///
/// An `Affine` transform `a` may be applied to a [`DVec2`] coordinate `v` via
/// multiplication: `a * v`. This is equivalent to `a.alpha() * v + a.delta()`.
/// Two transforms `a`, `b` may be combined via multiplication: `a * b`. Note
/// that this is associative but not commutative:
/// `b * (a * v) == (b * a) * v` but `a * b != b * a` in general.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    /// Rotation and scale component
    alpha: Linear,
    /// Translation component
    delta: DVec2,
}

impl Affine {
    /// The identity transform
    pub const IDENTITY: Affine = Affine {
        alpha: Linear::IDENTITY,
        delta: DVec2::ZERO,
    };

    /// Construct from components
    #[inline]
    pub fn new(alpha: Linear, delta: DVec2) -> Self {
        Affine { alpha, delta }
    }

    /// Construct a translation transform
    #[inline]
    pub fn translate(delta: DVec2) -> Self {
        Affine {
            alpha: Linear::IDENTITY,
            delta,
        }
    }

    /// Construct a scaling and rotation transform taking `x0` to `x1` and `y0` to `y1`
    ///
    /// This may be used to construct from two-finger touch motions. In this
    /// case, `x0 = old.finger1`, `x1 = new.finger1`, `y0 = old.finger2` and
    /// `y1 = new.finger2`.
    pub fn from_vectors(x0: DVec2, x1: DVec2, y0: DVec2, y1: DVec2) -> Self {
        let alpha = Linear::from_vectors(x1 - x0, y1 - y0);
        // Average delta from both movements:
        let delta = (x1 - alpha * x0 + y1 - alpha * y0) * 0.5;
        Affine { alpha, delta }
    }

    /// Construct from two vectors with optional scaling and rotation
    ///
    /// This is a multiplexer over [`Self::from_vectors`] and other methods,
    /// making scaling and rotation optional (though unless both are true,
    /// the transform won't map `x0` to `x1` and `y0` to `y1`).
    pub fn pan(x0: DVec2, x1: DVec2, y0: DVec2, y1: DVec2, (scale, rotate): (bool, bool)) -> Self {
        let alpha = Linear::pan(y0 - x0, y1 - x1, (scale, rotate));
        // Average delta from both movements:
        let delta = (x1 - alpha * x0 + y1 - alpha * y0) * 0.5;
        Affine { alpha, delta }
    }

    /// Get component `alpha`
    ///
    /// This represents scaling and rotation transformations.
    #[inline]
    pub fn alpha(self) -> Linear {
        self.alpha
    }

    /// Get component `delta`
    ///
    /// This is the translation, applied after rotation and scaling.
    #[inline]
    pub fn delta(self) -> DVec2 {
        self.delta
    }

    /// True if the transform translates its input
    #[inline]
    pub fn has_translation(self) -> bool {
        self.delta != DVec2::ZERO
    }

    /// Get as `delta` if `self` represents a pure translation
    #[inline]
    pub fn as_translation(self) -> Option<DVec2> {
        if self.alpha == Linear::IDENTITY {
            Some(self.delta)
        } else {
            None
        }
    }

    /// Get as `(scale, delta)` if `self` represents a scaling and translation
    #[inline]
    pub fn as_scale_and_translation(self) -> Option<(f64, DVec2)> {
        if self.alpha.0.1 == 0.0 {
            Some((self.alpha.0.0, self.delta))
        } else {
            None
        }
    }

    /// True if all components are finite
    #[inline]
    pub fn is_finite(self) -> bool {
        self.alpha.is_finite() && self.delta.is_finite()
    }

    /// Calculate the inverse transform
    ///
    /// If `self` has scale zero (i.e. maps everything to a point) then the
    /// result will not be finite.
    pub fn inverse(self) -> Self {
        let alpha = self.alpha.inverse();
        let delta = -alpha * self.delta;
        Affine { alpha, delta }
    }
}

impl From<Linear> for Affine {
    #[inline]
    fn from(alpha: Linear) -> Self {
        Affine {
            alpha,
            delta: DVec2::ZERO,
        }
    }
}

impl Mul<DVec2> for Affine {
    type Output = DVec2;

    #[inline]
    fn mul(self, rhs: DVec2) -> DVec2 {
        self.alpha * rhs + self.delta
    }
}

impl Mul<Linear> for Affine {
    type Output = Affine;

    #[inline]
    fn mul(mut self, rhs: Linear) -> Affine {
        self.alpha *= rhs;
        self
    }
}

impl MulAssign<Linear> for Affine {
    #[inline]
    fn mul_assign(&mut self, rhs: Linear) {
        self.alpha *= rhs;
    }
}

impl Mul<Affine> for Linear {
    type Output = Affine;

    fn mul(self, rhs: Affine) -> Affine {
        let alpha = self * rhs.alpha;
        let delta = self * rhs.delta;
        Affine { alpha, delta }
    }
}

impl Mul<Affine> for Affine {
    type Output = Affine;

    fn mul(self, rhs: Affine) -> Affine {
        let alpha = self.alpha * rhs.alpha;
        let delta = self.alpha * rhs.delta + self.delta;
        Affine { alpha, delta }
    }
}

impl Div<Linear> for Affine {
    type Output = Affine;

    #[inline]
    fn div(mut self, rhs: Linear) -> Affine {
        self.alpha /= rhs;
        self
    }
}

impl DivAssign<Linear> for Affine {
    #[inline]
    fn div_assign(&mut self, rhs: Linear) {
        self.alpha /= rhs;
    }
}

impl Div<Affine> for Linear {
    type Output = Affine;

    fn div(self, rhs: Affine) -> Affine {
        let alpha = self / rhs.alpha;
        let delta = -alpha * rhs.delta;
        Affine { alpha, delta }
    }
}

impl Div<Affine> for Affine {
    type Output = Affine;

    fn div(self, rhs: Affine) -> Affine {
        let alpha = self.alpha / rhs.alpha;
        let delta = self.delta - alpha * rhs.delta;
        Affine { alpha, delta }
    }
}

impl Add<DVec2> for Affine {
    type Output = Affine;

    #[inline]
    fn add(mut self, rhs: DVec2) -> Affine {
        self.delta += rhs;
        self
    }
}

impl AddAssign<DVec2> for Affine {
    #[inline]
    fn add_assign(&mut self, rhs: DVec2) {
        self.delta += rhs;
    }
}

impl Sub<DVec2> for Affine {
    type Output = Affine;

    #[inline]
    fn sub(mut self, rhs: DVec2) -> Affine {
        self.delta -= rhs;
        self
    }
}

impl SubAssign<DVec2> for Affine {
    #[inline]
    fn sub_assign(&mut self, rhs: DVec2) {
        self.delta -= rhs;
    }
}
