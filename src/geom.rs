// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Geometry data types

use kas::cast::Conv;
use kas::dir::Directional;
#[cfg(feature = "winit")]
use winit::dpi::{LogicalPosition, PhysicalPosition, PhysicalSize, Pixel};

mod vector;
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

            /// Return the minimum, componentwise
            #[inline]
            pub fn min(self, other: Self) -> Self {
                Self(self.0.min(other.0), self.1.min(other.1))
            }

            /// Return the maximum, componentwise
            #[inline]
            pub fn max(self, other: Self) -> Self {
                Self(self.0.max(other.0), self.1.max(other.1))
            }

            /// Return the value clamped to the given `min` and `max`
            ///
            /// In the case that `min > max`, the `min` value is returned.
            #[inline]
            pub fn clamp(self, min: Self, max: Self) -> Self {
                self.min(max).max(min)
            }

            /// Return the transpose (swap x and y values)
            #[inline]
            pub fn transpose(self) -> Self {
                Self(self.1, self.0)
            }

            /// Return the result of component-wise multiplication
            #[inline]
            pub fn cwise_mul(self, rhs: Self) -> Self {
                Self(self.0 * rhs.0, self.1 * rhs.1)
            }

            /// Return the result of component-wise division
            #[inline]
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
            /// This merely extracts the horizontal component. It never negates
            /// it, even if the direction is reversed.
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

        impl From<(i32, i32)> for $T {
            #[inline]
            fn from(v: (i32, i32)) -> Self {
                Self(v.0, v.1)
            }
        }
    };
}

