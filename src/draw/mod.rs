// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing API
//!
//! This module includes abstractions over the drawing API and some associated
//! types.

mod colour;
mod traits;
mod vector;

pub use colour::Colour;
pub use traits::{DrawFlat, DrawSquare, DrawRound};
pub use vector::Vec2;
