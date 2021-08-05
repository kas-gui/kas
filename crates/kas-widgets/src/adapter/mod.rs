// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Adapter widgets (wrappers)

mod label;
mod map;
mod reserve;
mod widget_ext;

pub use label::WithLabel;
pub use map::MapResponse;
pub use reserve::{Reserve, ReserveP};
pub use widget_ext::*;
