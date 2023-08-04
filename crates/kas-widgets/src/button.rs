// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Push-buttons

use super::AccelLabel;
use kas::draw::color::Rgb;
use kas::event::Key;
use kas::prelude::*;
use std::fmt::Debug;

impl_scope! {
    /// A push-button with a generic label
    ///
    /// Default alignment of content is centered.
    #[autoimpl(class_traits using self.inner where W: trait)]
    #[widget {
        Data = W::Data;
        layout = button!(self.inner, color = self.color);
        navigable = true;
        hover_highlight = true;
    }]
    pub struct Button<W: Widget> {
        core: widget_core!(),
        key: Option<Key>,
        color: Option<Rgb>,
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
                color: None,
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

        /// Add accelerator key (chain style)
        #[must_use]
        pub fn with_key(mut self, key: Key) -> Self {
            debug_assert!(self.key.is_none());
            self.key = Some(key);
            self
        }

        /// Set button color
        pub fn set_color(&mut self, color: Option<Rgb>) {
            self.color = color;
        }

        /// Set button color (chain style)
        #[must_use]
        pub fn with_color(mut self, color: Rgb) -> Self {
            self.color = Some(color);
            self
        }
    }

    impl Events for Self {
        fn configure(&mut self, cx: &mut ConfigCx) {
            if let Some(key) = self.key.clone() {
                cx.add_accel_key(self.id_ref(), key);
            }
        }

        fn handle_event(&mut self, cx: &mut EventCx, data: &W::Data, event: Event) -> Response {
            event.on_activate(cx, self.id(), |cx| {
                if let Some(f) = self.on_press.as_ref() {
                    f(cx, data);
                }
                Response::Used
            })
        }

        fn handle_messages(&mut self, cx: &mut EventCx, data: &W::Data) {
            if let Some(kas::message::Activate) = cx.try_pop() {
                if let Some(f) = self.on_press.as_ref() {
                    f(cx, data);
                }
            }
        }
    }

    impl Button<AccelLabel> {
        /// Construct a button with the given `label`
        ///
        /// This is a convenience method. It may be possible to merge this
        /// functionality into [`Button::new`] once Rust has support for
        /// overlapping trait implementations (not specialisation).
        pub fn label(label: impl Into<AccelString>) -> Self {
            Button::new(AccelLabel::new(label))
        }

        /// Construct a button with the given `label` and payload `msg`
        ///
        /// This is a convenience method. It may be possible to merge this
        /// functionality into [`Button::new_msg`] once Rust has support for
        /// overlapping trait implementations (not specialisation).
        pub fn label_msg<M>(label: impl Into<AccelString>, msg: M) -> Self
        where
            M: Clone + Debug + 'static,
        {
            Button::new_msg(AccelLabel::new(label), msg)
        }
    }
}
