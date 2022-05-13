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
use std::rc::Rc;

impl_scope! {
    /// A push-button with a generic label
    ///
    /// Default alignment of content is centered.
    #[autoimpl(Debug ignore self.on_push)]
    #[autoimpl(class_traits using self.inner where W: trait)]
    #[derive(Clone)]
    #[widget {
        layout = button(self.color): self.inner;
        key_nav = true;
        hover_highlight = true;
    }]
    pub struct Button<W: Widget> {
        core: widget_core!(),
        keys1: VirtualKeyCodes,
        color: Option<Rgb>,
        #[widget]
        pub inner: W,
        on_push: Option<Rc<dyn Fn(&mut EventMgr)>>,
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
                on_push: None,
            }
        }

        /// Set event handler `f`
        ///
        /// On activation (through user input events or [`Event::Activate`]) the
        /// closure `f` is called.
        #[inline]
        #[must_use]
        pub fn on_push<F>(self, f: F) -> Button<W>
        where
            F: Fn(&mut EventMgr) + 'static,
        {
            Button {
                core: self.core,
                keys1: self.keys1,
                color: self.color,
                inner: self.inner,
                on_push: Some(Rc::new(f)),
            }
        }
    }

    impl Self {
        /// Construct a button with a given `inner` widget and event handler `f`
        ///
        /// On activation (through user input events or [`Event::Activate`]) the
        /// closure `f` is called.
        #[inline]
        pub fn new_on<F>(inner: W, f: F) -> Self
        where
            F: Fn(&mut EventMgr) + 'static,
        {
            Button::new(inner).on_push(f)
        }

        /// Construct a button with a given `inner` and payload `msg`
        ///
        /// On activation (through user input events or [`Event::Activate`]) a clone
        /// of `msg` is returned to the parent widget. Click actions must be
        /// implemented through a handler on the parent widget (or other ancestor).
        #[inline]
        pub fn new_msg<M: Clone + Debug + 'static>(inner: W, msg: M) -> Self {
            Self::new_on(inner, move |mgr| mgr.push_msg(msg.clone()))
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

    impl Widget for Self {
        fn configure(&mut self, mgr: &mut SetRectMgr) {
            mgr.add_accel_keys(self.id_ref(), &self.keys1);
        }

        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            event.on_activate(mgr, self.id(), |mgr| {
                if let Some(f) = self.on_push.as_ref() {
                    f(mgr);
                }
                Response::Used
            })
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
    #[autoimpl(Debug ignore self.on_push)]
    #[derive(Clone)]
    #[widget {
        layout = button(self.color): self.label;
        key_nav = true;
        hover_highlight = true;
    }]
    pub struct TextButton {
        core: widget_core!(),
        keys1: VirtualKeyCodes,
        #[widget]
        label: AccelLabel,
        color: Option<Rgb>,
        on_push: Option<Rc<dyn Fn(&mut EventMgr)>>,
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
                on_push: None,
            }
        }

        /// Set event handler `f`
        ///
        /// On activation (through user input events or [`Event::Activate`]) the
        /// closure `f` is called.
        #[inline]
        #[must_use]
        pub fn on_push<F>(self, f: F) -> TextButton
        where
            F: Fn(&mut EventMgr) + 'static,
        {
            TextButton {
                core: self.core,
                keys1: self.keys1,
                color: self.color,
                label: self.label,
                on_push: Some(Rc::new(f)),
            }
        }

        /// Construct a button with a given `label` and event handler `f`
        ///
        /// On activation (through user input events or [`Event::Activate`]) the
        /// closure `f` is called.
        #[inline]
        pub fn new_on<S: Into<AccelString>, F>(label: S, f: F) -> Self
        where
            F: Fn(&mut EventMgr) + 'static,
        {
            TextButton::new(label).on_push(f)
        }

        /// Construct a button with a given `label` and payload `msg`
        ///
        /// On activation (through user input events or [`Event::Activate`]) a clone
        /// of `msg` is returned to the parent widget. Click actions must be
        /// implemented through a handler on the parent widget (or other ancestor).
        #[inline]
        pub fn new_msg<S: Into<AccelString>, M: Clone + Debug + 'static>(label: S, msg: M) -> Self {
            Self::new_on(label, move |mgr| mgr.push_msg(msg.clone()))
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
        fn set_accel_string(&mut self, string: AccelString) -> TkAction {
            self.label.set_accel_string(string)
        }
    }

    impl Widget for Self {
        fn configure(&mut self, mgr: &mut SetRectMgr) {
            mgr.add_accel_keys(self.id_ref(), &self.keys1);
            mgr.add_accel_keys(self.id_ref(), self.label.keys());
        }

        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            event.on_activate(mgr, self.id(), |mgr| {
                if let Some(f) = self.on_push.as_ref() {
                    f(mgr);
                }
                Response::Used
            })
        }
    }
}
