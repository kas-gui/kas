// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Push-buttons

use crate::AccelLabel;
use kas::draw::color::Rgb;
use kas::event::{VirtualKeyCode, VirtualKeyCodes};
use kas::prelude::*;
use kas::theme::TextClass;
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
        keys1: VirtualKeyCodes,
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
                keys1: Default::default(),
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

        /// Add accelerator keys (chain style)
        #[must_use]
        pub fn with_keys(mut self, keys: &[VirtualKeyCode]) -> Self {
            self.keys1.clear();
            self.keys1.extend_from_slice(keys);
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
            cx.add_accel_keys(self.id_ref(), &self.keys1);
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
}

impl_scope! {
    /// A push-button with a text label
    ///
    /// This is a specialised variant of [`Button`] supporting key shortcuts from an
    /// [`AccelString`] label and using a custom text class (and thus theme colour).
    ///
    /// Default alignment of content is centered.
    #[widget {
        layout = button!(self.label, color = self.color);
        navigable = true;
        hover_highlight = true;
    }]
    pub struct TextButton {
        core: widget_core!(),
        keys1: VirtualKeyCodes,
        #[widget]
        label: AccelLabel,
        color: Option<Rgb>,
        on_press: Option<Box<dyn Fn(&mut EventCx)>>,
    }

    impl Self {
        /// Construct a button with given `label`
        #[inline]
        pub fn new(label: impl Into<AccelString>) -> Self {
            TextButton {
                core: Default::default(),
                keys1: Default::default(),
                label: AccelLabel::new(label).with_class(TextClass::Button),
                color: None,
                on_press: None,
            }
        }

        /// Call the handler `f` on press / activation
        #[inline]
        #[must_use]
        pub fn with(mut self, f: impl Fn(&mut EventCx) + 'static) -> Self {
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
            self.with(move |cx| cx.push(msg.clone()))
        }

        /// Construct a button with a given `label` and payload `msg`
        ///
        /// When the button is activated, a clone of `msg` is sent to the
        /// parent widget. The parent (or an ancestor) should handle this using
        /// [`Events::handle_messages`].
        #[inline]
        pub fn new_msg<M: Clone + Debug + 'static>(label: impl Into<AccelString>, msg: M) -> Self {
            Self::new(label).with_msg(msg)
        }

        /// Add accelerator keys (chain style)
        ///
        /// These keys are added to those inferred from the label via `&` marks.
        #[must_use]
        pub fn with_keys(mut self, keys: &[VirtualKeyCode]) -> Self {
            self.keys1.clear();
            self.keys1.extend_from_slice(keys);
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

    impl HasStr for Self {
        fn get_str(&self) -> &str {
            self.label.get_str()
        }
    }

    impl SetAccel for Self {
        #[inline]
        fn set_accel_string(&mut self, string: AccelString) -> Action {
            self.label.set_accel_string(string)
        }
    }

    impl Events for Self {
        type Data = ();

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.add_accel_keys(self.id_ref(), &self.keys1);
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &(), event: Event) -> Response {
            event.on_activate(cx, self.id(), |cx| {
                if let Some(f) = self.on_press.as_ref() {
                    f(cx);
                }
                Response::Used
            })
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &()) {
            if let Some(kas::message::Activate) = cx.try_pop() {
                if let Some(f) = self.on_press.as_ref() {
                    f(cx);
                }
            }
        }
    }

    impl From<&str> for TextButton {
        #[inline]
        fn from(s: &str) -> Self {
            Self::new(s)
        }
    }
    impl From<String> for TextButton {
        #[inline]
        fn from(s: String) -> Self {
            Self::new(s)
        }
    }
}
