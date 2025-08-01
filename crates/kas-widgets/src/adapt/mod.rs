// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Adapter widgets (wrappers)

mod adapt;
mod adapt_cx;
mod adapt_events;
mod adapt_widget;
mod reserve;
mod with_label;

pub use adapt::{Adapt, Map};
pub use adapt_cx::{AdaptConfigCx, AdaptEventCx};
pub use adapt_events::AdaptEvents;
pub use adapt_widget::*;
#[doc(inline)] pub use kas::hidden::MapAny;
pub use kas::hidden::{Align, Pack};
pub use reserve::{Margins, Reserve};
pub use with_label::{WithHiddenLabel, WithLabel};
