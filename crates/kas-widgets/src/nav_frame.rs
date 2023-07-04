// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A "navigable" wrapper

use kas::prelude::*;

impl_scope! {
    /// Navigation Frame wrapper
    ///
    /// This widget is a wrapper that can be used to make a static widget such as a
    /// `Label` navigable with the keyboard.
    ///
    /// # Messages
    ///
    /// When activated, this widget pushes [`Select`] to the message stack.
    ///
    /// [`Select`]: kas::message::Select
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(class_traits using self.inner where W: trait)]
    #[derive(Clone, Default)]
    #[widget{
        navigable = true;
        layout = frame!(self.inner, style = kas::theme::FrameStyle::NavFocus);
    }]
    pub struct NavFrame<W: Widget> {
        core: widget_core!(),
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

    impl Events for Self {
        type Data = ();

        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::Command(cmd) if cmd.is_activate() => {
                    mgr.push(kas::message::Select);
                    Response::Used
                }
                _ => Response::Unused,
            }
        }
    }
}
