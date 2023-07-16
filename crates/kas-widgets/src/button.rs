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
        on_press: Option<Box<dyn Fn(&mut EventMgr, &W::Data)>>,
    }

    impl<W: Widget> Button<W> {
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

        /// Set event handler `f`
        ///
        /// This closure is called when the button is activated.
        #[inline]
        #[must_use]
        pub fn on_press<F>(self, f: F) -> Button<W>
        where
            F: Fn(&mut EventMgr, &W::Data) + 'static,
        {
            Button {
                core: self.core,
                keys1: self.keys1,
                color: self.color,
                inner: self.inner,
                on_press: Some(Box::new(f)),
            }
        }
    }

    impl Self {
        /// Construct a button with a given `inner` widget and event handler `f`
        ///
        /// This closure is called when the button is activated.
        #[inline]
        pub fn new_on<F>(inner: W, f: F) -> Self
        where
            F: Fn(&mut EventMgr, &W::Data) + 'static,
        {
            Button::new(inner).on_press(f)
        }

        /// Construct a button with a given `inner` and payload `msg`
        ///
        /// When the button is activated, a clone of `msg` is sent to the
        /// parent widget. The parent (or an ancestor) should handle this using
        /// [`Events::handle_messages`].
        #[inline]
        pub fn new_msg<M: Clone + Debug + 'static>(inner: W, msg: M) -> Self {
            Self::new_on(inner, move |mgr, _| mgr.push(msg.clone()))
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
        fn configure(&mut self, _: &W::Data, mgr: &mut ConfigMgr) {
            mgr.add_accel_keys(self.id_ref(), &self.keys1);
        }

        fn handle_event(&mut self, data: &W::Data, mgr: &mut EventMgr, event: Event) -> Response {
            event.on_activate(mgr, self.id(), |mgr| {
                if let Some(f) = self.on_press.as_ref() {
                    f(mgr, data);
                }
                Response::Used
            })
        }

        fn handle_messages(&mut self, data: &W::Data, mgr: &mut EventMgr) {
            if let Some(kas::message::Activate) = mgr.try_pop() {
                if let Some(f) = self.on_press.as_ref() {
                    f(mgr, data);
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
        on_press: Option<Box<dyn Fn(&mut EventMgr)>>,
    }

    impl Self {
        /// Construct a button with given `label`
        #[inline]
        pub fn new<S: Into<AccelString>>(label: S) -> Self {
            TextButton {
                core: Default::default(),
                keys1: Default::default(),
                label: AccelLabel::new(label).with_class(TextClass::Button),
                color: None,
                on_press: None,
            }
        }

        /// Set event handler `f`
        ///
        /// This closure is called when the button is activated.
        #[inline]
        #[must_use]
        pub fn on_press<F>(self, f: F) -> TextButton
        where
            F: Fn(&mut EventMgr) + 'static,
        {
            TextButton {
                core: self.core,
                keys1: self.keys1,
                color: self.color,
                label: self.label,
                on_press: Some(Box::new(f)),
            }
        }

        /// Construct a button with a given `label` and event handler `f`
        ///
        /// This closure is called when the button is activated.
        #[inline]
        pub fn new_on<S: Into<AccelString>, F>(label: S, f: F) -> Self
        where
            F: Fn(&mut EventMgr) + 'static,
        {
            TextButton::new(label).on_press(f)
        }

        /// Construct a button with a given `label` and payload `msg`
        ///
        /// When the button is activated, a clone of `msg` is sent to the
        /// parent widget. The parent (or an ancestor) should handle this using
        /// [`Events::handle_messages`].
        #[inline]
        pub fn new_msg<S: Into<AccelString>, M: Clone + Debug + 'static>(label: S, msg: M) -> Self {
            Self::new_on(label, move |mgr| mgr.push(msg.clone()))
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

        fn configure(&mut self, _: &(), mgr: &mut ConfigMgr) {
            mgr.add_accel_keys(self.id_ref(), &self.keys1);
        }

        fn handle_event(&mut self, _: &(), mgr: &mut EventMgr, event: Event) -> Response {
            event.on_activate(mgr, self.id(), |mgr| {
                if let Some(f) = self.on_press.as_ref() {
                    f(mgr);
                }
                Response::Used
            })
        }

        fn handle_messages(&mut self, _: &(), mgr: &mut EventMgr) {
            if let Some(kas::message::Activate) = mgr.try_pop() {
                if let Some(f) = self.on_press.as_ref() {
                    f(mgr);
                }
            }
        }
    }
}
