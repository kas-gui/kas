// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A simple frame

use kas::prelude::*;

impl_scope! {
    /// A frame around content
    ///
    /// This widget provides a simple abstraction: drawing a frame around its
    /// contents.
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(class_traits using self.inner where W: trait)]
    #[derive(Clone, Default)]
    #[widget{
        layout = frame!(self.inner);
    }]
    pub struct Frame<W: Widget> {
        core: widget_core!(),
        #[widget]
        pub inner: W,
    }

    impl Self {
        /// Construct a frame
        #[inline]
        pub fn new(inner: W) -> Self {
            Frame {
                core: Default::default(),
                inner,
            }
        }
    }
}

impl_scope! {
    /// A frame around pop-ups
    ///
    /// It is expected that this be the top-most widget inside any popup.
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(class_traits using self.inner where W: trait)]
    #[derive(Clone, Default)]
    #[widget{
        layout = frame!(self.inner, style = kas::theme::FrameStyle::Popup);
    }]
    pub struct PopupFrame<W: Widget> {
        core: widget_core!(),
        #[widget]
        pub inner: W,
    }

    impl Self {
        /// Construct a frame
        #[inline]
        pub fn new(inner: W) -> Self {
            PopupFrame {
                core: Default::default(),
                inner,
            }
        }
    }
}
