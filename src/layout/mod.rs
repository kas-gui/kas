// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout solver
//!
//! This is only of interest if building a custom widget with children.

mod grid_solver;
mod row_solver;
mod size_rules;
mod sizer;

pub use grid_solver::{FixedGridSolver, GridChildInfo};
pub use row_solver::FixedRowSolver;
pub use size_rules::SizeRules;
pub use sizer::{solve, AxisInfo, Sizer};
