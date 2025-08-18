// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Push-buttons

use super::AccessLabel;
use kas::event::Key;
use kas::prelude::*;
use kas::theme::{Background, FrameStyle};
use std::fmt::Debug;

#[impl_self]
mod Button {
    /// A push-button with a generic label
    ///
    /// Default alignment of content is centered.
    ///
    /// ### Messages
    ///
    /// [`kas::messages::Activate`] may be used to trigger the button.
    #[widget]
    #[layout(
        frame!(self.inner)
            .with_style(FrameStyle::Button)
            .with_background(self.bg)
            .align(AlignHints::CENTER)
    )]
    pub struct Button<W: Widget> {
        core: widget_core!(),
        key: Option<Key>,
        bg: Background,
        #[widget]
        pub inner: W,
        on_press: Option<Box<dyn Fn(&mut EventCx, &W::Data)>>,
    }

    impl Self {
        /// Construct a button with given `inner` widget
        #[inline]
        pub fn new(inner: W) -> Self {
            Button {
                core: Default::default(),
                key: Default::default(),
                bg: Background::Default,
                inner,
                on_press: None,
            }
        }

        /// Call the handler `f` on press / activation
        #[inline]
        #[must_use]
        pub fn with(mut self, f: impl Fn(&mut EventCx, &W::Data) + 'static) -> Self {
            debug_assert!(self.on_press.is_none());
            self.on_press = Some(Box::new(f));
            self
        }

        /// Send the message `msg` on press / activation
        #[inline]
        #[must_use]
        pub fn with_msg<M>(self, msg: M) -> Self
        where
            M: Clone + Debug + 'static,
        {
            self.with(move |cx, _| cx.push(msg.clone()))
        }

        /// Construct a button with a given `inner` and payload `msg`
        ///
        /// When the button is activated, a clone of `msg` is sent to the
        /// parent widget. The parent (or an ancestor) should handle this using
        /// [`Events::handle_messages`].
        #[inline]
        pub fn new_msg<M: Clone + Debug + 'static>(inner: W, msg: M) -> Self {
            Self::new(inner).with_msg(msg)
        }

        /// Add access key (chain style)
        #[must_use]
        pub fn with_access_key(mut self, key: Key) -> Self {
            debug_assert!(self.key.is_none());
            self.key = Some(key);
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

    impl Tile for Self {
        fn navigable(&self) -> bool {
            true
        }

        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::Button
        }

        fn probe(&self, _: Coord) -> Id {
            self.id()
        }
    }

    impl Events for Self {
        const REDRAW_ON_MOUSE_OVER: bool = true;

        type Data = W::Data;

        fn configure(&mut self, cx: &mut ConfigCx) {
            if let Some(key) = self.key.clone() {
                cx.add_access_key(self.id_ref(), key);
            }
        }

        fn handle_event(&mut self, cx: &mut EventCx, data: &W::Data, event: Event) -> IsUsed {
            event.on_click(cx, self.id(), |cx| {
                if let Some(f) = self.on_press.as_ref() {
                    f(cx, data);
                }
            })
        }

        fn handle_messages(&mut self, cx: &mut EventCx, data: &W::Data) {
            if let Some(kas::messages::Activate(code)) = cx.try_pop() {
                if let Some(f) = self.on_press.as_ref() {
                    f(cx, data);
                }
                cx.depress_with_key(&self, code);
            }
        }
    }

    impl Button<AccessLabel> {
        /// Construct a button with the given `label`
        ///
        /// This is a convenience method. It may be possible to merge this
        /// functionality into [`Button::new`] once Rust has support for
        /// overlapping trait implementations (not specialisation).
        pub fn label(label: impl Into<AccessString>) -> Self {
            Button::new(AccessLabel::new(label))
        }

        /// Construct a button with the given `label` and payload `msg`
        ///
        /// This is a convenience method. It may be possible to merge this
        /// functionality into [`Button::new_msg`] once Rust has support for
        /// overlapping trait implementations (not specialisation).
        pub fn label_msg<M>(label: impl Into<AccessString>, msg: M) -> Self
        where
            M: Clone + Debug + 'static,
        {
            Button::new_msg(AccessLabel::new(label), msg)
        }
    }
}
