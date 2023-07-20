// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shared data models
//!
//! Models of 0-, 1- and 2-dimensional data. These are used by "view widgets",
//! enabling synchronized views over shared data.
//!
//! All shared data must implement [`SharedData`].
//! For 0-dimensional data this alone is enough; `()` is used as a key.
//! For 1- or 2-dimensional data implement [`ListData`] or [`MatrixData`].
//!
//! Some implementations are provided, e.g. [`ListData`] is implemented for
//! `[T]`, `Vec<T>`.
mod data_impls;
mod data_traits;
pub mod filter;

pub use data_traits::*;
