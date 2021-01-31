// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Geometry data types

use kas::conv::Conv;
use kas::{Direction, Directional};
#[cfg(feature = "winit")]
use winit::dpi::{LogicalPosition, PhysicalPosition, PhysicalSize, Pixel};

mod vector;
pub use vector::{DVec2, Quad, Vec2, Vec3};

macro_rules! impl_common {
    ($T:ty) => {
        impl $T {
            /// The constant `(0, 0)`
            pub const ZERO: Self = Self(0, 0);

            /// Set the same value on all axes
            #[inline]
            pub const fn splat(n: i32) -> Self {
                Self(n, n)
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

            /// Return the transpose (swap x and y values)
            #[inline]
            pub fn transpose(self) -> Self {
                Self(self.1, self.0)
            }

            /// Extract one component, based on a direction
            ///
            /// Panics if direction is reversed. We should decide whether to
            /// negate in this case (maybe for coord and offset but not for size).
            #[inline]
            pub fn extract<D: Directional>(self, dir: D) -> i32 {
                match dir.as_direction() {
                    Direction::Right => self.0,
                    Direction::Down => self.1,
                    Direction::Left | Direction::Up => {
                        panic!("extract not defined for left/up directions!")
                    }
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

/// An `(x, y)` coordinate, also known as a **point**
///
/// This is not a vector type: one cannot add a point to a point.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Coord(pub i32, pub i32);

impl_common!(Coord);

impl Coord {
    /// Convert from a logical position
    #[cfg(feature = "winit")]
    pub fn from_logical<X: Pixel>(logical: LogicalPosition<X>, dpi_factor: f64) -> Self {
        let pos = PhysicalPosition::<i32>::from_logical(logical, dpi_factor);
        let pos: (i32, i32) = pos.into();
        Coord(pos.0, pos.1)
    }
}

impl std::ops::Sub for Coord {
    type Output = Size;

    #[inline]
    fn sub(self, other: Self) -> Size {
        Size(self.0 - other.0, self.1 - other.1)
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
        kas_text::Vec2(pos.0 as f32, pos.1 as f32)
    }
}

#[cfg(feature = "winit")]
impl<X: Pixel> From<PhysicalPosition<X>> for Coord {
    #[inline]
    fn from(pos: PhysicalPosition<X>) -> Coord {
        let pos: (i32, i32) = pos.cast::<i32>().into();
        Coord(pos.0, pos.1)
    }
}

#[cfg(feature = "winit")]
impl<X: Pixel> From<Coord> for PhysicalPosition<X> {
    #[inline]
    fn from(coord: Coord) -> PhysicalPosition<X> {
        let pos: PhysicalPosition<i32> = (coord.0, coord.1).into();
        pos.cast()
    }
}

/// A `(w, h)` size, also known as an **extent**
///
/// This is a vector type: the difference between one point and another is a
/// vector, represented using this type, `Size`. Components may be negative.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Size(pub i32, pub i32);

impl_common!(Size);

impl Size {
    /// Return the value clamped to the given `min` and `max`
    ///
    /// In the case that `min > max`, the `min` value is returned.
    #[inline]
    pub fn clamp(self, min: Self, max: Self) -> Self {
        self.min(max).max(min)
    }

    /// Saturating sub
    #[inline]
    pub fn saturating_sub(self, other: Self) -> Self {
        let w = self.0.saturating_sub(other.0);
        let h = self.1.saturating_sub(other.1);
        Size(w, h)
    }
}

#[cfg(feature = "winit")]
impl<X: Pixel> From<PhysicalSize<X>> for Size {
    #[inline]
    fn from(size: PhysicalSize<X>) -> Size {
        let size: (i32, i32) = size.cast::<i32>().into();
        Size(size.0, size.1)
    }
}

#[cfg(feature = "winit")]
impl<X: Pixel> From<Size> for PhysicalSize<X> {
    #[inline]
    fn from(size: Size) -> PhysicalSize<X> {
        let pos: PhysicalSize<i32> = (size.0, size.1).into();
        pos.cast()
    }
}

#[cfg(feature = "winit")]
impl From<Size> for winit::dpi::Size {
    #[inline]
    fn from(size: Size) -> winit::dpi::Size {
        winit::dpi::Size::Physical((size.0, size.1).into())
    }
}

impl From<Size> for kas_text::Vec2 {
    fn from(size: Size) -> kas_text::Vec2 {
        kas_text::Vec2(size.0 as f32, size.1 as f32)
    }
}

impl std::ops::Add<Coord> for Size {
    type Output = Coord;

    #[inline]
    fn add(self, other: Coord) -> Coord {
        Coord(self.0 + other.0, self.1 + other.1)
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
    fn sub(self, other: Self) -> Self {
        Size(self.0 - other.0, self.1 - other.1)
    }
}
impl std::ops::SubAssign for Size {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
        self.1 -= rhs.1;
    }
}

impl std::ops::Mul<i32> for Size {
    type Output = Self;

    #[inline]
    fn mul(self, x: i32) -> Self {
        Size(self.0 * x, self.1 * x)
    }
}
impl std::ops::Div<i32> for Size {
    type Output = Self;

    #[inline]
    fn div(self, x: i32) -> Self {
        Size(self.0 / x, self.1 / x)
    }
}

impl std::ops::Mul<f32> for Size {
    type Output = Self;

    #[inline]
    fn mul(self, x: f32) -> Self {
        Size((self.0 as f32 * x) as i32, (self.1 as f32 * x) as i32)
    }
}
impl std::ops::Div<f32> for Size {
    type Output = Self;

    #[inline]
    fn div(self, x: f32) -> Self {
        Size((self.0 as f32 / x) as i32, (self.1 as f32 / x) as i32)
    }
}

// used for marigns
impl From<(u16, u16)> for Size {
    fn from(v: (u16, u16)) -> Self {
        Self(i32::conv(v.0), i32::conv(v.1))
    }
}

/// An axis-aligned rectangular region
///
/// The region is defined by a point `pos` and an extent `size`, allowing easy
/// translations. It is empty unless `size` is positive on both axes.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Rect {
    pub pos: Coord,
    pub size: Size,
}

impl Rect {
    /// Construct from a [`Coord`] and [`Size`]
    #[inline]
    pub fn new(pos: Coord, size: Size) -> Self {
        Rect { pos, size }
    }

    /// Get second point (pos + size)
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

    /// Shrink self in all directions by the given `n`
    #[inline]
    pub fn shrink(&self, n: i32) -> Rect {
        let pos = self.pos + Size::splat(n);
        let w = self.size.0.saturating_sub(n + n);
        let h = self.size.1.saturating_sub(n + n);
        let size = Size(w, h);
        Rect { pos, size }
    }
}

impl std::ops::Add<Size> for Rect {
    type Output = Self;

    #[inline]
    fn add(self, offset: Size) -> Self {
        Rect::new(self.pos + offset, self.size)
    }
}
impl std::ops::AddAssign<Size> for Rect {
    #[inline]
    fn add_assign(&mut self, offset: Size) {
        self.pos += offset;
    }
}

impl std::ops::Sub<Size> for Rect {
    type Output = Self;

    #[inline]
    fn sub(self, offset: Size) -> Self {
        Rect::new(self.pos - offset, self.size)
    }
}
impl std::ops::SubAssign<Size> for Rect {
    #[inline]
    fn sub_assign(&mut self, offset: Size) {
        self.pos -= offset;
    }
}
