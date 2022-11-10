// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! "Handle" types used by themes

use std::ops::Deref;

use super::{Feature, FrameStyle, MarginStyle, TextClass};
use crate::autoimpl;
use crate::dir::Directional;
use crate::geom::{Rect, Size};
use crate::layout::{AlignPair, AxisInfo, FrameRules, Margins, SizeRules};
use crate::text::TextApi;

#[allow(unused)] use crate::text::TextApiExt;
#[allow(unused)]
use crate::{event::ConfigMgr, layout::Stretch, theme::DrawMgr};

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
    /// adjusted to suit the device type (e.g. a phone or desktop monitor) as
    /// well as the user's preference.
    ///
    /// One could use this value to calculate physical size, but be warned that
    /// the result may be quite inaccurate on anything other than a desktop
    /// monitor: `25.4 mm = 1 inch = (96 * scale_factor) pixels`
    ///
    /// To calculate screen pixel sizes from virtual pixel sizes:
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

    /// The Em size of the standard font in pixels
    ///
    /// The Em is a unit of typography (variously defined as the point-size of
    /// the font, the height of the font or the width of an upper-case `M`).
    /// The method [`Self::line_height`] returns a related but distinct value.
    ///
    /// This method returns the size of 1 Em in physical pixels, derived from
    /// the font size in use by the theme and the screen's scale factor.
    pub fn dpem(&self) -> f32 {
        self.0.dpem()
    }

    /// The minimum size of a scrollable area
    pub fn min_scroll_size(&self, axis: impl Directional) -> i32 {
        self.0.min_scroll_size(axis.is_vertical())
    }

    /// The length of a dragable handle for a scroll bar or slider
    ///
    /// This is the length in line with the control. The size on the opposite
    /// axis is assumed to be equal to the feature size as reported by
    /// [`Self::feature`].
    pub fn handle_len(&self) -> i32 {
        self.0.handle_len()
    }

    /// The width of a vertical scroll bar
    ///
    /// This value is also available through [`Self::feature`].
    pub fn scroll_bar_width(&self) -> i32 {
        self.0.scroll_bar_width()
    }

    /// Get margin size
    pub fn margins(&self, style: MarginStyle) -> Margins {
        self.0.margins(style)
    }

    /// Get margins for [`MarginStyle::Inner`]
    pub fn inner_margins(&self) -> Margins {
        self.0.margins(MarginStyle::Inner)
    }

    /// Get margins for [`MarginStyle::Tiny`]
    pub fn tiny_margins(&self) -> Margins {
        self.0.margins(MarginStyle::Tiny)
    }

    /// Get margins for [`MarginStyle::Small`]
    pub fn small_margins(&self) -> Margins {
        self.0.margins(MarginStyle::Small)
    }

    /// Get margins for [`MarginStyle::Large`]
    pub fn large_margins(&self) -> Margins {
        self.0.margins(MarginStyle::Large)
    }

    /// Get margins for [`MarginStyle::Text`]
    pub fn text_margins(&self) -> Margins {
        self.0.margins(MarginStyle::Text)
    }

    /// Size rules for a feature
    pub fn feature(&self, feature: Feature, axis: impl Directional) -> SizeRules {
        self.0.feature(feature, axis.is_vertical())
    }

    /// Size of a frame around another element
    pub fn frame(&self, style: FrameStyle, axis: impl Directional) -> FrameRules {
        self.0.frame(style, axis.is_vertical())
    }

    /// The height of a line of text using the corresponding font
    ///
    /// This method looks up the font face corresponding to the given `class`,
    /// scales this according to [`Self::dpem`], then measures the line height.
    /// The result is typically 100% - 150% of the value returned by
    /// [`Self::dpem`], depending on the font face.
    pub fn line_height(&self, class: TextClass) -> i32 {
        self.0.line_height(class)
    }

    /// Get [`SizeRules`] for a text element
    ///
    /// The [`TextClass`] is used to select a font and controls whether line
    /// wrapping is enabled.
    ///
    /// Alignment is set from [`AxisInfo::align_or_default`]. If other alignment
    /// is desired, modify `axis` before calling this method.
    ///
    /// Horizontal size without wrapping is simply the size the text.
    /// Horizontal size with wrapping is bounded to some width dependant on the
    /// theme, and may have non-zero [`Stretch`] depending on the size.
    ///
    /// Vertical size is the size of the text with or without wrapping, but with
    /// the minimum at least the height of one line of text.
    ///
    /// Widgets with editable text contents or internal scrolling enabled may
    /// wish to adjust the result.
    ///
    /// Note: this method partially prepares the `text` object. It is not
    /// required to call this method but it is required to call
    /// [`ConfigMgr::text_set_size`] before text display for correct results.
    pub fn text_rules(
        &self,
        text: &mut dyn TextApi,
        class: TextClass,
        axis: AxisInfo,
    ) -> SizeRules {
        self.0.text_rules(text, class, axis)
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
    fn handle_len(&self) -> i32;

    /// The width of a vertical scroll bar
    fn scroll_bar_width(&self) -> i32;

    /// Get margin size
    fn margins(&self, style: MarginStyle) -> Margins;

    /// Size rules for a feature
    fn feature(&self, feature: Feature, axis_is_vertical: bool) -> SizeRules;

    /// Align a feature's rect
    ///
    /// In case the input `rect` is larger than desired on either axis, it is
    /// reduced in size and offset within the original `rect` as is preferred.
    fn align_feature(&self, feature: Feature, rect: Rect, align: AlignPair) -> Rect;

    /// Size of a frame around another element
    fn frame(&self, style: FrameStyle, axis_is_vertical: bool) -> FrameRules;

    /// The height of a line of text using the standard font
    fn line_height(&self, class: TextClass) -> i32;

    /// Get [`SizeRules`] for a text element
    fn text_rules(&self, text: &mut dyn TextApi, class: TextClass, axis: AxisInfo) -> SizeRules;

    /// Update a text object, setting font properties and wrap size
    fn text_set_size(
        &self,
        text: &mut dyn TextApi,
        class: TextClass,
        size: Size,
        align: Option<AlignPair>,
    );
}
