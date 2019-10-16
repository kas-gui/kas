// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Utility data types

/// An `(x, y)` coordinate.
pub type Coord = (i32, i32);

/// A `(w, h)` size.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Size(pub u32, pub u32);

impl Size {
    /// A size of `(0, 0)`
    pub fn zero() -> Size {
        Size(0, 0)
    }
    
    /// Maximum possible size
    pub fn max() -> Size {
        Size(std::u32::MAX, std::u32::MAX)
    }
}

impl From<(u32, u32)> for Size {
    fn from(size: (u32, u32)) -> Size {
        Size(size.0, size.1)
    }
}

/// Child widget identifier
//TODO: make a tuple struct?
pub type WidgetId = u32;

/// A rectangular region.
#[derive(Clone, Copy, Default, Debug)]
pub struct Rect {
    pub pos: Coord,
    pub size: Size, // TODO: more efficient to store pos+size ?
}

impl Rect {
    /// Check whether the given coordinate is contained within this rect
    pub fn contains(&self, c: Coord) -> bool {
        c.0 >= self.pos.0
            && c.0 < self.pos.0 + (self.size.0 as i32)
            && c.1 >= self.pos.1
            && c.1 < self.pos.1 + (self.size.1 as i32)
    }

    /// Get pos as `f32` tuple
    pub fn pos_f32(&self) -> (f32, f32) {
        (self.pos.0 as f32, self.pos.1 as f32)
    }

    /// Get size as `f32` tuple
    pub fn size_f32(&self) -> (f32, f32) {
        (self.size.0 as f32, self.size.1 as f32)
    }
}
