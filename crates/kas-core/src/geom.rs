// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Geometry data types
//!
//! [`Coord`], [`Size`] and [`Offset`] are all 2D integer (`i32`) types,
//! representing positions, sizes and scroll deltas respectively.
//!
//! [`Vec2`] is a 2D floating-point (`f32`) type used mainly for screen-space
//! position during rendering.
//!
//! Conversions types mostly use [`Cast`] and [`Conv`]. [`From`] may be used to
//! simply pack/unpack components. To convert from floating-point types to
//! integer types, use [`CastApprox`] or [`CastFloat`] to specify the rounding
//! mode.

use crate::cast::*;
use crate::dir::Directional;
use std::cmp::{Ordering, PartialOrd};

mod transform;
mod vector;
pub use transform::{Affine, Linear};
pub use vector::{DVec2, Quad, Vec2, Vec3};

macro_rules! impl_common {
    ($T:ty) => {
        impl $T {
            /// The constant `(0, 0)`
            pub const ZERO: Self = Self(0, 0);

            /// The minimum value
            pub const MIN: Self = Self(i32::MIN, i32::MIN);

            /// The maximum value
            pub const MAX: Self = Self(i32::MAX, i32::MAX);

            /// Return the minimum, componentwise
            #[inline]
            #[must_use = "method does not modify self but returns a new value"]
            pub fn min(self, other: Self) -> Self {
                Self(self.0.min(other.0), self.1.min(other.1))
            }

            /// Return the maximum, componentwise
            #[inline]
            #[must_use = "method does not modify self but returns a new value"]
            pub fn max(self, other: Self) -> Self {
                Self(self.0.max(other.0), self.1.max(other.1))
            }

            /// Restrict a value to the specified interval, componentwise
            #[inline]
            #[must_use = "method does not modify self but returns a new value"]
            pub fn clamp(self, min: Self, max: Self) -> Self {
                debug_assert!(min <= max);
                self.min(max).max(min)
            }

            /// Return the transpose (swap x and y values)
            #[inline]
            #[must_use = "method does not modify self but returns a new value"]
            pub fn transpose(self) -> Self {
                Self(self.1, self.0)
            }

            /// Return the result of component-wise multiplication
            #[inline]
            #[must_use = "method does not modify self but returns a new value"]
            pub fn cwise_mul(self, rhs: Self) -> Self {
                Self(self.0 * rhs.0, self.1 * rhs.1)
            }

            /// Return the result of component-wise division
            #[inline]
            #[must_use = "method does not modify self but returns a new value"]
            pub fn cwise_div(self, rhs: Self) -> Self {
                Self(self.0 / rhs.0, self.1 / rhs.1)
            }

            /// Return the L1 (rectilinear / taxicab) distance
            #[inline]
            pub fn distance_l1(self) -> i32 {
                self.0.abs() + self.1.abs()
            }

            /// Return the L-inf (max) distance
            #[inline]
            pub fn distance_l_inf(self) -> i32 {
                self.0.abs().max(self.1.abs())
            }

            /// Extract one component, based on a direction
            ///
            /// This merely extracts the horizontal or vertical component.
            /// It never negates it, even if the axis is reversed.
            #[inline]
            pub fn extract<D: Directional>(self, dir: D) -> i32 {
                match dir.is_vertical() {
                    false => self.0,
                    true => self.1,
                }
            }

            /// Set one component of self, based on a direction
            ///
            /// This does not negate components when the direction is reversed.
            #[inline]
            pub fn set_component<D: Directional>(&mut self, dir: D, value: i32) {
                match dir.is_vertical() {
                    false => self.0 = value,
                    true => self.1 = value,
                }
            }
        }

        impl PartialOrd for $T {
            fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
                if self == rhs {
                    Some(Ordering::Equal)
                } else if self.0 < rhs.0 && self.1 < rhs.1 {
                    Some(Ordering::Less)
                } else if self.0 > rhs.0 && self.1 > rhs.1 {
                    Some(Ordering::Greater)
                } else {
                    None
                }
            }

            #[inline]
            fn lt(&self, rhs: &Self) -> bool {
                self.0 < rhs.0 && self.1 < rhs.1
            }

            #[inline]
            fn le(&self, rhs: &Self) -> bool {
                self.0 <= rhs.0 && self.1 <= rhs.1
            }

            #[inline]
            fn ge(&self, rhs: &Self) -> bool {
                self.0 >= rhs.0 && self.1 >= rhs.1
            }

            #[inline]
            fn gt(&self, rhs: &Self) -> bool {
                self.0 > rhs.0 && self.1 > rhs.1
            }
        }

        impl From<(i32, i32)> for $T {
            #[inline]
            fn from(v: (i32, i32)) -> Self {
                Self(v.0, v.1)
            }
        }
        impl Conv<(i32, i32)> for $T {
            #[inline]
            fn conv(v: (i32, i32)) -> Self {
                Self(v.0, v.1)
            }
            #[inline]
            fn try_conv(v: (i32, i32)) -> Result<Self> {
                Ok(Self::conv(v))
            }
        }
    };
}

