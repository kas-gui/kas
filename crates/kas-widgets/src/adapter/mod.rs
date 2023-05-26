// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Adapter widgets (wrappers)

mod adapt_events;
mod adapt_widget;
mod reserve;
mod with_align;
mod with_label;

pub use adapt_events::OnUpdate;
pub use adapt_widget::*;
pub use reserve::{FnSizeRules, Reserve, ReserveP};
pub use with_align::WithAlign;
pub use with_label::WithLabel;
