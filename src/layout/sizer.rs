// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout solver

use super::SizeRules;
use crate::geom::Size;
use crate::{Layout, TkWindow};

/// Parameter type passed to [`Layout::size_rules`]
#[derive(Copy, Clone, Debug)]
pub struct AxisInfo {
    is_vert: bool,
    fixed: bool,
    other: u32,
}

impl AxisInfo {
    /// Construct an instance
    #[inline]
    pub fn new(vert: bool, fixed: Option<u32>) -> Self {
        AxisInfo {
            is_vert: vert,
            fixed: fixed.is_some(),
            other: fixed.unwrap_or(0),
        }
    }

    /// Adjust horizontal axis
    #[inline]
    pub fn horiz(&self) -> bool {
        !self.is_vert
    }

    /// Adjust vertical axis
    #[inline]
    pub fn vert(&self) -> bool {
        self.is_vert
    }

    /// Has a fixed dimension for one axis
    #[inline]
    pub fn has_fixed(&self) -> bool {
        self.fixed
    }

    /// Size of other axis, if fixed and (`vert == self.vert()`).
    #[inline]
    pub fn fixed(&self, vert: bool) -> Option<u32> {
        if vert == self.is_vert && self.fixed {
            Some(self.other)
        } else {
            None
        }
    }

    /// Size of other axis, if applicable
    #[inline]
    pub fn other(&self) -> u32 {
        self.other
    }

    /// Set size of fixed axis, if applicable
    #[inline]
    pub fn set_size(&mut self, size: u32) {
        self.other = size;
    }
}

/// A [`SizeRules`] solver for layouts
pub trait Sizer {
    /// Type required by [`Sizer::for_child`] (see implementation documentation)
    type ChildInfo;
    /// Called before [`Sizer::for_child`]
    fn prepare(&mut self);
    /// Called once for each child. For most layouts the order is important.
    fn for_child<C: Layout>(&mut self, child_info: Self::ChildInfo, child: &mut C);
    /// Called at the end to output [`SizeRules`].
    ///
    /// Note that this does not include margins!
    fn finish<Iter: Iterator<Item = (usize, usize, usize)>>(self, span_iter: Iter) -> SizeRules;
}

/// Solve `widget` for `SizeRules` on both axes, horizontal first.
pub fn solve<L: Layout>(widget: &mut L, tk: &mut dyn TkWindow, size: Size) {
    // We call size_rules not because we want the result, but because our
    // spec requires that we do so before calling set_rect.
    let _w = widget.size_rules(tk, AxisInfo::new(false, None));
    let _h = widget.size_rules(tk, AxisInfo::new(true, Some(size.0)));
}