/// A 2D coordinate, also known as a point
///
/// A coordinate (or point) is an absolute position. One cannot add a point to
/// a point. The difference between two points is an [`Offset`].
///
/// `Coord` implements [`PartialOrd`] such that the comparison must be true of
/// all components: for example `a < b == a.0 < b.0 && a.1 < b.1`.
/// If `c == Coord(0, 1)` and `d == Coord(1, 0)` then
/// `c != d && !(c < d) && !(c > d)`. `Coord` does not implement [`Ord`].
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Coord(pub i32, pub i32);

impl_common!(Coord);

impl Coord {
    /// Construct
    #[inline]
    pub fn new(x: i32, y: i32) -> Self {
        Self(x, y)
    }

    /// Construct, using the same value on all axes
    #[inline]
    pub const fn splat(n: i32) -> Self {
        Self(n, n)
    }
}

impl std::ops::Sub for Coord {
    type Output = Offset;

    #[inline]
    fn sub(self, other: Self) -> Offset {
        Offset(self.0 - other.0, self.1 - other.1)
    }
}

impl std::ops::Add<Offset> for Coord {
    type Output = Self;

    #[inline]
    fn add(self, other: Offset) -> Self {
        Coord(self.0 + other.0, self.1 + other.1)
    }
}
impl std::ops::AddAssign<Offset> for Coord {
    #[inline]
    fn add_assign(&mut self, rhs: Offset) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}
impl std::ops::Sub<Offset> for Coord {
    type Output = Self;

    #[inline]
    fn sub(self, other: Offset) -> Self {
        Coord(self.0 - other.0, self.1 - other.1)
    }
}
impl std::ops::SubAssign<Offset> for Coord {
    #[inline]
    fn sub_assign(&mut self, rhs: Offset) {
        self.0 -= rhs.0;
        self.1 -= rhs.1;
    }
}

impl std::ops::Add<Size> for Coord {
    type Output = Self;

    #[inline]
    fn add(self, other: Size) -> Self {
        Coord(self.0 + other.0, self.1 + other.1)
    }
}
impl std::ops::AddAssign<Size> for Coord {
    #[inline]
    fn add_assign(&mut self, rhs: Size) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}
impl std::ops::Sub<Size> for Coord {
    type Output = Self;

    #[inline]
    fn sub(self, other: Size) -> Self {
        Coord(self.0 - other.0, self.1 - other.1)
    }
}
impl std::ops::SubAssign<Size> for Coord {
    #[inline]
    fn sub_assign(&mut self, rhs: Size) {
        self.0 -= rhs.0;
        self.1 -= rhs.1;
    }
}

impl Conv<Coord> for kas_text::Vec2 {
    #[inline]
    fn try_conv(pos: Coord) -> Result<Self> {
        Ok(Vec2::try_conv(pos)?.into())
    }
}

/// A 2D size, also known as an extent
///
/// This is both a size and a relative position. One can add or subtract a size
/// from a [`Coord`]. One can multiply a size by a scalar.
///
/// A `Size` is expected to be non-negative; some methods such as [`Size::new`]
/// and implementations of subtraction will check this, but only in debug mode
/// (similar to overflow checks on integers).
///
/// Subtraction is defined to be saturating subtraction.
///
/// `Size` implements [`PartialOrd`] such that the comparison must be true of
/// all components: for example `a < b == a.0 < b.0 && a.1 < b.1`.
/// If `c == Size(0, 1)` and `d == Size(1, 0)` then
/// `c != d && !(c < d) && !(c > d)`. `Size` does not implement [`Ord`].
///
/// This may be converted to [`Offset`] with `from` / `into`.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Size(pub i32, pub i32);

impl_common!(Size);

impl Size {
    /// Construct
    ///
    /// In debug mode, this asserts that components are non-negative.
    #[inline]
    pub fn new(w: i32, h: i32) -> Self {
        debug_assert!(w >= 0 && h >= 0, "Size::new({w}, {h}): negative value");
        Self(w, h)
    }

