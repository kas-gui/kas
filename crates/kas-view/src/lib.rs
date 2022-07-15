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
//! Shared data must implement [`SharedData`], optionally [`SharedDataMut`]
//! and (depending on dimension) optionally [`ListData`] or [`MatrixData`].
//!
//! For simpler cases it is not always necessary to implement your own shared
//! data type, for example `SharedRc<i32>` implements [`SharedData`] TODO and
//! `&'static [&'static str]` implements [`ListData`]. The [`SharedRc`] type
//! provides an `update` method and the [`UpdateId`] and version counter
//! required to synchronise views; `&[T]` does not (data is constant).
//!
//! # View widgets and drivers
//!
//! Standard widgets may be used to view data items, but to construct these a
//! [`Driver`] type is required: the driver constructs a (parameterized) widget
//! over the data, and may update the data on events from the widget.
//! Use [`driver::View`] or [`driver::NavView`] for simple data or see the
//! [`driver`] module for more.
//!
//! The user may implement a [`Driver`] or may use a standard one:
//!
//! -   [`driver::View`] constructs a default view widget over various data types
//! -   [`driver::NavView`] is a variant of the above, ensuring items support
//!     keyboard navigation (e.g. useful to allow selection of static items)
//! -   [`driver::CheckButton`] and [`driver::RadioButton`] support the `bool` type
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
//! -   supporting scrolling (see [`kas::Scrollable`])
//! -   supporting item selection
//! -   controlling scrolling and selection via otherwise unhandled events
//!
//! The following views are provided:
//!
//! -   [`SingleView`] creates a view over a [`SharedData`] object (no scrolling
//!     or selection support)
//! -   [`ListView`] creates a scrollable list view over a [`ListData`] object

#![cfg_attr(doc_cfg, feature(doc_cfg))]

#[allow(unused)]
use kas::event::UpdateId;
#[allow(unused)]
use kas::model::{ListData, MatrixData, SharedData, SharedDataMut, SharedRc};
use thiserror::Error;

mod list_view;
mod matrix_view;
mod single_view;

pub mod driver;

pub use driver::Driver;
pub use list_view::ListView;
pub use matrix_view::MatrixView;
pub use single_view::SingleView;

/// Used to notify selection and deselection of [`ListView`] and [`MatrixView`] children
#[derive(Clone, Debug)]
pub enum SelectionMsg<K> {
    /// Selection of item
    Select(K),
    /// Deselection of item
    ///
    /// Note: not emitted due to selection of another item in single-item selection mode.
    Deselect(K),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum PressPhase {
    None,
    Start(kas::geom::Coord),
    Pan,
}

/// Selection mode used by [`ListView`]
#[derive(Clone, Copy, Debug)]
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
