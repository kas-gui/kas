// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text functionality
//!
//! This module is built over the [KAS Text] API; several items here are direct
//! re-exports.
//!
//! [KAS Text]: https://github.com/kas-gui/kas-text/

pub use kas_text::{
    Align, DPU, Direction, Line, LineIterator, MarkerPos, MarkerPosIter, NotReady, Status,
    TextDisplay, Vec2, fonts,
};

mod display;
pub mod format;
/// Glyph rastering
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
pub mod raster;
mod selection;
mod string;
mod text;

pub use display::ConfiguredDisplay;
pub use selection::{CursorRange, SelectionHelper};
pub use string::AccessString;
pub use text::Text;
