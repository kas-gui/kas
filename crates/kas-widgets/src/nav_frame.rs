// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A "navigable" wrapper

use kas::{event, prelude::*};

impl_scope! {
    /// Navigation Frame wrapper
    ///
    /// This widget is a wrapper that can be used to make a static widget such as a
    /// `Label` navigable with the keyboard.
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(class_traits using self.inner where W: trait)]
    #[derive(Clone, Debug, Default)]
    #[widget{
        key_nav = true;
        layout = frame(kas::theme::FrameStyle::NavFocus): self.inner;
    }]
    pub struct NavFrame<W: Widget> {
        #[widget_core]
        core: CoreData,
        #[widget]
        pub inner: W,
    }

    impl Self {
        /// Construct a frame
        #[inline]
        pub fn new(inner: W) -> Self {
            NavFrame {
                core: Default::default(),
                inner,
            }
        }
    }

    impl event::Handler for Self {
        fn handle(&mut self, _mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::Activate => Response::Select,
                _ => Response::Unused,
            }
        }
    }
}
