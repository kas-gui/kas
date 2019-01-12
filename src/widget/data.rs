// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Utility data types

/// An `(x, y)` coordinate or a size.
pub type Coord = (i32, i32);

/// A rectangular region.
#[derive(Clone, Default, Debug)]
pub struct Rect {
    pub pos: Coord,
    pub size: Coord,
}
