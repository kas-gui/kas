// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A simple frame

use kas::prelude::*;

widget! {
    /// A frame around content
    ///
    /// This widget provides a simple abstraction: drawing a frame around its
    /// contents.
    #[autoimpl(Deref, DerefMut on self.inner)]
    #[autoimpl(class_traits where W: trait on self.inner)]
    #[derive(Clone, Debug, Default)]
    #[handler(msg = <W as Handler>::Msg)]
    #[widget{
        layout = frame(self.inner, kas::theme::FrameStyle::Frame);
    }]
    pub struct Frame<W: Widget> {
        #[widget_core]
        core: CoreData,
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
