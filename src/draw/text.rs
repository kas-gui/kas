// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text-drawing API

use super::{Colour, Pass};
use crate::geom::Vec2;
use crate::text::PreparedText;

/// Abstraction over text rendering
///
/// Note: the current API is designed to meet only current requirements since
/// changes are expected to support external font shaping libraries.
pub trait DrawText {
    /// Draw text
    fn text(&mut self, pass: Pass, pos: Vec2, col: Colour, text: &PreparedText);
}
