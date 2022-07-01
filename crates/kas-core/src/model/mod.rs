// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shared data models
//!
//! Models of 0-, 1- and 2-dimensional data. These are used by "view widgets",
//! enabling synchronized views over shared data.

mod data_impls;
mod data_traits;
pub mod filter;
mod shared_rc;

pub use data_traits::{
    ListData, ListDataMut, MatrixData, MatrixDataMut, SingleData, SingleDataMut,
};
pub use shared_rc::SharedRc;
