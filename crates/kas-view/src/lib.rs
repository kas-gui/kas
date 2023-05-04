// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! View widgets and shared data
//!
//! View widgets allow data-oriented design. This is vaguely similar to the
//! Model-View-Controller pattern or Elm's Model-View-Update design, but with
//! no direct link between Model and Controller:
//!
//! 1.  [`kas::model`] traits describe data **models**:
//!     [`SingleData`](kas::model::SingleData),
//!     [`ListData`](kas::model::ListData),
//!     [`MatrixData`](kas::model::MatrixData)
//! 2.  [**Drivers**](`driver`) describe how to build a widget view over data
//!     and (optionally) how to handle **messages** from view widgets
//! 3.  **Controllers** are special widgets which manage views over data
//!
//! Three controllers are provided by this crate:
//!
//! -   [`ListView`] constructs a row or column of views over indexable data
//! -   [`MatrixView`] constructs a table/sheet of views over two-dimensional
//!     indexable data
//!
//! Both [`ListView`] and [`MatrixView`] support virtual scrolling: the number
//! of view widget instances is limited (approximately) to the number required
//! to cover the visible area, and these are re-used to enable fast scrolling
//! through large data sets.

#![cfg_attr(doc_cfg, feature(doc_cfg))]

use thiserror::Error;

// mod list_view;
// mod matrix_view;

// pub mod driver;

// pub use driver::Driver;
// pub use list_view::ListView;
// pub use matrix_view::MatrixView;

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

/// Selection mode used by [`ListView`]
#[derive(Clone, Copy, Debug, Default)]
pub enum SelectionMode {
    /// Disable selection
    #[default]
    None,
    /// Support single-item selection. Selecting another item automatically
    /// clears the prior selection (without sending [`SelectionMsg::Deselect`]).
    Single,
    /// Support multi-item selection.
    Multiple,
}

/// Selection errors
#[derive(Error, Debug)]
pub enum SelectionError {
    #[error("selection disabled")]
    Disabled,
    #[error("invalid key or index")]
    Key,
}
