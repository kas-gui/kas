// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS prelude
//!
//! This module allows convenient importation of common unabiguous items:
//! ```
//! use kas::prelude::*;
//! ```
//!
//! This prelude may be more useful when implementing widgets than when simply
//! using widgets in a GUI.

pub use kas::geom::{Coord, Rect, Size};
pub use kas::macros::*;
pub use kas::{class, draw, event, geom, layout, widget};
pub use kas::{Align, AlignHints, Direction, Directional, Horizontal, Vertical, WidgetId};
pub use kas::{CloneTo, Layout, ThemeApi, Widget, WidgetChildren, WidgetConfig, WidgetCore};
pub use kas::{CoreData, LayoutData};
pub use kas::{CowString, CowStringL};
pub use kas::{TkAction, TkWindow};
