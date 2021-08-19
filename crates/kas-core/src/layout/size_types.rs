// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Types used by size rules

use super::{AlignHints, AxisInfo, SizeRules};
use crate::cast::{Cast, CastFloat, Conv, ConvFloat};
use crate::dir::Directional;
use crate::geom::{Rect, Size, Vec2};

// for doc use
#[allow(unused)]
use crate::draw::SizeHandle;

/// Margin sizes
///
/// Used by the layout system for margins around child widgets. Margins may be
/// drawn in and handle events like any other widget area.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
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

    /// Extract one component, based on a direction
    ///
    /// This merely extracts the horizontal or vertical component.
    /// It never negates it, even if the axis is reversed.
    #[inline]
    pub fn extract<D: Directional>(self, dir: D) -> (u16, u16) {
        match dir.is_vertical() {
            false => self.horiz,
            true => self.vert,
        }
    }
}

impl From<Size> for Margins {
    fn from(size: Size) -> Self {
        Margins::hv_splat(size.0.cast(), size.1.cast())
    }
}

/// Margins (selectable)
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MarginSelector {
    /// Use the theme's default around-widget margins
    Outer,
    /// Use the theme's default within-widget margins
    Inner,
    /// Use theme's default around-text margins
    Text,
    /// Use fixed margins
    Fixed(Margins),
    /// Use scaled margins (single value)
    ScaledSplat(f32),
}

impl Default for MarginSelector {
    fn default() -> Self {
        MarginSelector::Outer
    }
}

impl MarginSelector {
    /// Convert to fixed [`Margins`]
    pub fn select(&self, sh: &dyn SizeHandle) -> Margins {
        match self {
            MarginSelector::Outer => sh.outer_margins(),
            MarginSelector::Inner => Margins::from(sh.inner_margin()),
            MarginSelector::Text => sh.text_margins(),
            MarginSelector::Fixed(fixed) => *fixed,
            MarginSelector::ScaledSplat(m) => {
                Margins::splat(u16::conv_nearest(m * sh.scale_factor()))
            }
        }
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

/// Scaling of image according to scale factor
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SpriteScaling {
    /// Do not scale with scale factor
    Original,
    /// Use the nearest integer of scale factor (e.g. 1, 2, 3)
    Integer,
    /// Use raw scale factor
    Real,
}

impl Default for SpriteScaling {
    fn default() -> Self {
        SpriteScaling::Integer
    }
}

/// Scaling of image sprite within allocation
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum AspectScaling {
    /// Align sprite within available space without further scaling
    None,
    /// Scale sprite to available space with fixed aspect ratio
    Fixed,
    /// Scale sprite freely
    Free,
    // TODO: we could add repeat (tile) and mirrored repeat modes here
}

impl Default for AspectScaling {
    fn default() -> Self {
        AspectScaling::Fixed
    }
}

/// Widget component for displaying a sprite
#[derive(Clone, Debug, PartialEq)]
pub struct SpriteDisplay {
    /// Margins
    pub margins: MarginSelector,
    /// The native size of the sprite
    pub size: Size,
    /// Sprite scaling according to scale factor
    pub scaling: SpriteScaling,
    /// Sprite scaling within allocation, after impact of scale factor
    ///
    /// Note: this only has an impact if `stretch > Stretch::None`.
    pub aspect: AspectScaling,
    /// Widget stretchiness
    pub stretch: Stretch,
}

impl Default for SpriteDisplay {
    fn default() -> Self {
        SpriteDisplay {
            margins: MarginSelector::Outer,
            size: Size::ZERO,
            scaling: SpriteScaling::Integer,
            aspect: AspectScaling::Fixed,
            stretch: Stretch::None,
        }
    }
}

impl SpriteDisplay {
    /// Generates `size_rules` based on size
    ///
    /// Set [`Self::size`] before calling this.
    pub fn size_rules(&mut self, sh: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let margins = self.margins.select(sh).extract(axis);
        let size = self.size.extract(axis);
        let size = match self.scaling {
            SpriteScaling::Original => size,
            SpriteScaling::Integer => i32::conv_nearest(sh.scale_factor()) * size,
            SpriteScaling::Real => (sh.scale_factor() * f32::conv(size)).cast_nearest(),
        };
        SizeRules::new(size, size, margins, self.stretch)
    }

    /// Aligns `rect` according to stretch policy
    ///
    /// Assign the result to `self.core_data_mut().rect`.
    pub fn align_rect(&mut self, rect: Rect, align: AlignHints) -> Rect {
        let ideal = match self.aspect {
            AspectScaling::None => self.size,
            AspectScaling::Fixed => {
                let size = Vec2::from(self.size);
                let ratio = Vec2::from(rect.size) / size;
                // Use smaller ratio, which must be finite
                if ratio.0 < ratio.1 {
                    Size(rect.size.0, i32::conv_nearest(ratio.0 * size.1))
                } else if ratio.1 < ratio.0 {
                    Size(i32::conv_nearest(ratio.1 * size.0), rect.size.1)
                } else {
                    // Non-finite ratio implies size is zero on at least one axis
                    rect.size
                }
            }
            AspectScaling::Free => rect.size,
        };
        align
            .complete(Default::default(), Default::default())
            .aligned_rect(ideal, rect)
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
    pub fn new_sym(size: i32, inner_margin: i32, outer_margin: u16) -> Self {
        Self::new(size, size, inner_margin, (outer_margin, outer_margin))
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
    pub fn surround_with_margin(self, content: SizeRules) -> (SizeRules, i32, i32) {
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

    /// Variant: frame surrounds content
    ///
    /// The content's margin is reduced by the size of the frame, with any
    /// residual margin applying outside the frame (using the max of the
    /// frame's own margin and the residual). In other respects,
    /// this is the same as [`FrameRules::surround_with_margin`].
    pub fn surround_as_margin(self, content: SizeRules) -> (SizeRules, i32, i32) {
        let (m0, m1) = content.margins();
        let offset = self.offset + self.inner_margin;
        let m0 = u16::conv((i32::conv(m0) - offset).max(0));
        let size = self.size + 2 * self.inner_margin;
        let m1 = u16::conv((i32::conv(m1) + offset - size).max(0));
        let margins = (self.m.0.max(m0), self.m.1.max(m1));

        let rules = SizeRules::new(
            content.min_size() + size,
            content.ideal_size() + size,
            margins,
            content.stretch(),
        );
        (rules, offset, size)
    }
}
