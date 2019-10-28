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
    /// Adjust horizontal axis only.
    /// If param is true, other axis is given fixed dimension.
    Horiz(bool),
    /// Adjust vertical axis only.
    /// If param is true, other axis is given fixed dimension.
    Vert(bool),
}

impl Axes {
    /// Adjust horizontal axis
    pub fn horiz(self) -> bool {
        if let Axes::Vert(_) = self {
            false
        } else {
            true
        }
    }

    /// Adjust vertical axis
    pub fn vert(self) -> bool {
        if let Axes::Horiz(_) = self {
            false
        } else {
            true
        }
    }
}

/// Widget size and layout.
pub trait Layout: Core + fmt::Debug {
    /// Get the size according to the given preference and cache the result.
    ///
    /// For simple widgets (without children), this method is usually just a
    /// wrapper around [`TkWidget::size_pref`]. For widgets with children this
    /// method is much more complex, and it is strongly recommended to rely on
    /// the [`kas_macros`] macros for implementations.
    // This function should calculate a size recommendation for one or both axes
    // (in compliance with the `axes` parameter), according to the given `pref`,
    // and optionally with the other axis fixed (see `Axes` enum).
    // The resulting size should then be cached locally at the given `index`
    // (0 or 1) but only for the enabled `axes`.
    fn size_pref(&mut self, tk: &mut dyn TkWidget, pref: SizePref, axes: Axes, index: bool)
        -> Size;

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
    fn set_rect(&mut self, rect: Rect, axes: Axes) {
        match axes {
            Axes::Both => {
                self.core_data_mut().rect = rect;
            }
            Axes::Horiz(_) => {
                self.core_data_mut().rect.size.0 = rect.size.0;
                self.core_data_mut().rect.pos.0 = rect.pos.0;
            }
            Axes::Vert(_) => {
                self.core_data_mut().rect.size.1 = rect.size.1;
                self.core_data_mut().rect.pos.1 = rect.pos.1;
            }
        }
    }
}