    /// Construct, using the same value on all axes
    ///
    /// In debug mode, this asserts that components are non-negative.
    #[inline]
    pub fn splat(n: i32) -> Self {
        debug_assert!(n >= 0, "Size::splat({n}): negative value");
        Self(n, n)
    }

    /// Scale to fit within the target size, keeping aspect ratio
    ///
    /// If either dimension of self is 0, this returns None.
    pub fn aspect_scale_to(self, target: Size) -> Option<Size> {
        if self.0 == 0 || self.1 == 0 {
            return None;
        }

        let h = i32::conv((i64::conv(self.1) * i64::conv(target.0)) / i64::conv(self.0));
        if h <= target.1 {
            Some(Size(target.0, h))
        } else {
            let w = i32::conv((i64::conv(self.0) * i64::conv(target.1)) / i64::conv(self.1));
            Some(Size(w, target.1))
        }
    }
}

impl std::ops::Add for Size {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Size(self.0 + other.0, self.1 + other.1)
    }
}
impl std::ops::AddAssign for Size {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

/// Subtract a `Size` from a `Size`
///
/// This is saturating subtraction: `Size::ZERO - Size::splat(6) == Size::ZERO`.
impl std::ops::Sub for Size {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        // This impl should aid vectorisation.
        Size(self.0 - rhs.0, self.1 - rhs.1).max(Size::ZERO)
    }
}
/// Subtract a `Size` from a `Size`
///
/// This is saturating subtraction.
impl std::ops::SubAssign for Size {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

/// Multiply a `Size` by an integer
///
/// In debug mode this asserts that the result is non-negative.
impl std::ops::Mul<i32> for Size {
    type Output = Self;

    #[inline]
    fn mul(self, x: i32) -> Self {
        debug_assert!(x >= 0);
        Size(self.0 * x, self.1 * x)
    }
}
/// Divide a `Size` by an integer
///
/// In debug mode this asserts that the result is non-negative.
impl std::ops::Div<i32> for Size {
    type Output = Self;

    #[inline]
    fn div(self, x: i32) -> Self {
        debug_assert!(x >= 0);
        Size(self.0 / x, self.1 / x)
    }
}

/// Convert an [`Offset`] into a [`Coord`]
///
/// In debug mode this asserts that the result is non-negative.
impl Conv<Offset> for Coord {
    #[inline]
    fn try_conv(v: Offset) -> Result<Self> {
        debug_assert!(v.0 >= 0 && v.1 >= 0, "Coord::conv({v:?}): negative value");
        Ok(Self(v.0, v.1))
    }
}

/// Convert an [`Offset`] into a [`Size`]
///
/// In debug mode this asserts that the result is non-negative.
impl Conv<Offset> for Size {
    #[inline]
    fn try_conv(v: Offset) -> Result<Self> {
        debug_assert!(v.0 >= 0 && v.1 >= 0, "Size::conv({v:?}): negative value");
        Ok(Self(v.0, v.1))
    }
}

// used for marigns
impl Conv<Size> for (u16, u16) {
    #[inline]
    fn try_conv(size: Size) -> Result<Self> {
        Ok((size.0.try_cast()?, size.1.try_cast()?))
    }
}
impl Conv<(u16, u16)> for Size {
    #[inline]
    fn try_conv(v: (u16, u16)) -> Result<Self> {
        Ok(Self(i32::try_conv(v.0)?, i32::try_conv(v.1)?))
    }
}

impl Conv<(u32, u32)> for Size {
    #[inline]
    fn try_conv(v: (u32, u32)) -> Result<Self> {
        Ok(Self(i32::try_conv(v.0)?, i32::try_conv(v.1)?))
    }
}

impl Conv<Size> for (u32, u32) {
    #[inline]
    fn try_conv(size: Size) -> Result<Self> {
        Ok((u32::try_conv(size.0)?, u32::try_conv(size.1)?))
    }
}

impl Conv<Size> for kas_text::Vec2 {
    #[inline]
    fn try_conv(size: Size) -> Result<Self> {
        Ok(Vec2::try_conv(size)?.into())
    }
}

/// A `(x, y)` offset, also known as a **vector**
///
/// This is a relative position. It can be added to or subtracted from a
/// [`Coord`], and it can be added to or subtracted from itself. It can be
/// negative. It can be multiplied by a scalar.
///
/// `Offset` implements [`PartialOrd`] such that the comparison must be true of
/// all components: for example `a < b == a.0 < b.0 && a.1 < b.1`.
/// If `c == Offset(0, 1)` and `d == Offset(1, 0)` then
/// `c != d && !(c < d) && !(c > d)`. `Offset` does not implement [`Ord`].
///
/// This may be converted to [`Size`] with `from` / `into`.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Offset(pub i32, pub i32);

impl_common!(Offset);

impl Offset {
    /// Construct
    #[inline]
    pub fn new(x: i32, y: i32) -> Self {
        Self(x, y)
    }

