// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout utilities
//!
//! For documentation of layout resolution, see the [`Layout`] trait.
//!
//! Size units are physical (real) pixels. This applies to most of KAS.
//!
//! ## Data types
//!
//! [`SizeRules`] is the "heart" of widget layout, used to specify a widget's
//! size requirements. It provides various methods to compute derived rules
//! and [`SizeRules::solve_seq`], the "muscle" of the layout engine.
//!
//! [`AxisInfo`], [`Margins`] and [`Stretch`] are auxilliary data types.
//!
//! ## Solvers
//!
//! The [`RulesSolver`] and [`RulesSetter`] traits define interfaces for
//! layout engines:
//!
//! -   [`SingleSolver`] and [`SingleSetter`] are trivial implementations for
//!     single-child parents
//! -   [`RowSolver`] and [`RowSetter`] set out a row or column of children.
//!     These are parametrised over `S: RowStorage` allowing both efficient
//!     operation on a small fixed number of children with [`FixedRowStorage`]
//!     and operation on a over a `Vec` with [`DynRowStorage`].
//! -   [`GridSolver`] and [`GridSetter`] set out children assigned to grid
//!     cells with optional cell-spans. This is the most powerful and flexible
//!     layout engine.
//!
//! [`RowPositionSolver`] may be used with widgets set out by [`RowSetter`]
//! to quickly locate children from a `coord` or `rect`.
//!
//! [`Layout`]: crate::Layout

mod align;
mod grid_solver;
mod row_solver;
mod single_solver;
mod size_rules;
mod size_types;
mod sizer;
mod storage;
mod visitor;

use crate::dir::{Direction, Directional, Directions};

pub use align::{Align, AlignHints, AlignPair};
pub use grid_solver::{DefaultWithLen, GridCellInfo, GridDimensions, GridSetter, GridSolver};
pub use row_solver::{RowPositionSolver, RowSetter, RowSolver};
pub use single_solver::{SingleSetter, SingleSolver};
pub use size_rules::SizeRules;
pub use size_types::*;
pub use sizer::{solve_size_rules, RulesSetter, RulesSolver, SolveCache};
pub use storage::*;
pub use visitor::{FrameStorage, PackStorage};

#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
pub use visitor::{LayoutList, Visitor};

/// Information on which axis is being resized
///
/// Also conveys the size of the other axis, if fixed.
#[derive(Copy, Clone, Debug)]
pub struct AxisInfo {
    vertical: bool,
    has_fixed: bool,
    other_axis: i32,
}

impl AxisInfo {
    /// Construct with direction and an optional value for the other axis
    ///
    /// This method is *usually* not required by user code.
    #[inline]
    pub fn new(vertical: bool, fixed: Option<i32>) -> Self {
        AxisInfo {
            vertical,
            has_fixed: fixed.is_some(),
            other_axis: fixed.unwrap_or(0),
        }
    }

    /// True if the current axis is vertical
    #[inline]
    pub fn is_vertical(self) -> bool {
        self.vertical
    }

    /// True if the current axis is horizontal
    #[inline]
    pub fn is_horizontal(self) -> bool {
        !self.vertical
    }

    /// Size of other axis, if fixed
    #[inline]
    pub fn other(&self) -> Option<i32> {
        if self.has_fixed {
            Some(self.other_axis)
        } else {
            None
        }
    }

    /// Subtract `x` from size of other axis (if applicable)
    #[inline]
    pub fn sub_other(&mut self, x: i32) {
        self.other_axis -= x;
    }
}

impl Directional for AxisInfo {
    type Flipped = Self;
    type Reversed = Self;

    fn flipped(mut self) -> Self::Flipped {
        self.vertical = !self.vertical;
        self.has_fixed = false;
        self
    }

    #[inline]
    fn reversed(self) -> Self::Reversed {
        self
    }

    #[inline]
    fn as_direction(self) -> Direction {
        match self.vertical {
            false => Direction::Right,
            true => Direction::Down,
        }
    }
}

impl From<AxisInfo> for Directions {
    fn from(axis: AxisInfo) -> Directions {
        match axis.vertical {
            false => Directions::LEFT | Directions::RIGHT,
            true => Directions::UP | Directions::DOWN,
        }
    }
}

#[cfg(test)]
#[test]
fn size() {
    assert_eq!(std::mem::size_of::<AxisInfo>(), 8);
}
