// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shared data models
//!
//! Models of 0-, 1- and 2-dimensional data. These are used by "view widgets",
//! enabling synchronized views over shared data.
//!
//! All shared data must implement [`SharedData`] (optionally also
//! [`SharedDataMut`] allowing direct access via mutable reference).
//! For 0-dimensional data this alone is enough; `()` is used as a key.
//! For 1- or 2-dimensional data implement [`ListData`] or [`MatrixData`].
//!
//! Some implementations are provided, e.g. [`ListData`] is implemented for
//! `[T]`, `Vec<T>`. The [`SharedRc`] type is a wrapper enabling sharing of
//! 0-dimensional data via the `Rc<RefCell<T>>` pattern (with additions for
//! synchronization).
//!
//! # Temporary design
//!
//! It is intended that once Rust has stable (lifetime) Generic Associated Types
//! the traits provided here be revised as follows:
//!
//! -   Add `SharedData::borrow`, functioning as [`SharedRc::borrow`]
//! -   Add `SharedDataMut::borrow_mut`
//! -   Revise [`SharedData::update`]: probably return a type supporting
//!     `DerefMut<Target = SharedData::Item>` while still updating the reference
//!     counter
//! -   Revise [`ListData::iter_vec`] etc: return an iterator instead of a [`Vec`]
mod data_impls;
mod data_traits;
pub mod filter;
mod shared_arc;
mod shared_rc;

pub use data_traits::{DataKey, ListData, MatrixData, SharedData, SharedDataMut, SingleData};
pub use shared_arc::SharedArc;
pub use shared_rc::{SharedRc, SharedRcRef};
