// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Push-buttons

use kas::draw::color::Rgb;
use kas::event::{self, VirtualKeyCode, VirtualKeyCodes};
use kas::layout;
use kas::prelude::*;
use kas::theme::TextClass;
use std::rc::Rc;

widget! {
    /// A push-button with a generic label
    ///
    /// Default alignment is centred. Content (label) alignment is derived from the
    /// button alignment.
    #[autoimpl(Debug skip on_push)]
    #[autoimpl(class_traits where W: trait on inner)]
    #[derive(Clone)]
    pub struct Button<W: Widget<Msg = VoidMsg>, M: 'static> {
        #[widget_core]
        core: kas::CoreData,
        keys1: VirtualKeyCodes,
        layout_frame: layout::FrameStorage,
        color: Option<Rgb>,
        #[widget]
        pub inner: W,
        on_push: Option<Rc<dyn Fn(&mut Manager) -> Option<M>>>,
    }

    impl WidgetConfig for Self {
        fn configure(&mut self, mgr: &mut Manager) {
            mgr.add_accel_keys(self.id(), &self.keys1);
        }

        fn key_nav(&self) -> bool {
            true
        }
        fn hover_highlight(&self) -> bool {
            true
        }
    }

    impl Layout for Self {
        fn layout(&mut self) -> layout::Layout<'_> {
            let inner = layout::Layout::single(&mut self.inner);
            layout::Layout::button(&mut self.layout_frame, inner, self.color)
        }
    }

    impl<W: Widget<Msg = VoidMsg>> Button<W, VoidMsg> {
        /// Construct a button with given `inner` widget
        #[inline]
        pub fn new(inner: W) -> Self {
            Button {
                core: Default::default(),
                keys1: Default::default(),
                layout_frame: Default::default(),
                color: None,
                inner,
                on_push: None,
            }
        }

        /// Set event handler `f`
        ///
        /// On activation (through user input events or [`Event::Activate`]) the
        /// closure `f` is called. The result of `f` is converted to
        /// [`Response::Msg`] or [`Response::Used`] and returned to the parent.
        #[inline]
        #[must_use]
        pub fn on_push<M, F>(self, f: F) -> Button<W, M>
        where
            F: Fn(&mut Manager) -> Option<M> + 'static,
        {
            Button {
                core: self.core,
                keys1: self.keys1,
                layout_frame: self.layout_frame,
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
        /// closure `f` is called. The result of `f` is converted to
        /// [`Response::Msg`] or [`Response::Used`] and returned to the parent.
        #[inline]
        pub fn new_on<F>(inner: W, f: F) -> Self
        where
            F: Fn(&mut Manager) -> Option<M> + 'static,
        {
            Button::new(inner).on_push(f)
        }

        /// Construct a button with a given `inner` and payload `msg`
        ///
        /// On activation (through user input events or [`Event::Activate`]) a clone
        /// of `msg` is returned to the parent widget. Click actions must be
        /// implemented through a handler on the parent widget (or other ancestor).
        #[inline]
        pub fn new_msg(inner: W, msg: M) -> Self
        where
            M: Clone,
        {
            Self::new_on(inner, move |_| Some(msg.clone()))
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

    impl Handler for Self {
        type Msg = M;

        #[inline]
        fn activation_via_press(&self) -> bool {
            true
        }

        fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<M> {
            match event {
                Event::Activate => Response::used_or_msg(self.on_push.as_ref().and_then(|f| f(mgr))),
                _ => Response::Unused,
            }
        }
    }

    impl SendEvent for Self {
        fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<M> {
            if self.is_disabled() {
                return Response::Unused;
            }
            if self.eq_id(&id) {
                Manager::handle_generic(self, mgr, event)
            } else {
                debug_assert!(self.inner.id().is_ancestor_of(&id));
                self.inner.send(mgr, id, event).void_into()
            }
        }
    }
}

widget! {
    /// A push-button with a text label
    ///
    /// This is a specialised variant of [`Button`] supporting key shortcuts from an
    /// [`AccelString`] label and using a custom text class (and thus theme colour).
    ///
    /// Default alignment of the button is to stretch horizontally and centre
    /// vertically. The text label is always centred (irrespective of alignment
    /// parameters).
    #[autoimpl(Debug skip on_push)]
    #[derive(Clone)]
    pub struct TextButton<M: 'static> {
        #[widget_core]
        core: kas::CoreData,
        keys1: VirtualKeyCodes,
        layout_frame: layout::FrameStorage,
        layout_text: layout::TextStorage,
        color: Option<Rgb>,
        label: Text<AccelString>,
        on_push: Option<Rc<dyn Fn(&mut Manager) -> Option<M>>>,
    }

    impl WidgetConfig for Self {
        fn configure(&mut self, mgr: &mut Manager) {
            mgr.add_accel_keys(self.id(), &self.keys1);
            mgr.add_accel_keys(self.id(), self.label.text().keys());
        }

        fn key_nav(&self) -> bool {
            true
        }
        fn hover_highlight(&self) -> bool {
            true
        }
    }

    impl Layout for Self {
        fn layout(&mut self) -> layout::Layout<'_> {
            let inner = layout::Layout::text(&mut self.layout_text, &mut self.label, TextClass::Button);
            layout::Layout::button(&mut self.layout_frame, inner, self.color)
        }
    }

    impl TextButton<VoidMsg> {
        /// Construct a button with given `label`
        #[inline]
        pub fn new<S: Into<AccelString>>(label: S) -> Self {
            let label = label.into();
            let text = Text::new_single(label);
            TextButton {
                core: Default::default(),
                keys1: Default::default(),
                layout_frame: Default::default(),
                layout_text: Default::default(),
                color: None,
                label: text,
                on_push: None,
            }
        }

        /// Set event handler `f`
        ///
        /// On activation (through user input events or [`Event::Activate`]) the
        /// closure `f` is called. The result of `f` is converted to
        /// [`Response::Msg`] or [`Response::Used`] and returned to the parent.
        #[inline]
        #[must_use]
        pub fn on_push<M, F>(self, f: F) -> TextButton<M>
        where
            F: Fn(&mut Manager) -> Option<M> + 'static,
        {
            TextButton {
                core: self.core,
                keys1: self.keys1,
                layout_frame: self.layout_frame,
                layout_text: self.layout_text,
                color: self.color,
                label: self.label,
                on_push: Some(Rc::new(f)),
            }
        }
    }

    impl Self {
        /// Construct a button with a given `label` and event handler `f`
        ///
        /// On activation (through user input events or [`Event::Activate`]) the
        /// closure `f` is called. The result of `f` is converted to
        /// [`Response::Msg`] or [`Response::Used`] and returned to the parent.
        #[inline]
        pub fn new_on<S: Into<AccelString>, F>(label: S, f: F) -> Self
        where
            F: Fn(&mut Manager) -> Option<M> + 'static,
        {
            TextButton::new(label).on_push(f)
        }

        /// Construct a button with a given `label` and payload `msg`
        ///
        /// On activation (through user input events or [`Event::Activate`]) a clone
        /// of `msg` is returned to the parent widget. Click actions must be
        /// implemented through a handler on the parent widget (or other ancestor).
        #[inline]
        pub fn new_msg<S: Into<AccelString>>(label: S, msg: M) -> Self
        where
            M: Clone,
        {
            Self::new_on(label, move |_| Some(msg.clone()))
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
            self.label.as_str()
        }
    }

    impl SetAccel for Self {
        fn set_accel_string(&mut self, string: AccelString) -> TkAction {
            let mut action = TkAction::empty();
            if self.label.text().keys() != string.keys() {
                action |= TkAction::RECONFIGURE;
            }
            let avail = self.core.rect.size.clamped_sub(self.layout_frame.size);
            action | kas::text::util::set_text_and_prepare(&mut self.label, string, avail)
        }
    }

    impl event::Handler for Self {
        type Msg = M;

        #[inline]
        fn activation_via_press(&self) -> bool {
            true
        }

        fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<M> {
            match event {
                Event::Activate => Response::used_or_msg(self.on_push.as_ref().and_then(|f| f(mgr))),
                _ => Response::Unused,
            }
        }
    }
}
