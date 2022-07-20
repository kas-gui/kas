// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! "Handle" types used by themes

use std::ops::Deref;

use super::{Feature, FrameStyle, TextClass};
use crate::dir::Directional;
use crate::geom::{Rect, Size};
use crate::layout::{AlignHints, AxisInfo, FrameRules, Margins, SizeRules};
use crate::macros::autoimpl;
use crate::text::{Align, TextApi};
#[allow(unused)]
use crate::{event::ConfigMgr, theme::DrawMgr};

// for doc use
#[allow(unused)]
use crate::text::TextApiExt;

/// Size and scale interface
///
/// This interface is provided to widgets in [`crate::Layout::size_rules`].
/// It may also be accessed through [`crate::event::EventMgr::size_mgr`],
/// [`DrawMgr::size_mgr`].
///
/// Most methods get or calculate the size of some feature. These same features
/// may be drawn through [`DrawMgr`].
pub struct SizeMgr<'a>(&'a dyn ThemeSize);

impl<'a> SizeMgr<'a> {
    /// Construct from a [`ThemeSize`]
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub fn new(h: &'a dyn ThemeSize) -> Self {
        SizeMgr(h)
    }

    /// Reborrow with a new lifetime
    ///
    /// Rust allows references like `&T` or `&mut T` to be "reborrowed" through
    /// coercion: essentially, the pointer is copied under a new, shorter, lifetime.
    /// Until rfcs#1403 lands, reborrows on user types require a method call.
    ///
    /// Calling this method is zero-cost.
    #[inline(always)]
    pub fn re<'b>(&'b self) -> SizeMgr<'b>
    where
        'a: 'b,
    {
        SizeMgr(self.0)
    }

    /// Get the scale factor
    ///
    /// "Traditional" PC screens have a scale factor of 1; high-DPI screens
    /// may have a factor of 2 or higher. This may be fractional and may be
    /// adjusted to the user's taste.
    ///
    /// DPI is usually `96.0 * scale_factor` but this may be inaccurate (due to
    /// user's preference or inaccurate screen size measurements or some form
    /// of scaling used on mobile devices).
    ///
    /// It is recommended to calculate integer pixel sizes as follows:
    /// ```
    /// use kas_core::cast::*;
    /// # let scale_factor = 1.5f32;
    /// let size: i32 = (100.0 * scale_factor).cast_ceil();
    /// ```
    ///
    /// This value may change during a program's execution (e.g. when a window
    /// is moved to a different monitor); in this case all widgets will be
    /// resized via [`crate::Layout::size_rules`].
    pub fn scale_factor(&self) -> f32 {
        self.0.scale_factor()
    }

    /// Get the Em size of the standard font in pixels
    ///
    /// The Em is a unit of typography, corresponding to the distance between
    /// ascent and descent bounding lines (thus, the line height).
    ///
    /// This method returns the size of 1 Em in physical pixels.
    pub fn dpem(&self) -> f32 {
        self.0.dpem()
    }

    /// The minimum size of a scrollable area
    pub fn min_scroll_size(&self, axis: impl Directional) -> i32 {
        self.0.min_scroll_size(axis.is_vertical())
    }

    /// Minimum length of a dragable handle for a scroll bar or slider
    pub fn handle_len(&self) -> i32 {
        self.0.handle_len()
    }

    /// The margin around content within a widget
    ///
    /// Though inner margins are *usually* empty, they are sometimes drawn to,
    /// for example focus indicators.
    pub fn inner_margin(&self) -> Size {
        self.0.inner_margin()
    }

    /// The margin between UI elements, where desired
    ///
    /// Widgets must not draw in outer margins.
    pub fn outer_margins(&self) -> Margins {
        self.0.outer_margins()
    }

    /// The margin around text elements
    ///
    /// Similar to [`Self::outer_margins`], but intended for things like text
    /// labels which do not have a visible hard edge.
    pub fn text_margins(&self) -> Margins {
        self.0.text_margins()
    }

    /// Size rules for a feature
    pub fn feature(&self, feature: Feature, axis: impl Directional) -> SizeRules {
        self.0.feature(feature, axis.is_vertical())
    }

    /// Size of a frame around another element
    pub fn frame(&self, style: FrameStyle, axis: impl Directional) -> FrameRules {
        self.0.frame(style, axis.is_vertical())
    }

    /// Update a text object, setting font properties and getting a size bound
    ///
    /// This method updates the text's [`Environment`] and uses the result to
    /// calculate size requirements.
    ///
    /// It is necessary to update the environment *again* once the target `rect`
    /// is known: use [`ConfigMgr::text_set_size`] to do this.
    ///
    /// [`Environment`]: crate::text::Environment
    pub fn text_bound(
        &self,
        text: &mut dyn TextApi,
        class: TextClass,
        axis: AxisInfo,
    ) -> SizeRules {
        self.0.text_bound(text, class, axis)
    }
}

/// Theme sizing implementation
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
#[autoimpl(for<S: trait + ?Sized, R: Deref<Target = S>> R)]
pub trait ThemeSize {
    /// Get the scale factor
    fn scale_factor(&self) -> f32;

    /// Get the Em size of the standard font in pixels
    fn dpem(&self) -> f32;

    /// The minimum size of a scrollable area
    fn min_scroll_size(&self, axis_is_vertical: bool) -> i32;

    /// The length of a dragable handle for a scroll bar or slider
    ///
    /// This is the length in line with the control. The size on the opposite
    /// axis is assumed to be equal to the feature size as reported by
    /// [`Self::feature`].
    fn handle_len(&self) -> i32;

    /// The margin around content within a widget
    ///
    /// Though inner margins are *usually* empty, they are sometimes drawn to,
    /// for example focus indicators.
    fn inner_margin(&self) -> Size;

    /// The margin between UI elements, where desired
    ///
    /// Widgets must not draw in outer margins.
    fn outer_margins(&self) -> Margins;

    /// The margin around text elements
    ///
    /// Similar to [`Self::outer_margins`], but intended for things like text
    /// labels which do not have a visible hard edge.
    fn text_margins(&self) -> Margins;

    /// Size rules for a feature
    fn feature(&self, feature: Feature, axis_is_vertical: bool) -> SizeRules;

    /// Align a feature's rect
    ///
    /// In case the input `rect` is larger than desired on either axis, it is
    /// reduced in size and offset within the original `rect` as is preferred.
    fn align_feature(&self, feature: Feature, rect: Rect, hints: AlignHints) -> Rect;

    /// Size of a frame around another element
    fn frame(&self, style: FrameStyle, axis_is_vertical: bool) -> FrameRules;

    /// Update a text object, setting font properties and getting a size bound
    ///
    /// This method updates the text's [`Environment`] and uses the result to
    /// calculate size requirements.
    ///
    /// It is necessary to update the environment *again* once the target `rect`
    /// is known: use [`Self::text_set_size`] to do this.
    ///
    /// [`Environment`]: crate::text::Environment
    fn text_bound(&self, text: &mut dyn TextApi, class: TextClass, axis: AxisInfo) -> SizeRules;

    /// Update a text object, setting font properties and wrap size
    fn text_set_size(
        &self,
        text: &mut dyn TextApi,
        class: TextClass,
        size: Size,
        align: (Align, Align),
    );
}