/// A 2D coordinate, also known as a point
///
/// A coordinate (or point) is an absolute position. One cannot add a point to
/// a point. The difference between two points is an [`Offset`].
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
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

    /// Convert from a logical position
    #[cfg(feature = "winit")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "winit")))]
    pub fn from_logical<X: Pixel>(logical: LogicalPosition<X>, dpi_factor: f64) -> Self {
        let pos = PhysicalPosition::<i32>::from_logical(logical, dpi_factor);
        let pos: (i32, i32) = pos.into();
        Coord(pos.0, pos.1)
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

impl From<Coord> for kas_text::Vec2 {
    fn from(pos: Coord) -> kas_text::Vec2 {
        Vec2::from(pos).into()
    }
}

#[cfg(feature = "winit")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "winit")))]
impl<X: Pixel> From<PhysicalPosition<X>> for Coord {
    #[inline]
    fn from(pos: PhysicalPosition<X>) -> Coord {
        let pos: (i32, i32) = pos.cast::<i32>().into();
        Coord(pos.0, pos.1)
    }
}

#[cfg(feature = "winit")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "winit")))]
impl<X: Pixel> From<Coord> for PhysicalPosition<X> {
    #[inline]
    fn from(coord: Coord) -> PhysicalPosition<X> {
        let pos: PhysicalPosition<i32> = (coord.0, coord.1).into();
        pos.cast()
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
/// This may be converted to [`Offset`] with `from` / `into`.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Size(pub i32, pub i32);

impl_common!(Size);

impl Size {
    /// Construct
    ///
    /// In debug mode, this asserts that components are non-negative.
    #[inline]
    pub fn new(w: i32, h: i32) -> Self {
        debug_assert!(w >= 0 && h >= 0, "Size::new({}, {}): negative value", w, h);
        Self(w, h)
    }

    /// Construct, using the same value on all axes
    #[inline]
    pub fn splat(n: i32) -> Self {
        debug_assert!(n >= 0, "Size::splat({}): negative value", n);
        Self(n, n)
    }

    /// Subtraction, clamping the result to 0 or greater
    #[inline]
    pub fn clamped_sub(self, rhs: Self) -> Self {
        // This impl should aid vectorisation. We avoid Sub impl because of its check.
        Size(self.0 - rhs.0, self.1 - rhs.1).max(Size::ZERO)
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

impl std::ops::Sub for Size {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        debug_assert!(
            self.0 >= rhs.0 && self.1 >= rhs.1,
            "Size::sub: expected lhs >= rhs"
        );
        Self(self.0 - rhs.0, self.1 - rhs.1)
    }
}
impl std::ops::SubAssign for Size {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        debug_assert!(
            self.0 >= rhs.0 && self.1 >= rhs.1,
            "Size::sub: expected lhs >= rhs"
        );
        self.0 -= rhs.0;
        self.1 -= rhs.1;
    }
}

impl std::ops::Mul<i32> for Size {
    type Output = Self;

    #[inline]
    fn mul(self, x: i32) -> Self {
        debug_assert!(x >= 0);
        Size(self.0 * x, self.1 * x)
    }
}
impl std::ops::Div<i32> for Size {
    type Output = Self;

    #[inline]
    fn div(self, x: i32) -> Self {
        debug_assert!(x >= 0);
        Size(self.0 / x, self.1 / x)
    }
}

impl std::ops::Mul<f32> for Size {
    type Output = Self;

    #[inline]
    fn mul(self, x: f32) -> Self {
        debug_assert!(x >= 0.0);
        let v = Vec2::from(self) * x;
        v.into()
    }
}
impl std::ops::Div<f32> for Size {
    type Output = Self;

    #[inline]
    fn div(self, x: f32) -> Self {
        debug_assert!(x >= 0.0);
        let v = Vec2::from(self) / x;
        v.into()
    }
}

impl From<Offset> for Size {
    fn from(v: Offset) -> Self {
        debug_assert!(v.0 >= 0 && v.1 >= 0, "Size::from({:?}): negative value", v);
        Self(v.0, v.1)
    }
}

// used for marigns
impl From<(u16, u16)> for Size {
    fn from(v: (u16, u16)) -> Self {
        Self(i32::conv(v.0), i32::conv(v.1))
    }
}

impl From<(u32, u32)> for Size {
    fn from(v: (u32, u32)) -> Self {
        Self(i32::conv(v.0), i32::conv(v.1))
    }
}

impl From<Size> for (u32, u32) {
    fn from(size: Size) -> (u32, u32) {
        (u32::conv(size.0), u32::conv(size.1))
    }
}

impl From<Size> for kas_text::Vec2 {
    fn from(size: Size) -> kas_text::Vec2 {
        debug_assert!(size.0 >= 0 && size.1 >= 0);
        Vec2::from(size).into()
    }
}

#[cfg(feature = "winit")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "winit")))]
impl<X: Pixel> From<PhysicalSize<X>> for Size {
    #[inline]
    fn from(size: PhysicalSize<X>) -> Size {
        let size: (i32, i32) = size.cast::<i32>().into();
        debug_assert!(size.0 >= 0 && size.1 >= 0);
        Size(size.0, size.1)
    }
}

#[cfg(feature = "winit")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "winit")))]
impl<X: Pixel> From<Size> for PhysicalSize<X> {
    #[inline]
    fn from(size: Size) -> PhysicalSize<X> {
        debug_assert!(size.0 >= 0 && size.1 >= 0);
        let pos: PhysicalSize<i32> = (size.0, size.1).into();
        pos.cast()
    }
}

#[cfg(feature = "winit")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "winit")))]
impl From<Size> for winit::dpi::Size {
    #[inline]
    fn from(size: Size) -> winit::dpi::Size {
        debug_assert!(size.0 >= 0 && size.1 >= 0);
        winit::dpi::Size::Physical((size.0, size.1).into())
    }
}

/// A `(x, y)` offset, also known as a **vector**
///
/// This is a relative position. It can be added to or subtracted from a
/// [`Coord`], and it can be added to or subtracted from itself. It can be
/// negative. It can be multiplied by a scalar.
///
/// This may be converted to [`Size`] with `from` / `into`.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
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

impl std::ops::Mul<f32> for Offset {
    type Output = Self;

    #[inline]
    fn mul(self, x: f32) -> Self {
        let v = Vec2::from(self) * x;
        v.into()
    }
}
impl std::ops::Div<f32> for Offset {
    type Output = Self;

    #[inline]
    fn div(self, x: f32) -> Self {
        let v = Vec2::from(self) / x;
        v.into()
    }
}

impl From<Size> for Offset {
    fn from(v: Size) -> Self {
        Self(v.0, v.1)
    }
}

impl From<Offset> for kas_text::Vec2 {
    fn from(size: Offset) -> kas_text::Vec2 {
        Vec2::from(size).into()
    }
}

/// An axis-aligned rectangular region
///
/// The region is defined by a point `pos` and an extent `size`, allowing easy
/// translations. It is empty unless `size` is positive on both axes.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
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
        if pos.le(pos2) {
            Some(Rect::new(pos, (pos2 - pos).into()))
        } else {
            None
        }
    }

    /// Shrink self in all directions by the given `n`
    #[inline]
    pub fn shrink(&self, n: i32) -> Rect {
        let pos = self.pos + Offset::splat(n);
        let size = self.size.clamped_sub(Size::splat(n + n));
        Rect { pos, size }
    }

    /// Expand self in all directions by the given `n`
    #[inline]
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
