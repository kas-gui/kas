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
//! Shared data must implement several traits, namely those in
//! [`kas::updatable`] and one of the "view" traits: [`SingleData`],
//! [`ListData`] or [`MatrixData`]. These traits together form the "model".
//!
//! For simpler cases it is not always necessary to implement your own shared
//! data type, for example `SharedRc<i32>` implements [`SingleData`] and
//! `&'static [&'static str]` implements [`ListData`]. The [`SharedRc`] type
//! provides the [`UpdateHandle`] required to synchronise views.
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
//! -   [`driver::DefaultNav`] is a variant of the above, ensuring items support
//!     keyboard navigation (e.g. useful to allow selection of static items)
//! -   [`driver::CheckBox`] and [`driver::RadioBox`] support the `bool` type
//! -   [`driver::Slider`] constructs a slider with a fixed range
//!
//! In MVC terminology, the driver is perhaps most similar to the controller,
//! while the widgets constructed by the driver are the view, but this analogy
//! is not quite accurate.
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
use kas::event::UpdateHandle;
use kas::macros::VoidMsg;
#[allow(unused)]
use kas::updatable::{FilteredList, ListData, MatrixData, SharedRc, SingleData};
use thiserror::Error;

mod filter_list;
mod list_view;
mod matrix_view;
mod single_view;

pub mod driver;

pub use driver::Driver;
pub use filter_list::FilterListView;
pub use list_view::ListView;
pub use matrix_view::MatrixView;
pub use single_view::SingleView;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum PressPhase {
    None,
    Start(kas::geom::Coord),
    Pan,
}

/// Selection mode used by [`ListView`]
#[derive(Clone, Copy, Debug, VoidMsg)]
pub enum SelectionMode {
    None,
    Single,
    Multiple,
}
impl Default for SelectionMode {
    fn default() -> Self {
        SelectionMode::None
    }
}

/// Selection errors
#[derive(Error, Debug)]
pub enum SelectionError {
    #[error("selection disabled")]
    Disabled,
    #[error("invalid key or index")]
    Key,
}
