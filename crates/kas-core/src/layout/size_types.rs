// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Types used by size rules

use super::{Align, AlignHints, AxisInfo, SizeRules};
use crate::cast::traits::*;
use crate::dir::Directional;
use crate::geom::{Rect, Size, Vec2};
use kas_macros::{impl_default, impl_scope};

// for doc use
#[allow(unused)]
use crate::theme::SizeMgr;

/// Logical (pre-scaling) pixel size
///
/// A measure of size in "logical pixels". May be used to define scalable
/// layouts.
#[derive(Copy, Clone, PartialEq, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LogicalSize(pub f32, pub f32);

impl LogicalSize {
    /// Convert to physical pixels
    ///
    /// Values are multiplied by the window's scale factor and cast to nearest.
    pub fn to_physical(self, scale_factor: f32) -> Size {
        let w = i32::conv_nearest(self.0 * scale_factor);
        let h = i32::conv_nearest(self.1 * scale_factor);
        Size(w, h)
    }

    /// Convert to [`SizeRules`], fixed size
    pub fn to_rules(self, dir: impl Directional, scale_factor: f32) -> SizeRules {
        SizeRules::fixed(self.extract_scaled(dir, scale_factor), (0, 0))
    }

    /// Convert to [`SizeRules`]
    ///
    /// Ideal size is `component * ideal_factor * scale_factor`.
    pub fn to_rules_with_factor(
        self,
        dir: impl Directional,
        scale_factor: f32,
        ideal_factor: f32,
    ) -> SizeRules {
        let min = self.extract_scaled(dir, scale_factor);
        let ideal = self.extract_scaled(dir, scale_factor * ideal_factor);
        SizeRules::new(min, ideal, (0, 0), Stretch::None)
    }

    /// Take horizontal/vertical axis component
    pub fn extract(self, dir: impl Directional) -> f32 {
        match dir.is_vertical() {
            false => self.0,
            true => self.1,
        }
    }

    /// Take component and scale
    pub fn extract_scaled(self, dir: impl Directional, scale_factor: f32) -> i32 {
        (self.extract(dir) * scale_factor).cast_nearest()
    }
}

impl From<(f32, f32)> for LogicalSize {
    #[inline]
    fn from((w, h): (f32, f32)) -> Self {
        LogicalSize(w, h)
    }
}

impl From<(i32, i32)> for LogicalSize {
    #[inline]
    fn from((w, h): (i32, i32)) -> Self {
        LogicalSize(w.cast(), h.cast())
    }
}

