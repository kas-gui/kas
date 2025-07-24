// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A simple frame

use kas::prelude::*;
use kas::theme::{Background, FrameStyle};

/// Make a [`Frame`] widget
///
/// # Syntax
///
/// ## Stand-alone usage
///
/// When called as a stand-alone macro, `frame!(inner)` is just syntactic sugar
/// for `Frame::new(inner)`, and yes, this makes the macro pointless.
///
/// ## Usage within widget layout syntax
///
/// When called within [widget layout syntax], `frame!` may be evaluated as a
/// recursive macro and the result does not have a specified type, except that
/// methods [`map_any`], [`align`], [`pack`] and [`with_style`] are supported
/// via emulation.
///
/// # Example
///
/// ```
/// let my_widget = kas_widgets::frame!(kas_widgets::Label::new("content"));
/// ```
///
/// [widget layout syntax]: macro@kas::layout
/// [`map_any`]: crate::AdaptWidgetAny::map_any
/// [`align`]: crate::AdaptWidget::align
/// [`pack`]: crate::AdaptWidget::pack
/// [`with_style`]: Frame::with_style
#[macro_export]
macro_rules! frame {
    ( $e:expr ) => {
        $crate::Frame::new($e)
    };
}

#[impl_self]
mod Frame {
    /// A frame around content
    ///
    /// This widget provides a simple abstraction: drawing a frame around its
    /// contents.
    //
    // NOTE: this would use derive mode if that supported custom layout syntax,
    // but it does not. This would allow us to implement Deref to self.inner.
    #[derive(Clone, Default)]
    #[widget]
    #[layout(frame!(self.inner).with_style(self.style))]
    pub struct Frame<W: Widget> {
        core: widget_core!(),
        style: FrameStyle,
        bg: Background,
        /// The inner widget
        #[widget]
        pub inner: W,
    }

    impl Events for Self {
        type Data = W::Data;
    }

    impl Self {
        /// Construct a frame
        #[inline]
        pub fn new(inner: W) -> Self {
            Frame {
                core: Default::default(),
                style: FrameStyle::Frame,
                bg: Background::default(),
                inner,
            }
        }

        /// Set the frame style (inline)
        ///
        /// The default style is [`FrameStyle::Frame`].
        ///
        /// Note: using [`FrameStyle::NavFocus`] does not automatically make
        /// this widget interactive.
        #[inline]
        #[must_use]
        pub fn with_style(mut self, style: FrameStyle) -> Self {
            self.style = style;
            self
        }

        /// Set the frame background color (inline)
        ///
        /// The default background is [`Background::Default`].
        #[inline]
        #[must_use]
        pub fn with_background(mut self, bg: Background) -> Self {
            self.bg = bg;
            self
        }
    }
}
