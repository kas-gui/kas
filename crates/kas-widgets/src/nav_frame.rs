// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A "navigable" wrapper

use kas::{event, prelude::*};

widget! {
    /// Navigation Frame wrapper
    ///
    /// This widget is a wrapper that can be used to make a static widget such as a
    /// `Label` navigable with the keyboard.
    #[autoimpl(Deref, DerefMut on self.inner)]
    #[autoimpl(class_traits where W: trait on self.inner)]
    #[derive(Clone, Debug, Default)]
    #[widget{
        key_nav = true;
        layout = frame(self.inner, kas::theme::FrameStyle::NavFocus);
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
        type Msg = <W as Handler>::Msg;

        fn handle(&mut self, _mgr: &mut EventMgr, event: Event) -> Response<Self::Msg> {
            match event {
                Event::Activate => Response::Select,
                _ => Response::Unused,
            }
        }
    }
}
