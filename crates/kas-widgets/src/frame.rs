// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A simple frame

use kas::prelude::*;
use kas::theme::FrameStyle;

/// Make a [`Frame`] widget
///
/// When called as a stand-alone macro, `frame!(inner)` is just syntactic sugar
/// for `Frame::new(inner)`, and yes, this makes the macro pointless.
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
/// [widget layout syntax]: macro@widget#layout-1
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

impl_scope! {
    /// A frame around content
    ///
    /// This widget provides a simple abstraction: drawing a frame around its
    /// contents.
    //
    // NOTE: this would use derive mode if that supported custom layout syntax,
    // but it does not. This would allow us to implement Deref to self.inner.
    #[derive(Clone, Default)]
    #[widget{
        Data = W::Data;
        layout = frame!(self.inner).with_style(self.style);
    }]
    pub struct Frame<W: Widget> {
        core: widget_core!(),
        style: FrameStyle,
        /// The inner widget
        #[widget]
        pub inner: W,
    }

    impl Self {
        /// Construct a frame
        #[inline]
        pub fn new(inner: W) -> Self {
            Frame {
                core: Default::default(),
                style: FrameStyle::Frame,
                inner,
            }
        }

        /// Set the frame style (inline)
        ///
        /// The default style is [`FrameStyle::Frame`].
        ///
        /// Note: using [`FrameStyle::NavFocus`] does not automatically make
        /// this widget interactive. Use [`NavFrame`](crate::NavFrame) for that.
        #[inline]
        #[must_use]
        pub fn with_style(mut self, style: FrameStyle) -> Self {
            self.style = style;
            self
        }
    }
}
