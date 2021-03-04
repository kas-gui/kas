// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! View widgets and shared data
//!
//! So called "view widgets" are able to form a view over some shared data.
//!
//! # Shared data
//!
//! Shared data must implement one of a family of traits:
//!
//! -   [`SingleData`] supports viewing a single item ("datum")
//! -   [`ListData`] supports viewing an ordered sequence of items (requires
//!     that items are ordered and that items can be accessed by some key type)
//!
//! Each of these has a "Mut" variant, supporting direct modification of values
//! when a mutable reference is available. In other cases, the `update` method
//! *may* support modification.
//!
//! ## Filters
//!
//! -   [`FilteredList`] is a filtered view over [`ListData`]
//!
//! # Viewing data via widgets
//!
//! The [`View`] trait provides a mechanism for constructing and updating
//! arbitrary widgets from a data source.
//!
//! # View widget drivers
//!
//! Building on all the above, the **view widgets** combine data and a driver:
//!
//! -   [`SingleView`] creates a view over a [`SingleData`] object
//! -   [`ListView`] creates a scrollable list view over a [`ListData`] object.
//!     Performance is potentially bounded by O(v) in all operations where `v`
//!     is the number of visible items (depending on the [`ListData`] object).

mod data_traits;
mod filter;
mod list_view;
mod shared_data;
mod single_view;
mod view_widget;

pub use data_traits::{ListData, ListDataMut, SingleData, SingleDataMut};
pub use filter::{Filter, FilteredList, SimpleCaseInsensitiveFilter};
pub use list_view::{ListMsg, ListView, SelectionMode};
pub use shared_data::SharedRc;
pub use single_view::SingleView;
pub use view_widget::{CheckBoxView, RadioBoxBareView, RadioBoxView, SliderView};
pub use view_widget::{DefaultView, View, WidgetView};
