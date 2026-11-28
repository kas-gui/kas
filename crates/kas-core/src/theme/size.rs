// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! "Handle" types used by themes

use super::{BarStyle, BoxStyle, FrameStyle, MarginStyle, MarkStyle, TextClass};
use crate::dir::{Direction, Directional};
use crate::event::{CanRedraw, EventState};
use crate::geom::Rect;
use crate::layout::{AlignPair, FrameRules, LogicalBuilder, Margins, SizeRules};
use crate::theme::dimensions::Window;
use std::ops::{Deref, DerefMut};

#[allow(unused)] use super::DrawCx;

/// Size and scaling interface
///
/// This context provides scaled sizing information from the theme.
///
/// Most methods get or calculate the size of some feature. These same features
/// may be drawn through [`DrawCx`].
pub struct SizeCx<'a> {
    ev: &'a mut EventState,
    w: &'a Window,
}

impl<'a> Deref for SizeCx<'a> {
    type Target = EventState;
    fn deref(&self) -> &EventState {
        self.ev
    }
}
impl<'a> DerefMut for SizeCx<'a> {
    fn deref_mut(&mut self) -> &mut EventState {
        self.ev
    }
}

impl<'a> SizeCx<'a> {
    /// Construct from [`EventState`] and theme window data
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    #[inline]
    pub fn new(ev: &'a mut EventState, w: &'a Window) -> Self {
        SizeCx { ev, w }
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
    /// let size: i32 = (100.0 * scale_factor).cast_to(Ceil);
    /// ```
    ///
    /// This value may change during a program's execution (e.g. when a window
    /// is moved to a different monitor); in this case all widgets will be
    /// resized via [`crate::Layout::size_rules`].
    #[inline]
    pub fn scale_factor(&self) -> f32 {
        self.w.scale_factor()
    }

    /// Build [`SizeRules`] from the input size in logical pixels
    #[inline]
    pub fn logical(&self, width: f32, height: f32) -> LogicalBuilder {
        LogicalBuilder::new((width, height), self.scale_factor())
    }

    /// Get the default font size for `class`
    ///
    /// Units are physical pixels per font Em.
    ///
    /// The Em is a unit of typography (variously defined as the point-size of
    /// the font, the height of the font or the width of an upper-case `M`).
    ///
    /// This method returns the size of 1 Em in physical pixels, derived from
    /// the font size in use by the theme and the screen's scale factor.
    #[inline]
    pub fn dpem(&self, class: TextClass) -> f32 {
        self.w.dpem(class)
    }

    /// Get the `(min, ideal)` line length for text which wraps
    #[inline]
    pub fn wrapped_line_len(&self, class: TextClass, dpem: f32) -> (i32, i32) {
        self.w.wrapped_line_len(class, dpem)
    }

    /// The smallest reasonable size for a visible (non-frame) component
    ///
    /// This is used as a suggestion by some heuristics.
    #[inline]
    pub fn min_element_size(&self) -> i32 {
        self.w.min_element_size()
    }

    /// The minimum size of a scrollable area
    #[inline]
    pub fn min_scroll_size(&self, axis: impl Directional, class: Option<TextClass>) -> i32 {
        let class = class.unwrap_or(TextClass::Standard);
        self.w.min_scroll_size(axis.is_vertical(), class)
    }

    /// The length of the grip (draggable handle) on a scroll bar or slider
    ///
    /// This is the length in line with the control. The size on the opposite
    /// axis is assumed to be equal to the feature size as reported by
    /// [`Self::bar`].
    #[inline]
    pub fn grip_len(&self) -> i32 {
        self.w.grip_len()
    }

    /// The width of a vertical scroll bar
    ///
    /// This value is also available through [`Self::bar`].
    #[inline]
    pub fn scroll_bar_width(&self) -> i32 {
        self.w.scroll_bar_width()
    }

    /// Get margin size
    #[inline]
    pub fn margins(&self, style: MarginStyle) -> Margins {
        self.w.margins(style)
    }

    /// Get margins for [`MarginStyle::Inner`]
    #[inline]
    pub fn inner_margins(&self) -> Margins {
        self.w.margins(MarginStyle::Inner)
    }

    /// Get margins for [`MarginStyle::Tiny`]
    #[inline]
    pub fn tiny_margins(&self) -> Margins {
        self.w.margins(MarginStyle::Tiny)
    }

    /// Get margins for [`MarginStyle::Small`]
    #[inline]
    pub fn small_margins(&self) -> Margins {
        self.w.margins(MarginStyle::Small)
    }

    /// Get margins for [`MarginStyle::Large`]
    #[inline]
    pub fn large_margins(&self) -> Margins {
        self.w.margins(MarginStyle::Large)
    }

    /// Get margins for [`MarginStyle::Text`]
    #[inline]
    pub fn text_margins(&self) -> Margins {
        self.w.margins(MarginStyle::Text)
    }

    /// Size rules for a bar feature with the given `dir`ection
    #[inline]
    pub fn bar(&self, style: BarStyle, dir: Direction, axis: impl Directional) -> SizeRules {
        self.w.bar(style, dir, axis.is_vertical())
    }

    /// Align a bar within `rect`
    ///
    /// Bars always stretch in their given `dir`ection, and are centralised on
    /// the opposite axis.
    #[inline]
    pub fn align_bar(&self, style: BarStyle, dir: Direction, rect: Rect) -> Rect {
        self.w.align_bar(style, dir, rect)
    }

    /// Size rules for a box feature
    ///
    /// Note: since `box` is a reserved keyword, this method must be called
    /// using raw identifier syntax: `cx.r#box(style, axis)`.
    #[inline]
    pub fn r#box(&self, style: BoxStyle, _axis: impl Directional) -> SizeRules {
        self.w.r#box(style)
    }

    /// Align a box within `rect`
    ///
    /// In case the input `rect` is larger than desired on either axis, it is
    /// reduced in size and offset within the original `rect` as is preferred.
    #[inline]
    pub fn align_box(&self, style: BoxStyle, rect: Rect, align: AlignPair) -> Rect {
        self.w.align_box(style, rect, align)
    }

    /// Size rules for a mark
    #[inline]
    pub fn mark(&self, style: MarkStyle, axis: impl Directional) -> SizeRules {
        self.w.mark(style, axis.is_vertical())
    }

    /// Size of a frame around another element
    #[inline]
    pub fn frame(&self, style: FrameStyle, axis: impl Directional) -> FrameRules {
        self.w.frame(style, axis.is_vertical())
    }
}

/// [`SizeCx`] implements [`CanRedraw`] for compatibility with trait users
///
/// Since a redraw is implied after sizing, [`Self::redraw`] does nothing.
impl<'a> CanRedraw for SizeCx<'a> {
    #[inline]
    fn event_state(&self) -> &EventState {
        self.ev
    }
    #[inline]
    fn event_state_mut(&mut self) -> &mut EventState {
        self.ev
    }
    #[inline]
    fn size_cx<'b>(&'b mut self) -> SizeCx<'b>
    where
        'a: 'b,
    {
        SizeCx::new(self.ev, self.w)
    }
    #[inline]
    fn redraw(&mut self) {}
}
