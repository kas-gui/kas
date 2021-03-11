// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! View widgets and shared data
//!
//! So called "view widgets" allow separation of data and view. The system has
//! some similarities with the Model-View-Controller (MVC) pattern, but with
//! different separations of responsibility. Perhaps we should instead call the
//! pattern Model-View-Driver?
//!
//! # Shared data and *model*
//!
//! A family of data model traits is available in [`kas::data`]. Shared data
//! must implement one or more of these traits for use with view widgets.
//!
//! In MVC terminology, the implementation of these traits over some data may
//! be called the model.
//!
//! ## Adapters
//!
//! -   [`FilteredList`] presents a filtered list over a [`ListData`]
//!
//! # View widgets and drivers
//!
//! Standard widgets may be used to view data items, but to construct these a
//! *driver* is required. These implement the [`Driver`] trait which constructs
//! widgets from data items and optionally also the reverse binding.
//!
//! The user may implement a [`Driver`] or may use a standard one:
//!
//! -   [`driver::Default`] constructs a default view widget over various data types
//! -   [`driver::CheckBox`] and [`driver::RadioBox`] support the `bool` type
//! -   [`driver::Slider`] constructs a slider with a fixed range
//!
//! In MVC terminology, the driver is perhaps most similar to the controller,
//! while the widgets constructed by the driver are the view, and yet ...
//!
//! # Views
//!
//! Something else is required to construct one or more view widgets over the
//! data model as well as to perform event handling (at a minimum, forwarding
//! events to the appropriate view widgets), and that thing is here referred to
//! as the **view** (MVC terminology, it is part view and part controller, while
//! not being the whole of either).
//!
//! These *views* are widgets and provide additional services:
//!
//! -   updating view widgets when the model is changed
//! -   notifying other users of the data when view widgets update the data
//! -   ideally allowing `O(v)` performance where `v` is the number of visible
//!     data items, thus allowing good scaling to large data sets (this depends
//!     on the performance of the model)
//! -   supporting scrolling (see [`super::Scrollable`])
//! -   supporting item selection
//! -   controlling scrolling and selection via otherwise unhandled events
//!
//! The following views are provided:
//!
//! -   [`SingleView`] creates a view over a [`SingleData`] object (no scrolling
//!     or selection support)
//! -   [`ListView`] creates a scrollable list view over a [`ListData`] object

#[allow(unused)]
use kas::data::{ListData, SingleData};

mod filter;
mod list_view;
mod shared_data;
mod single_view;

pub mod driver;

pub use driver::Driver;
pub use filter::{Filter, FilteredList, SimpleCaseInsensitiveFilter};
pub use list_view::{ListMsg, ListView, SelectionMode};
pub use shared_data::SharedRc;
pub use single_view::SingleView;