    /// Construct, using the same value on all axes
    #[inline]
    pub const fn splat(n: i32) -> Self {
        Self(n, n)
    }
}

impl std::ops::Add for Offset {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Offset(self.0 + other.0, self.1 + other.1)
    }
}
impl std::ops::AddAssign for Offset {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

impl std::ops::Sub for Offset {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        Offset(self.0 - other.0, self.1 - other.1)
    }
}
impl std::ops::SubAssign for Offset {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
        self.1 -= rhs.1;
    }
}

impl std::ops::Mul<i32> for Offset {
    type Output = Self;

    #[inline]
    fn mul(self, x: i32) -> Self {
        Offset(self.0 * x, self.1 * x)
    }
}
impl std::ops::Div<i32> for Offset {
    type Output = Self;

    #[inline]
    fn div(self, x: i32) -> Self {
        Offset(self.0 / x, self.1 / x)
    }
}

impl Conv<Coord> for Offset {
    #[inline]
    fn try_conv(v: Coord) -> Result<Self> {
        Ok(Self(v.0, v.1))
    }
}

impl Conv<Size> for Offset {
    #[inline]
    fn try_conv(v: Size) -> Result<Self> {
        Ok(Self(v.0, v.1))
    }
}

impl Conv<Offset> for kas_text::Vec2 {
    #[inline]
    fn try_conv(v: Offset) -> Result<Self> {
        Ok(Vec2::try_conv(v)?.into())
    }
}

/// An axis-aligned rectangular region
///
/// The region is defined by a point `pos` and an extent `size`, allowing easy
/// translations. It is empty unless `size` is positive on both axes.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rect {
    pub pos: Coord,
    pub size: Size,
}

impl Rect {
    /// The empty rect (all fields zero)
    pub const ZERO: Self = Self::new(Coord::ZERO, Size::ZERO);

    /// Construct from a [`Coord`] and [`Size`]
    #[inline]
    pub const fn new(pos: Coord, size: Size) -> Self {
        Rect { pos, size }
    }

    /// Construct from two coords
    ///
    /// It is expected that `pos <= pos2`.
    #[inline]
    pub fn from_coords(pos: Coord, pos2: Coord) -> Self {
        let size = (pos2 - pos).cast();
        Rect { pos, size }
    }

    /// Get the second point (pos + size)
    #[inline]
    pub fn pos2(&self) -> Coord {
        self.pos + self.size
    }

    /// Check whether the given coordinate is contained within this rect
    #[inline]
    pub fn contains(&self, c: Coord) -> bool {
        c.0 >= self.pos.0
            && c.0 < self.pos.0 + (self.size.0)
            && c.1 >= self.pos.1
            && c.1 < self.pos.1 + (self.size.1)
    }

    /// Calculate the intersection of two rects
    #[inline]
    pub fn intersection(&self, rhs: &Rect) -> Option<Rect> {
        let (l1, l2) = (self.pos, self.pos2());
        let (r1, r2) = (rhs.pos, rhs.pos2());
        let pos = l1.max(r1);
        let pos2 = l2.min(r2);
        if pos <= pos2 {
            Some(Rect::new(pos, (pos2 - pos).cast()))
        } else {
            None
        }
    }

    /// Shrink self in all directions by the given `n`
    #[inline]
    #[must_use = "method does not modify self but returns a new value"]
    pub fn shrink(&self, n: i32) -> Rect {
        let pos = self.pos + Offset::splat(n);
        let size = self.size - Size::splat(n + n);
        Rect { pos, size }
    }

    /// Expand self in all directions by the given `n`
    ///
    /// In debug mode this asserts that `n` is non-negative.
    #[inline]
    #[must_use = "method does not modify self but returns a new value"]
    pub fn expand(&self, n: i32) -> Rect {
        debug_assert!(n >= 0);
        let pos = self.pos - Offset::splat(n);
        let size = self.size + Size::splat(n + n);
        Rect { pos, size }
    }
}

