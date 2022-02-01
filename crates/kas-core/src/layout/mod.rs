// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout solver
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
//! [`MarginSelector`] is a utility type facilitating user-selection of margins.
//!
//! ## Layout engines
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

use crate::dir::{Direction, Directional};
use crate::draw::DrawShared;
use crate::event::EventState;
use crate::theme::{SizeHandle, SizeMgr};
use crate::TkAction;
use std::ops::{Deref, DerefMut};

pub use align::{Align, AlignHints, CompleteAlignment};
pub use grid_solver::{DefaultWithLen, GridChildInfo, GridDimensions, GridSetter, GridSolver};
pub use row_solver::{RowPositionSolver, RowSetter, RowSolver};
pub use single_solver::{SingleSetter, SingleSolver};
pub use size_rules::SizeRules;
pub use size_types::{
    AspectScaling, FrameRules, MarginSelector, Margins, SpriteDisplay, SpriteScaling, Stretch,
};
pub use sizer::{solve_size_rules, RulesSetter, RulesSolver, SolveCache};
pub use storage::{
    DynGridStorage, DynRowStorage, FixedGridStorage, FixedRowStorage, GridStorage, RowStorage,
    RowTemp, Storage,
};
pub use visitor::{FrameStorage, Layout, StorageChain, TextStorage};

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
    pub fn is_vertical(&self) -> bool {
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

    /// Size of other axis, if fixed and `vertical` matches this axis.
    #[inline]
    pub fn size_other_if_fixed(&self, vertical: bool) -> Option<i32> {
        if vertical == self.vertical && self.has_fixed {
            Some(self.other_axis)
        } else {
            None
        }
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

/// Manager available to [`Layout::set_rect`] and [`crate::WidgetConfig::configure`]
///
/// This type is functionally a superset of [`SizeMgr`] and subset of
/// [`crate::theme::DrawMgr`], with support for the appropriate conversions.
///
/// `SetRectMgr` supports [`Deref`] and [`DerefMut`] with target [`EventState`].
#[must_use]
pub struct SetRectMgr<'a> {
    sh: &'a dyn SizeHandle,
    ds: &'a mut dyn DrawShared,
    ev: &'a mut EventState,
}

impl<'a> SetRectMgr<'a> {
    /// Construct
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub fn new(sh: &'a dyn SizeHandle, ds: &'a mut dyn DrawShared, ev: &'a mut EventState) -> Self {
        SetRectMgr { sh, ds, ev }
    }

    /// Access a [`SizeMgr`]
    pub fn size_mgr(&self) -> SizeMgr<'a> {
        SizeMgr::new(self.sh)
    }

    /// Access [`DrawShared`]
    pub fn draw_shared(&mut self) -> &mut dyn DrawShared {
        self.ds
    }

    /// Access [`EventState`]
    pub fn ev_state(&mut self) -> &mut EventState {
        self.ev
    }

    /// Access the screen's scale factor
    #[inline]
    pub fn scale_factor(&self) -> f32 {
        self.sh.scale_factor()
    }
}

impl<'a> std::ops::BitOrAssign<TkAction> for SetRectMgr<'a> {
    #[inline]
    fn bitor_assign(&mut self, action: TkAction) {
        self.ev.send_action(action);
    }
}

impl<'a> Deref for SetRectMgr<'a> {
    type Target = EventState;
    fn deref(&self) -> &EventState {
        self.ev
    }
}
impl<'a> DerefMut for SetRectMgr<'a> {
    fn deref_mut(&mut self) -> &mut EventState {
        self.ev
    }
}
