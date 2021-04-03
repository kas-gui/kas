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
pub use kas::cast::{Cast, CastFloat, Conv, ConvFloat};
#[doc(no_inline)]
pub use kas::class::*;
#[doc(no_inline)]
pub use kas::dir::{Direction, Directional};
#[doc(no_inline)]
pub use kas::draw::{DrawHandle, DrawHandleExt, SizeHandle, ThemeApi};
#[doc(no_inline)]
pub use kas::event::{
    Event, Handler, Manager, ManagerState, Response, SendEvent, UpdateHandle, VoidMsg,
};
#[doc(no_inline)]
pub use kas::geom::{Coord, Offset, Rect, Size};
#[doc(no_inline)]
pub use kas::layout::{Align, AlignHints, AxisInfo, FrameRules, Margins, SizeRules, Stretch};
#[doc(no_inline)]
pub use kas::macros::*;
#[doc(no_inline)]
pub use kas::text::AccelString;
#[doc(no_inline)]
pub use kas::text::{EditableTextApi, Text, TextApi, TextApiExt};
#[doc(no_inline)]
pub use kas::WidgetId;
#[doc(no_inline)]
pub use kas::{Boxed, TkAction, WidgetExt};
#[doc(no_inline)]
pub use kas::{CoreData, LayoutData};
#[doc(no_inline)]
pub use kas::{Layout, Widget, WidgetChildren, WidgetConfig, WidgetCore};