impl std::ops::Add<Offset> for Rect {
    type Output = Self;

    #[inline]
    fn add(self, offset: Offset) -> Self {
        Rect::new(self.pos + offset, self.size)
    }
}
impl std::ops::AddAssign<Offset> for Rect {
    #[inline]
    fn add_assign(&mut self, offset: Offset) {
        self.pos += offset;
    }
}

impl std::ops::Sub<Offset> for Rect {
    type Output = Self;

    #[inline]
    fn sub(self, offset: Offset) -> Self {
        Rect::new(self.pos - offset, self.size)
    }
}
impl std::ops::SubAssign<Offset> for Rect {
    #[inline]
    fn sub_assign(&mut self, offset: Offset) {
        self.pos -= offset;
    }
}

#[cfg(feature = "accesskit")]
mod accesskit_impls {
    use super::{Coord, Offset, Rect};
    use crate::cast::{Cast, CastApprox, Conv, ConvApprox, Result};

    impl ConvApprox<accesskit::Point> for Coord {
        fn try_conv_approx(p: accesskit::Point) -> Result<Self> {
            Ok(Coord(p.x.try_cast_approx()?, p.y.try_cast_approx()?))
        }
    }

    impl Conv<Rect> for accesskit::Rect {
        fn try_conv(rect: Rect) -> Result<Self> {
            let p = rect.pos;
            let p2 = rect.pos2();
            Ok(accesskit::Rect {
                x0: p.0.try_cast()?,
                y0: p.1.try_cast()?,
                x1: p2.0.try_cast()?,
                y1: p2.1.try_cast()?,
            })
        }
    }

    impl ConvApprox<accesskit::Rect> for Rect {
        fn try_conv_approx(rect: accesskit::Rect) -> Result<Self> {
            let pos = Coord(rect.x0.try_cast_approx()?, rect.y0.try_cast_approx()?);
            let p2 = Coord(rect.x1.try_cast_approx()?, rect.y1.try_cast_approx()?);
            let size = (p2 - pos).cast();
            Ok(Rect { pos, size })
        }
    }

    impl ConvApprox<accesskit::Point> for Offset {
        fn try_conv_approx(point: accesskit::Point) -> Result<Self> {
            Ok(Offset(
                point.x.try_cast_approx()?,
                point.y.try_cast_approx()?,
            ))
        }
    }
}

mod winit_impls {
    use super::{Coord, Size};
    use crate::cast::{Cast, CastApprox, Conv, ConvApprox, Result};
    use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};

    impl<X: CastApprox<i32>> ConvApprox<PhysicalPosition<X>> for Coord {
        #[inline]
        fn try_conv_approx(pos: PhysicalPosition<X>) -> Result<Self> {
            Ok(Coord(pos.x.try_cast_approx()?, pos.y.try_cast_approx()?))
        }
    }

    impl<X: Cast<i32>> Conv<PhysicalSize<X>> for Size {
        #[inline]
        fn try_conv(size: PhysicalSize<X>) -> Result<Self> {
            Ok(Size(size.width.cast(), size.height.cast()))
        }
    }

    impl Coord {
        /// Convert to a "physical" [`winit::dpi::Position`]
        ///
        /// This implies that the [`Coord`] was calculated using the correct
        /// scale factor. Before the window has been constructed (when the
        /// scale factor is supposedly unknown) this should not be used.
        #[inline]
        pub fn as_physical(self) -> winit::dpi::Position {
            winit::dpi::Position::Physical(PhysicalPosition::new(self.0, self.1))
        }
    }

    impl Size {
        /// Convert to a "physical" [`winit::dpi::Size`]
        ///
        /// This implies that the [`Size`] was calculated using the correct
        /// scale factor. Before the window has been constructed (when the
        /// scale factor is supposedly unknown) this should not be used.
        #[inline]
        pub fn as_physical(self) -> winit::dpi::Size {
            let (w, h): (u32, u32) = self.cast();
            winit::dpi::Size::Physical(PhysicalSize::new(w, h))
        }

        /// Convert to a "logical" [`winit::dpi::Size`]
        ///
        /// This implies that the [`Size`] was calculated using `scale_factor = 1`.
        #[inline]
        pub fn as_logical(self) -> winit::dpi::Size {
            let (w, h) = (self.0 as f64, self.1 as f64);
            winit::dpi::Size::Logical(LogicalSize::new(w, h))
        }
    }
}
