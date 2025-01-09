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
    //
    // NOTE: this would use derive mode if that supported custom layout syntax,
    // but it does not. This would allow us to implement Deref to self.inner.
    #[autoimpl(class_traits using self.inner where W: trait)]
    #[derive(Clone, Default)]
    #[widget{
        Data = W::Data;
        layout = frame!(self.inner);
    }]
    pub struct Frame<W: Widget> {
        core: widget_core!(),
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
                inner,
            }
        }
    }
}
