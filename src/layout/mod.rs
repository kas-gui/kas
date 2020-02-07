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

use kas::geom::Size;

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
    fn new(vertical: bool, fixed: Option<u32>) -> Self {
        AxisInfo {
            vertical: vertical,
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

    /// Size of other axis, if fixed and `vertical == self.is_vertical()`.
    #[inline]
    pub fn fixed(&self, vertical: bool) -> Option<u32> {
        if vertical == self.vertical && self.has_fixed {
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

pub trait Directional: Copy + Sized + std::fmt::Debug {
    fn as_direction(self) -> Direction;

    #[inline]
    fn is_vertical(self) -> bool {
        self.as_direction() == Direction::Vertical
    }

    #[inline]
    fn is_horizontal(self) -> bool {
        self.as_direction() == Direction::Horizontal
    }
}

#[derive(Copy, Clone, Default, Debug)]
pub struct Horizontal;
impl Directional for Horizontal {
    #[inline]
    fn as_direction(self) -> Direction {
        Direction::Horizontal
    }
}

#[derive(Copy, Clone, Default, Debug)]
pub struct Vertical;
impl Directional for Vertical {
    #[inline]
    fn as_direction(self) -> Direction {
        Direction::Vertical
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Direction {
    Horizontal = 0,
    Vertical = 1,
}

impl Directional for Direction {
    #[inline]
    fn as_direction(self) -> Direction {
        self
    }
}
