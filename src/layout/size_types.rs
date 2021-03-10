// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Types used by size rules

use super::SizeRules;
use crate::geom::Size;

// for doc use
#[allow(unused)]
use kas::draw::SizeHandle;

/// Margin sizes
///
/// Used by the layout system for margins around child widgets. Margins may be
/// drawn in and handle events like any other widget area.
#[derive(Copy, Clone, Debug, Default)]
pub struct Margins {
    /// Size of horizontal margins
    pub horiz: (u16, u16),
    /// Size of vertical margins
    pub vert: (u16, u16),
}

impl Margins {
    /// Zero-sized margins
    pub const ZERO: Margins = Margins::splat(0);

    /// Margins with equal size on each edge.
    #[inline]
    pub const fn splat(size: u16) -> Self {
        Margins::hv_splat(size, size)
    }

    /// Margins via horizontal and vertical sizes
    #[inline]
    pub const fn hv(horiz: (u16, u16), vert: (u16, u16)) -> Self {
        Margins { horiz, vert }
    }

    /// Margins via horizontal and vertical sizes
    #[inline]
    pub const fn hv_splat(h: u16, v: u16) -> Self {
        Margins {
            horiz: (h, h),
            vert: (v, v),
        }
    }

    /// Sum of horizontal margins
    #[inline]
    pub fn sum_horiz(&self) -> i32 {
        i32::from(self.horiz.0) + i32::from(self.horiz.1)
    }

    /// Sum of vertical margins
    #[inline]
    pub fn sum_vert(&self) -> i32 {
        i32::from(self.vert.0) + i32::from(self.vert.1)
    }

    /// Pad a size with margins
    pub fn pad(self, size: Size) -> Size {
        Size::new(size.0 + self.sum_horiz(), size.1 + self.sum_vert())
    }
}

/// Priority for stretching widgets beyond ideal size
///
/// Space is allocated based on priority, with extra space (beyond the minimum)
/// shared between widgets in the highest priority class.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum Stretch {
    /// No expectations beyond the minimum
    ///
    /// Note: this does not prevent stretching (specifically, it can happen with
    /// other widgets in the same row/column wishing more size).
    None,
    /// Fill unwanted space
    Filler,
    /// Extra space is considered of low utility (but higher than `Filler`)
    Low,
    /// Extra space is considered of high utility
    High,
    /// Greedily consume as much space as possible
    Maximize,
}

impl Default for Stretch {
    fn default() -> Self {
        Stretch::None
    }
}

/// Frame size rules
///
/// This is a special variant of [`SizeRules`] for frames. It is assumed that
/// frames are not stretchy (i.e. that min-size equals ideal-size); additionally
/// frame rules have a content offset and a minimum internal margin size.
#[derive(Clone, Copy, Debug)]
pub struct FrameRules {
    offset: i32,
    size: i32,
    inner_margin: i32,
    // (pre, post) margins
    m: (u16, u16),
}

impl FrameRules {
    /// Construct
    ///
    /// -   `first`: size of left or top edge
    /// -   `second`: size of right or bottom edge
    /// -   `inner_margin`: minimum size of inner margins
    /// -   `outer_margins`: size of (left, right) or (top, bottom) outer margins
    #[inline]
    pub fn new(first: i32, second: i32, inner_margin: i32, outer_margins: (u16, u16)) -> Self {
        FrameRules {
            offset: first,
            size: first + second,
            inner_margin,
            m: outer_margins,
        }
    }

    /// Construct (symmetric on axis)
    #[inline]
    pub fn new_sym(size: i32, inner_margin: i32, outer_margins: (u16, u16)) -> Self {
        Self::new(size, size, inner_margin, outer_margins)
    }

    /// Generate rules for content surrounded by this frame
    ///
    /// It is assumed that the content's margins apply inside this frame, and
    /// that the margin is at least as large as self's `inner_margin`.
    ///
    /// Returns the tuple `(rules, offset, size)`:
    ///
    /// -   the generated `rules`
    /// -   the content `offset` within the allocated rect
    /// -   the size consumed by the frame and inner margins (thus the content's
    ///     size will be that allocated for this object minus this `size` value)
    pub fn surround(self, content: SizeRules) -> (SizeRules, i32, i32) {
        let (m0, m1) = content.margins_i32();
        let m0 = m0.max(self.inner_margin);
        let m1 = m1.max(self.inner_margin);
        let offset = self.offset + m0;
        let size = self.size + m0 + m1;

        let rules = SizeRules::new(
            content.min_size() + size,
            content.ideal_size() + size,
            self.m,
            content.stretch(),
        );
        (rules, offset, size)
    }
}
