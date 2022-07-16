// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! View widgets and shared data
//!
//! So called "view widgets" allow separation of data and view. This system has
//! three parts:
//!
//! 1.  **Models** are defined by [`kas::model`], principally
//!     [`SharedData`](kas::model::SharedData).
//!     Separate models are available for [`ListData`](kas::model::ListData)
//!     and [`MatrixData`](kas::model::MatrixData).
//! 2.  **Views** are widgets constructed over shared data by a controller.
//!     The **view controller** is a special widget responsible for constructing
//!     and managing view widgets over data.
//!
//!     Three controllers are available:
//!     [`SingleView`], [`ListView`] and [`MatrixView`].
//!
//!     In the case of [`ListView`] and [`MatrixView`], the controller provides
//!     additional features: enabling scrolling of content, "paging" (loading
//!     only visible content) and (optionally) allowing selection of items.
//! 3.  **Drivers** are the "glue" enabling a view controller to build view
//!     widget(s) tailored to a specific data type as well as (optionally)
//!     updating this data in response to widget events.
//!
//!     If the driver is not explicitly provided, [`driver::View`] is used,
//!     which provides a read-only view over several data types.
//!     Other options are available in the [`driver`] module, or [`Driver`] may
//!     be implemented directly.

#![cfg_attr(doc_cfg, feature(doc_cfg))]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::len_zero)]

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
    /// Disable selection
    None,
    /// Support single-item selection. Selecting another item automatically
    /// clears the prior selection (without sending [`SelectionMsg::Deselect`]).
    Single,
    /// Support multi-item selection.
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
