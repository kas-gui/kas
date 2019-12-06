// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing API
//!
//! This module includes abstractions over the drawing API and some associated
//! types.
//!
//! All draw operations are batched and do not happen immediately. Each
//! [`Style`] of drawing may batch operations independently of other styles or
//! may share batching with another style. Roughly speaking, later [`Style`]s
//! are drawn later, but draw order is implementation defined.

mod colour;
mod traits;
mod vector;

pub use colour::Colour;
pub use traits::{Draw, Style};
pub use vector::{Quad, Vec2};
