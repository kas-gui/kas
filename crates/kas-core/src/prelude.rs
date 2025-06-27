// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS core prelude
//!
//! It is recommended to use `kas::prelude` instead, which is an extension of
//! this crate's prelude.

#[doc(no_inline)] pub use crate::Action;
#[doc(no_inline)] pub use crate::cast::traits::*;
#[doc(no_inline)]
pub use crate::dir::{Direction, Directional};
#[doc(no_inline)]
pub use crate::event::{ConfigCx, Event, EventCx, EventState, IsUsed, Unused, Used};
#[doc(no_inline)]
pub use crate::geom::{Coord, Offset, Rect, Size};
#[doc(no_inline)]
pub use crate::layout::{Align, AlignHints, AlignPair, AxisInfo, SizeRules, Stretch};
#[doc(no_inline)] pub use crate::text::AccessString;
#[doc(no_inline)] pub use crate::theme::{DrawCx, SizeCx};
#[doc(no_inline)]
pub use crate::{Events, Layout, Tile, TileExt, Widget, Window, WindowCommand};
#[doc(no_inline)] pub use crate::{HasId, Id};
#[doc(no_inline)] pub use crate::{Node, Scrollable};
#[doc(no_inline)]
pub use crate::{autoimpl, impl_anon, impl_default, impl_scope, impl_self};
#[doc(no_inline)]
pub use crate::{widget_index, widget_set_rect};
