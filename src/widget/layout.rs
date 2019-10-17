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
    MaxUseful,
    /// Absolute maximum desired
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
            Large => MaxUseful,
            MaxUseful => Max,
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
            MaxUseful => Large,
            Max => MaxUseful,
        }
    }
}

/// Widget size and layout.
pub trait Layout: Core + fmt::Debug {
    #[doc(hidden)]
    /// Get the size according to the given preference and cache the result.
    ///
    /// Usually, this method is a wrapper around [`TkWidget::size`].
    ///
    /// Widgets do not need to return distinct sizes for each `SizePref`.
    /// The `SizePref` type supports `Ord`, allowing effective use of ranges.
    /// This should be taken advantage of since size categories may be adjusted
    /// in the future.
    fn size_pref(&mut self, tk: &dyn TkWidget, pref: SizePref) -> Size;

    #[doc(hidden)]
    /// Adjust to the given size.
    ///
    /// It is suggested that widgets (at least, those with children) cache the
    /// results of the last two calls to [`Layout::size`]; the size set by this
    /// method should lie between the last two [`Layout::size`] results.
    fn set_rect(&mut self, rect: Rect) {
        self.core_data_mut().rect = rect;
    }
}
