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
use crate::event::ConfigMgr;
use crate::geom::{Coord, Rect};
use crate::theme::{DrawMgr, SizeMgr};
use crate::WidgetId;

#[allow(unused)] use crate::Layout;

pub use align::{Align, AlignHints, AlignPair};
pub use grid_solver::{DefaultWithLen, GridChildInfo, GridDimensions, GridSetter, GridSolver};
pub use row_solver::{RowPositionSolver, RowSetter, RowSolver};
pub use single_solver::{SingleSetter, SingleSolver};
pub use size_rules::SizeRules;
pub use size_types::*;
pub use sizer::{solve_size_rules, RulesSetter, RulesSolver, SolveCache};
pub use storage::*;
pub use visitor::{FrameStorage, PackStorage, Visitor};

/// Information on which axis is being resized
///
/// Also conveys the size of the other axis, if fixed.
#[derive(Copy, Clone, Debug)]
pub struct AxisInfo {
    vertical: bool,
    has_fixed: bool,
    other_axis: i32,
    align: Option<Align>,
}

impl AxisInfo {
    /// Construct with direction and an optional value for the other axis
    ///
    /// This method is *usually* not required by user code.
    #[inline]
    pub fn new(vertical: bool, fixed: Option<i32>, align: Option<Align>) -> Self {
        AxisInfo {
            vertical,
            has_fixed: fixed.is_some(),
            other_axis: fixed.unwrap_or(0),
            align,
        }
    }

    /// Construct a copy using the given alignment hints
    #[inline]
    pub fn with_align_hints(mut self, hints: AlignHints) -> Self {
        self.align = hints.extract(self).or(self.align);
        self
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

    /// Get align parameter
    #[inline]
    pub fn align(self) -> Option<Align> {
        self.align
    }

    /// Set align parameter
    #[inline]
    pub fn set_align(&mut self, align: Option<Align>) {
        self.align = align;
    }

    /// Set default alignment
    ///
    /// If the optional alignment parameter is `None`, replace with `align`.
    #[inline]
    pub fn set_default_align(&mut self, align: Align) {
        if self.align.is_none() {
            self.align = Some(align);
        }
    }

    /// Set default alignment
    ///
    /// If the optional alignment parameter is `None`, replace with either
    /// `horiz` or `vert` depending on this axis' orientation.
    #[inline]
    pub fn set_default_align_hv(&mut self, horiz: Align, vert: Align) {
        if self.align.is_none() {
            if self.is_horizontal() {
                self.align = Some(horiz);
            } else {
                self.align = Some(vert);
            }
        }
    }

    /// Get align parameter, defaulting to [`Align::Default`]
    #[inline]
    pub fn align_or_default(self) -> Align {
        self.align.unwrap_or(Align::Default)
    }

    /// Get align parameter, defaulting to [`Align::Center`]
    #[inline]
    pub fn align_or_center(self) -> Align {
        self.align.unwrap_or(Align::Center)
    }

    /// Get align parameter, defaulting to [`Align::Stretch`]
    #[inline]
    pub fn align_or_stretch(self) -> Align {
        self.align.unwrap_or(Align::Stretch)
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

    /// Size of other axis, if fixed and `vertical` matches this axis.
    #[inline]
    pub fn size_other_if_fixed(&self, vertical: bool) -> Option<i32> {
        if vertical == self.vertical && self.has_fixed {
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

/// Implementation generated by use of `layout = ..` property of `#[widget]`
///
/// This trait need not be implemented by the user, however it may be useful to
/// adjust the result of an automatic implementation, for example:
/// ```
/// # extern crate kas_core as kas;
/// use kas::prelude::*;
///
/// impl_scope! {
///     #[widget {
///         Data = ();
///         layout = "Example";
///     }]
///     struct Example {
///         core: widget_core!(),
///     }
///     impl Layout for Self {
///         fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
///             let mut rules = kas::layout::AutoLayout::size_rules(self, size_mgr, axis);
///             rules.set_stretch(Stretch::High);
///             rules
///         }
///     }
/// }
/// ```
///
/// It is not recommended to import this trait since method names conflict with [`Layout`].
pub trait AutoLayout {
    /// Get size rules for the given axis
    ///
    /// This functions identically to [`Layout::size_rules`].
    fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules;

    /// Set size and position
    ///
    /// This functions identically to [`Layout::set_rect`].
    fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect);

    /// Translate a coordinate to a [`WidgetId`]
    ///
    /// This functions identically to [`Layout::find_id`].
    fn find_id(&mut self, coord: Coord) -> Option<WidgetId>;

    /// Draw a widget and its children
    ///
    /// This functions identically to [`Layout::draw`].
    fn draw(&mut self, draw: DrawMgr);
}

#[cfg(test)]
#[test]
fn size() {
    assert_eq!(std::mem::size_of::<AxisInfo>(), 8);
}
