// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS Theme lib

#![cfg_attr(feature = "gat", feature(generic_associated_types))]
#![cfg_attr(feature = "stack_dst", feature(unsize))]

#[cfg(all(feature = "gat", feature = "stack_dst"))]
compile_error!("Crate features 'gat' and 'stack_dst' are incompatible.");

mod col;
mod dim;
mod font;
#[cfg(all(feature = "stack_dst", not(feature = "gat")))]
mod multi;
#[cfg(all(feature = "stack_dst", not(feature = "gat")))]
mod theme_dst;
mod traits;

pub use kas;
pub use kas::theme::*;

pub use col::ThemeColours;
pub use dim::{Dimensions, DimensionsParams, DimensionsWindow};
pub use font::get_font;
#[cfg(all(feature = "stack_dst", not(feature = "gat")))]
pub use multi::MultiTheme;
#[cfg(all(feature = "stack_dst", not(feature = "gat")))]
pub use theme_dst::{ThemeDst, WindowDst};
pub use traits::{Theme, Window};
