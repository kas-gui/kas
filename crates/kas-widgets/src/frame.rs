// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A simple frame

use kas::macros::make_layout;
use kas::{event, layout, prelude::*};

widget! {
    /// A frame around content
    ///
    /// This widget provides a simple abstraction: drawing a frame around its
    /// contents.
    #[autoimpl(Deref, DerefMut on inner)]
    #[autoimpl(class_traits where W: trait on inner)]
    #[derive(Clone, Debug, Default)]
    #[handler(msg = <W as Handler>::Msg)]
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

    impl Layout for Self {
        fn layout<'a>(&'a mut self) -> layout::Layout<'a> {
            make_layout!(self.core; frame(self.inner))
        }

        fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
            draw_handle.outer_frame(self.core_data().rect);
            let disabled = disabled || self.is_disabled();
            self.inner.draw(draw_handle, mgr, disabled);
        }
    }
}
