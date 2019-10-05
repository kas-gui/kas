// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Utility data types

/// An `(x, y)` coordinate.
pub type Coord = (i32, i32);

/// A `(w, h)` size.
pub type Size = (u32, u32);

/// A rectangular region.
#[derive(Clone, Default, Debug)]
pub struct Rect {
    pub pos: Coord,
    pub size: Size,    // TODO: more efficient to store pos+size ?
}

impl Rect {
    /// Check whether the given coordinate is contained within this rect
    pub fn contains(&self, c: Coord) -> bool {
        c.0 >= self.pos.0 && c.0 < self.pos.0 + (self.size.0 as i32) &&
        c.1 >= self.pos.1 && c.1 < self.pos.1 + (self.size.1 as i32)
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
