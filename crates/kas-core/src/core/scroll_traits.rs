// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Scroll bar traits

use crate::event::EventCx;
use crate::geom::{Offset, Size};
use crate::Widget;
#[allow(unused)] use crate::{Events, Layout};

/// Additional functionality on scrollable widgets
///
/// This trait should be implemented by widgets supporting scrolling, enabling
/// a parent to control scrolling.
///
/// If the widget scrolls itself it should set a scroll action via [`EventCx::set_scroll`].
pub trait Scrollable: Widget {
    /// Given size `size`, returns whether `(horiz, vert)` scrolling is required
    ///
    /// Note: this is called *before* [`Layout::set_rect`], thus must may need
    /// to perform independent calculation of the content size.
    fn scroll_axes(&self, size: Size) -> (bool, bool);

    /// Get the maximum scroll offset
    ///
    /// Note: the minimum scroll offset is always zero.
    ///
    /// Note: this is called immediately after [`Layout::set_rect`], thus should
    /// be updated there (as well as by [`Events::update`] if appropriate).
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
    fn set_scroll_offset(&mut self, cx: &mut EventCx, offset: Offset) -> Offset;
}
