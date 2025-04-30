// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! # Views
//!
//! Views allow virtual scrolling and query views over a data set, supporting
//! both sync and async access.
//!
//! Each visible data `Item` is assigned a **view widget**, with dynamic
//! re-assignment as the view changes.
//!
//! ## Data sets and clerks
//!
//! The full data set might be available in local memory, on disk, or on a
//! remote server.
//!
//! A [`DataClerk`] manages all interactions between the view and the data as
//! well as providing a local cache of (at least) the currently visible data.
//!
//! ## View controller
//!
//! This crate provides the following **view controllers**:
//!
//! -   [`ListView`] constructs a row or column view over items indexed by type `usize`
//! -   [`MatrixView`] constructs a table over items indexed by type `(u32, u32)`
//!
//! ## Driver
//!
//! A view controller uses a **driver** to construct and re-assign view widgets.
//! Simple types (strings and numbers) may use a pre-defined [`driver`],
//! otherwise a custom implementation of [`Driver`] is required.

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

mod data_traits;
pub use data_traits::*;

pub mod filter;

pub mod driver;
pub use driver::Driver;

mod list_view;
pub use list_view::ListView;

mod matrix_view;
pub use matrix_view::{MatrixIndex, MatrixView};

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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
