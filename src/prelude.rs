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

#[doc(no_inline)]
pub use kas::draw::{DrawHandle, SizeHandle};
#[doc(no_inline)]
pub use kas::event::{Event, Handler, Manager, ManagerState, Response, SendEvent, VoidMsg};
#[doc(no_inline)]
pub use kas::geom::{Coord, Rect, Size};
#[doc(no_inline)]
pub use kas::layout::{AxisInfo, Margins, SizeRules, StretchPolicy};
#[doc(no_inline)]
pub use kas::macros::*;
#[doc(no_inline)]
pub use kas::string::{AccelString, LabelString};
#[doc(no_inline)]
pub use kas::text::{PreparedText, PreparedTextExt, RichText};
#[doc(no_inline)]
pub use kas::{class, draw, event, geom, layout, widget};
#[doc(no_inline)]
pub use kas::{Align, AlignHints, Direction, Directional, WidgetId};
#[doc(no_inline)]
pub use kas::{Boxed, TkAction, TkWindow};
#[doc(no_inline)]
pub use kas::{CloneTo, Layout, ThemeApi, Widget, WidgetChildren, WidgetConfig, WidgetCore};
#[doc(no_inline)]
pub use kas::{CoreData, LayoutData};
