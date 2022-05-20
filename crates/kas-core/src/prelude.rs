// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS core prelude
//!
//! It is recommended to use `kas::prelude` instead, which is an extension of
//! this crate's prelude.

#[doc(no_inline)]
pub use crate::cast::traits::*;
#[doc(no_inline)]
pub use crate::class::*;
#[doc(no_inline)]
pub use crate::dir::{Direction, Directional};
#[doc(no_inline)]
pub use crate::draw::{DrawShared, ImageHandle};
#[doc(no_inline)]
pub use crate::event::{components, Event, EventMgr, EventState, Response, UpdateId};
#[doc(no_inline)]
pub use crate::geom::{Coord, Offset, Rect, Size};
#[doc(no_inline)]
pub use crate::layout::{
    Align, AlignHints, AxisInfo, LogicalSize, Margins, SetRectMgr, SizeRules, Stretch,
};
#[doc(no_inline)]
pub use crate::macros::*;
#[doc(no_inline)]
pub use crate::text::AccelString;
#[doc(no_inline)]
pub use crate::text::{EditableTextApi, Text, TextApi, TextApiExt};
#[doc(no_inline)]
pub use crate::theme::{DrawMgr, SizeMgr, ThemeControl};
#[doc(no_inline)]
pub use crate::CoreData;
#[doc(no_inline)]
pub use crate::TkAction;
#[doc(no_inline)]
pub use crate::WidgetId;
#[doc(no_inline)]
pub use crate::{Layout, Widget, WidgetChildren, WidgetCore, WidgetExt, Window};
