// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A "navigable" wrapper

use kas::prelude::*;

#[impl_self]
mod NavFrame {
    /// Navigation Frame wrapper
    ///
    /// This widget is a wrapper that can be used to make a static widget such as a
    /// `Label` navigable with the keyboard.
    ///
    /// # Messages
    ///
    /// When activated, this widget pushes [`Select`] to the message stack.
    ///
    /// [`Select`]: kas::messages::Select
    #[derive(Clone, Default)]
    #[widget]
    #[layout(frame!(self.inner).with_style(kas::theme::FrameStyle::NavFocus))]
    pub struct NavFrame<W: Widget> {
        core: widget_core!(),
        /// The inner widget
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
        const NAVIGABLE: bool = true;

        type Data = W::Data;

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::Command(cmd, code) if cmd.is_activate() => {
                    cx.depress_with_key(self.id(), code);
                    cx.push(kas::messages::Select);
                    Used
                }
                _ => Unused,
            }
        }
    }
}
