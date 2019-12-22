// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Geometry data types

#[cfg(feature = "winit")]
use winit::dpi::{PhysicalPosition, PhysicalSize};

/// An `(x, y)` coordinate.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Coord(pub i32, pub i32);

impl Coord {
    /// A coord of `(0, 0)`
    pub const ZERO: Coord = Coord(0, 0);

    /// Return the minimum, componentwise
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Coord(self.0.min(other.0), self.1.min(other.1))
    }

    /// Return the maximum, componentwise
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Coord(self.0.max(other.0), self.1.max(other.1))
    }
}

impl From<(i32, i32)> for Coord {
    #[inline]
    fn from(coord: (i32, i32)) -> Coord {
        Coord(coord.0, coord.1)
    }
}

impl From<Size> for Coord {
    #[inline]
    fn from(size: Size) -> Coord {
        Coord(size.0 as i32, size.1 as i32)
    }
}

impl std::ops::Add for Coord {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Coord(self.0 + other.0, self.1 + other.1)
    }
}

impl std::ops::Sub for Coord {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        Coord(self.0 - other.0, self.1 - other.1)
    }
}

impl std::ops::Add<Size> for Coord {
    type Output = Self;

    #[inline]
    fn add(self, other: Size) -> Self {
        Coord(self.0 + other.0 as i32, self.1 + other.1 as i32)
    }
}

#[cfg(feature = "winit")]
impl From<PhysicalPosition> for Coord {
    #[inline]
    fn from(pos: PhysicalPosition) -> Coord {
        let pos: (i32, i32) = pos.into();
        Coord(pos.0, pos.1)
    }
}

#[cfg(feature = "winit")]
impl From<Coord> for PhysicalPosition {
    #[inline]
    fn from(coord: Coord) -> PhysicalPosition {
        (coord.0, coord.1).into()
    }
}

impl std::ops::AddAssign<Size> for Coord {
    #[inline]
    fn add_assign(&mut self, rhs: Size) {
        self.0 += rhs.0 as i32;
        self.1 += rhs.1 as i32;
    }
}

/// A `(w, h)` size.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Size(pub u32, pub u32);

impl Size {
    /// A size of `(0, 0)`
    pub const ZERO: Size = Size(0, 0);

    /// Uniform size in each dimension
    #[inline]
    pub const fn uniform(v: u32) -> Self {
        Size(v, v)
    }

    /// Return the minimum, componentwise
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Size(self.0.min(other.0), self.1.min(other.1))
    }

    /// Return the maximum, componentwise
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Size(self.0.max(other.0), self.1.max(other.1))
    }
}

impl From<(u32, u32)> for Size {
    fn from(size: (u32, u32)) -> Size {
        Size(size.0, size.1)
    }
}

#[cfg(feature = "winit")]
impl From<PhysicalSize> for Size {
    #[inline]
    fn from(size: PhysicalSize) -> Size {
        let size: (u32, u32) = size.into();
        Size(size.0, size.1)
    }
}

#[cfg(feature = "winit")]
impl From<Size> for PhysicalSize {
    #[inline]
    fn from(size: Size) -> PhysicalSize {
        (size.0, size.1).into()
    }
}

impl std::ops::Add for Size {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Size(self.0 + other.0, self.1 + other.1)
    }
}

impl std::ops::Sub for Size {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        Size(self.0 - other.0, self.1 - other.1)
    }
}

impl std::ops::AddAssign for Size {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

impl std::ops::SubAssign for Size {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
        self.1 -= rhs.1;
    }
}

/// A rectangular region.
#[derive(Clone, Copy, Default, Debug)]
pub struct Rect {
    pub pos: Coord,
    pub size: Size,
}

impl Rect {
    /// Check whether the given coordinate is contained within this rect
    #[inline]
    pub fn contains(&self, c: Coord) -> bool {
        c.0 >= self.pos.0
            && c.0 < self.pos.0 + (self.size.0 as i32)
            && c.1 >= self.pos.1
            && c.1 < self.pos.1 + (self.size.1 as i32)
    }
}

impl std::ops::Add<Coord> for Rect {
    type Output = Self;

    #[inline]
    fn add(self, offset: Coord) -> Self {
        let pos = self.pos + offset;
        Rect {
            pos,
            size: self.size,
        }
    }
}

impl std::ops::Sub<Coord> for Rect {
    type Output = Self;

    #[inline]
    fn sub(self, offset: Coord) -> Self {
        let pos = self.pos - offset;
        Rect {
            pos,
            size: self.size,
        }
    }
}
