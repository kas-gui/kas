// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! View widgets
//!
//! View widgets exist as a view over some shared data.

mod data_traits;
mod filter;
mod list_view;
mod shared_data;
mod single_view;
mod view_widget;

pub use data_traits::{ListData, SingleData, SingleDataMut};
pub use filter::{Filter, FilteredList, SimpleCaseInsensitiveFilter};
pub use list_view::{ListMsg, ListView, SelectionMode};
pub use shared_data::{SharedConst, SharedRc};
pub use single_view::SingleView;
pub use view_widget::{DefaultView, ViewWidget};
