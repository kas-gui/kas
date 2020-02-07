// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout solver
//!
//! This is only of interest if building a custom widget with children.

mod grid_solver;
mod misc_solver;
mod row_solver;
mod size_rules;
mod sizer;
mod storage;

use crate::geom::Size;
use crate::{Direction, Directional};

pub use grid_solver::{GridChildInfo, GridSetter, GridSolver};
pub use misc_solver::SingleSetter;
pub use row_solver::{RowPositionSolver, RowSetter, RowSolver};
pub use size_rules::{Margins, SizeRules};
pub use sizer::{solve, RulesSetter, RulesSolver};
pub use storage::{
    DynGridStorage, DynRowStorage, FixedGridStorage, FixedRowStorage, GridStorage, RowStorage,
    RowTemp, Storage,
};

/// Information on which axis is being resized
///
/// Also conveys the size of the other axis, if fixed.
#[derive(Copy, Clone, Debug)]
pub struct AxisInfo {
    vertical: bool,
    has_fixed: bool,
    other_axis: u32,
}

impl AxisInfo {
    fn new(dir: Direction, fixed: Option<u32>) -> Self {
        AxisInfo {
            vertical: dir.is_vertical(),
            has_fixed: fixed.is_some(),
            other_axis: fixed.unwrap_or(0),
        }
    }

    /// True if the current axis is vertical
    #[inline]
    pub fn is_vertical(&self) -> bool {
        self.vertical
    }

    /// True if the current axis is horizontal
    #[inline]
    pub fn is_horizontal(self) -> bool {
        !self.vertical
    }

    /// Size of other axis, if fixed and `direction` matches this axis.
    #[inline]
    pub fn size_other_if_fixed(&self, dir: Direction) -> Option<u32> {
        if dir.is_vertical() == self.vertical && self.has_fixed {
            Some(self.other_axis)
        } else {
            None
        }
    }

    /// Extract horizontal or vertical component of a [`Size`]
    #[inline]
    pub fn extract_size(&self, size: Size) -> u32 {
        if !self.vertical {
            size.0
        } else {
            size.1
        }
    }
}