impl From<(u32, u32)> for LogicalSize {
    #[inline]
    fn from((w, h): (u32, u32)) -> Self {
        LogicalSize(w.cast(), h.cast())
    }
}

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
        Margins::hv_splat((size, size))
    }

    /// Margins via horizontal and vertical sizes
    #[inline]
    pub const fn hv(horiz: (u16, u16), vert: (u16, u16)) -> Self {
        Margins { horiz, vert }
    }

    /// Margins via horizontal and vertical sizes
    #[inline]
    pub const fn hv_splat((h, v): (u16, u16)) -> Self {
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
        Margins::hv_splat(size.cast())
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
    pub fn select(&self, mgr: SizeMgr) -> Margins {
        match self {
            MarginSelector::Outer => mgr.outer_margins(),
            MarginSelector::Inner => Margins::from(mgr.inner_margin()),
            MarginSelector::Text => mgr.text_margins(),
            MarginSelector::Fixed(fixed) => *fixed,
            MarginSelector::ScaledSplat(m) => {
                Margins::splat(u16::conv_nearest(m * mgr.scale_factor()))
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
    /// Prefer not to stretch beyond ideal size
    ///
    /// When `min == ideal`, this is as close to fixed-size as one can get, but
    /// still does not completely prevent stretching, e.g. if another widget in
    /// the same column causes it to be wider.
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

/// Sprite size
#[impl_default(SpriteSize::Relative(1.0))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpriteSize {
    /// Size in logical pixels
    Logical(LogicalSize),
    /// Scale relative to input size
    Relative(f32),
}

impl_scope! {
    /// Widget component for displaying a sprite
    #[impl_default]
    #[derive(Clone, Debug, PartialEq)]
    pub struct SpriteDisplay {
        /// Margins
        pub margins: MarginSelector,
        /// Display size
        pub size: SpriteSize,
        /// Ideal size relative to minimum size
        pub ideal_factor: f32 = 1.0,
        /// If true, output size must be an integer mulitple of the raw size
        ///
        /// Usually used for icons and pixel-art images.
        ///
        /// It is recommended to use `stretch == None` and `ideal_factor == 1.0`,
        /// since currently assignment of space does not use a "step" (this may
        /// change in the future). Regardless, the result of [`Self::align_rect`]
        /// should respect `int_scale_factor`.
        pub int_scale_factor: bool = false,
        /// If true, aspect ratio is fixed
        pub fix_aspect: bool = false,
        /// Widget stretchiness
        ///
        /// If is `None`, max size is limited.
        pub stretch: Stretch,
    }
}

impl SpriteDisplay {
    /// Calculate render size (physical pixels) from input (pixels)
    pub fn size_from_pixels(&self, raw_size: Size, scale_factor: f32) -> Size {
        match self.size {
            SpriteSize::Logical(logical) if self.int_scale_factor => {
                logical.to_physical(scale_factor.round())
            }
            SpriteSize::Logical(logical) => logical.to_physical(scale_factor),
            SpriteSize::Relative(rel) if self.int_scale_factor => {
                raw_size * i32::conv_nearest(rel * scale_factor)
            }
            SpriteSize::Relative(rel) => {
                (Vec2::conv(raw_size) * (rel * scale_factor)).cast_nearest()
            }
        }
    }

    /// Generates `size_rules` based on size
    ///
    /// Set [`Self::size`] before calling this.
    pub fn size_rules(&mut self, mgr: SizeMgr, axis: AxisInfo, raw_size: Size) -> SizeRules {
        let margins = self.margins.select(mgr.re()).extract(axis);
        let scale_factor = mgr.scale_factor();
        let min = self.size_from_pixels(raw_size, scale_factor).extract(axis);
        let ideal = self
            .size_from_pixels(raw_size, scale_factor * self.ideal_factor)
            .extract(axis);
        SizeRules::new(min, ideal, margins, self.stretch)
    }

    /// Constrains and aligns within `rect`
    ///
    /// If `self.stretch == Stretch::None`, maximum size is limited.
    /// If `self.fix_aspect`, size is corrected for aspect ratio.
    ///
    /// The resulting size is then aligned using the `align` hints, defaulting to centered.
    pub fn align_rect(
        &mut self,
        rect: Rect,
        align: AlignHints,
        raw_size: Size,
        scale_factor: f32,
    ) -> Rect {
        let mut size = rect.size;
        let raw_size = Vec2::conv(raw_size);
        if self.stretch == Stretch::None {
            let ideal = Size::conv_nearest(raw_size * (scale_factor * self.ideal_factor));
            size = size.min(ideal);
        }

        if self.fix_aspect {
            let ratio = Vec2::conv(size) / raw_size;
            // Use smaller ratio, which must be finite
            size = if !ratio.0.is_finite() || !ratio.1.is_finite() {
                size
            } else if self.int_scale_factor {
                let ratio = i32::conv_floor(ratio.0.min(ratio.1)).max(1);
                size * ratio
            } else if ratio.0 < ratio.1 {
                Size(size.0, i32::conv_nearest(ratio.0 * raw_size.1))
            } else {
                debug_assert!(ratio.1 < ratio.0);
                Size(i32::conv_nearest(ratio.1 * raw_size.0), size.1)
            };
        }

        align
            .complete(Align::Center, Align::Center)
            .aligned_rect(size, rect)
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
    /// The content's margins apply inside this frame. External margins come
    /// from this type.
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

    /// Variant: frame is content margin
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

    /// Variant: frame replaces content margin
    ///
    /// The content's margin is ignored. In other respects,
    /// this is the same as [`FrameRules::surround_with_margin`].
    pub fn surround_no_margin(self, content: SizeRules) -> (SizeRules, i32, i32) {
        let offset = self.offset + self.inner_margin;
        let size = self.size + 2 * self.inner_margin;

        let rules = SizeRules::new(
            content.min_size() + size,
            content.ideal_size() + size,
            self.m,
            content.stretch(),
        );
        (rules, offset, size)
    }
}
