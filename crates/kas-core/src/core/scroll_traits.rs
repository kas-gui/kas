// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Scroll bar traits

use super::Widget;
use crate::event::EventMgr;
use crate::geom::{Offset, Size};
use crate::Action;

/// Additional functionality on scrollable widgets
///
/// This trait should be implemented by widgets supporting scrolling, enabling
/// a parent to control scrolling.
///
/// If the widget scrolls itself it should set a scroll action via [`EventMgr::set_scroll`].
pub trait Scrollable: Widget {
    /// Given size `size`, returns whether `(horiz, vert)` scrolling is required
    fn scroll_axes(&self, size: Size) -> (bool, bool);

    /// Get the maximum scroll offset
    ///
    /// Note: the minimum scroll offset is always zero.
    fn max_scroll_offset(&self) -> Offset;

    /// Get the current scroll offset
    ///
    /// Contents of the scroll region are translated by this offset (to convert
    /// coordinates from the outer region to the scroll region, add this offset).
    ///
    /// The offset is restricted between [`Offset::ZERO`] and
    /// [`Self::max_scroll_offset`].
    fn scroll_offset(&self) -> Offset;

    /// Set the scroll offset
    ///
    /// This may be used for programmatic scrolling, e.g. by a wrapping widget
    /// with scroll controls. Note that calling this method directly on the
    /// scrolling widget will not update any controls in a wrapping widget.
    ///
    /// The offset is clamped to the available scroll range and applied. The
    /// resulting offset is returned.
    fn set_scroll_offset(&mut self, mgr: &mut EventMgr, offset: Offset) -> Offset;
}

/// Scroll bar mode
///
/// Note that in addition to this mode, bars may be disabled on each axis.
#[kas_macros::impl_default(ScrollBarMode::Auto)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ScrollBarMode {
    /// Scroll bars are always shown if enabled.
    Fixed,
    /// Automatically enable/disable scroll bars as required when resized.
    ///
    /// This has the side-effect of reserving enough space for scroll bars even
    /// when not required.
    Auto,
    /// Scroll bars float over content and are only drawn when hovered over by
    /// the mouse.
    Invisible,
}

/// Scroll bar control
pub trait HasScrollBars {
    /// Get mode
    fn get_mode(&self) -> ScrollBarMode;

    /// Set mode
    fn set_mode(&mut self, mode: ScrollBarMode) -> Action;

    /// Get currently visible bars
    ///
    /// Returns `(horiz, vert)` tuple.
    fn get_visible_bars(&self) -> (bool, bool);

    /// Set enabled bars without adjusting mode
    ///
    /// Note: if mode is `Auto` this has no effect.
    ///
    /// This requires a [`Action::RESIZE`].
    fn set_visible_bars(&mut self, bars: (bool, bool)) -> Action;

    /// Set auto mode (inline)
    #[inline]
    fn with_auto_bars(mut self) -> Self
    where
        Self: Sized,
    {
        let _ = self.set_mode(ScrollBarMode::Auto);
        self
    }

    /// Set fixed bars (inline)
    #[inline]
    fn with_fixed_bars(mut self, horiz: bool, vert: bool) -> Self
    where
        Self: Sized,
    {
        let _ = self.set_mode(ScrollBarMode::Fixed);
        let _ = self.set_visible_bars((horiz, vert));
        self
    }

    /// Set invisible bars (inline)
    #[inline]
    fn with_invisible_bars(mut self, horiz: bool, vert: bool) -> Self
    where
        Self: Sized,
    {
        let _ = self.set_mode(ScrollBarMode::Invisible);
        let _ = self.set_visible_bars((horiz, vert));
        self
    }
}
