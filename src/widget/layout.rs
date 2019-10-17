// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Simple widget layout system

use std::fmt;

use crate::toolkit::TkWidget;
use crate::widget::{Core, Rect, Size};

/// Size preferences.
///
/// This type supports `Ord` such that for all values `x`,
/// `SizePref::Min <= x` and `x <= SizePref::Max`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SizePref {
    /// Minimal functional size
    Min,
    /// Small size
    Small,
    /// The default and preferred size
    Default,
    /// Larger size
    Large,
    /// Maximal useful size
    ///
    /// Widgets may be enlarged beyond this size
    Max,
}

impl SizePref {
    /// Increment the size, saturating
    pub fn increment(self) -> SizePref {
        use SizePref::*;
        match self {
            Min => Small,
            Small => Default,
            Default => Large,
            Large => Max,
            Max => Max,
        }
    }

    /// Decrement the size, saturating
    pub fn decrement(self) -> SizePref {
        use SizePref::*;
        match self {
            Min => Min,
            Small => Min,
            Default => Small,
            Large => Default,
            Max => Large,
        }
    }
}

/// Axes being resized
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Axes {
    /// Adjust both axes
    Both,
    /// Adjust horizontal axis only
    Horiz,
    /// Adjust vertical axis only
    Vert,
}

/// Widget size and layout.
pub trait Layout: Core + fmt::Debug {
    #[doc(hidden)]
    /// Get the size according to the given preference and cache the result.
    ///
    /// For simple widgets (without children), this method is usually just a
    /// wrapper around [`TkWidget::size`]. For parent widgets it gets
    /// significantly more complicated, and we recommend use of the
    /// [`kas_macros::make_widget`] macro.
    ///
    /// The last two returned sizes should be "cached" for use by
    /// [`Layout::set_rect`], but only where the axis is included by the `axes`
    /// argument. Sizes returned for other axes need not be correct.
    ///
    /// Widgets do not need to return distinct sizes for each `SizePref`.
    /// The `SizePref` type supports `Ord`, allowing effective use of ranges.
    /// This should be taken advantage of since size categories may be adjusted
    /// in the future.
    fn size_pref(&mut self, tk: &dyn TkWidget, pref: SizePref, axes: Axes) -> Size;

    #[doc(hidden)]
    /// Adjust to the given size.
    ///
    /// For many widgets this operation is trivial and the default
    /// implementation will suffice. For layout widgets (those with children),
    /// this operation is more complex.
    ///
    /// See notes on the [`Layout::size_pref`] method regarding caching of
    /// results. For each axis, the size specified by this method is guaranteed
    /// to be either equal to the result of the last `size_pref` query, or
    /// between the results of the last two queries.
    fn set_rect(&mut self, rect: Rect) {
        self.core_data_mut().rect = rect;
    }
}
