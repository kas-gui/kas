// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Kas core widgets
//!
//! This is a minimal widget library intended to cover the needs of windows and
//! window decorations.

pub mod adapt;
mod decorations;
mod label;
mod mark;

#[doc(inline)] pub use decorations::*;
pub use label::Label;
pub use mark::{Mark, MarkButton};
