// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Push-buttons

use kas::draw::{color::Rgb, TextClass};
use kas::event::{self, VirtualKeyCode, VirtualKeyCodes};
use kas::prelude::*;
use std::rc::Rc;

widget! {
    /// A push-button with a generic label
    ///
    /// Default alignment is centred. Content (label) alignment is derived from the
    /// button alignment.
    #[autoimpl(Debug skip on_push)]
    #[autoimpl(class_traits where L: trait on label)]
    #[derive(Clone)]
    pub struct Button<L: Widget<Msg = VoidMsg>, M: 'static> {
        #[widget_core]
        core: kas::CoreData,
        keys1: VirtualKeyCodes,
        frame_size: Size,
        frame_offset: Offset,
        ideal_size: Size,
        color: Option<Rgb>,
        #[widget]
        pub label: L,
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
        fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
            let frame_rules = size_handle.button_surround(axis.is_vertical());
            let content_rules = self.label.size_rules(size_handle, axis);

            let (rules, offset, size) = frame_rules.surround_as_margin(content_rules);
            self.frame_size.set_component(axis, size);
            self.frame_offset.set_component(axis, offset);
            self.ideal_size.set_component(axis, rules.ideal_size());
            rules
        }

        fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
            let mut rect = align
                .complete(Align::Centre, Align::Centre)
                .aligned_rect(self.ideal_size, rect);
            self.core.rect = rect;
            rect.pos += self.frame_offset;
            rect.size -= self.frame_size;
            self.label.set_rect(mgr, rect, align);
        }

        fn draw(&self, theme: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
            theme.button(self.core.rect, self.color, self.input_state(mgr, disabled));
            self.label.draw(theme, mgr, disabled);
        }
    }

    impl<L: Widget<Msg = VoidMsg>> Button<L, VoidMsg> {
        /// Construct a button with given `label`
        #[inline]
        pub fn new(label: L) -> Self {
            Button {
                core: Default::default(),
                keys1: Default::default(),
                frame_size: Default::default(),
                frame_offset: Default::default(),
                ideal_size: Default::default(),
                color: None,
                label,
                on_push: None,
            }
        }

        /// Set event handler `f`
        ///
        /// On activation (through user input events or [`Event::Activate`]) the
        /// closure `f` is called. The result of `f` is converted to
        /// [`Response::Msg`] or [`Response::None`] and returned to the parent.
        #[inline]
        pub fn on_push<M, F>(self, f: F) -> Button<L, M>
        where
            F: Fn(&mut Manager) -> Option<M> + 'static,
        {
            Button {
                core: self.core,
                keys1: self.keys1,
                frame_size: self.frame_size,
                frame_offset: self.frame_offset,
                ideal_size: self.ideal_size,
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
        /// [`Response::Msg`] or [`Response::None`] and returned to the parent.
        #[inline]
        pub fn new_on<F>(label: L, f: F) -> Self
        where
            F: Fn(&mut Manager) -> Option<M> + 'static,
        {
            Button::new(label).on_push(f)
        }

        /// Construct a button with a given `label` and payload `msg`
        ///
        /// On activation (through user input events or [`Event::Activate`]) a clone
        /// of `msg` is returned to the parent widget. Click actions must be
        /// implemented through a handler on the parent widget (or other ancestor).
        #[inline]
        pub fn new_msg(label: L, msg: M) -> Self
        where
            M: Clone,
        {
            Self::new_on(label, move |_| Some(msg.clone()))
        }

        /// Add accelerator keys (chain style)
        ///
        /// These keys are added to those inferred from the label via `&` marks.
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
                Event::Activate => Response::none_or_msg(self.on_push.as_ref().and_then(|f| f(mgr))),
                _ => Response::Unhandled,
            }
        }
    }

    impl SendEvent for Self {
        fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<M> {
            if id < self.label.id() {
                self.label.send(mgr, id, event).void_into()
            } else {
                debug_assert_eq!(id, self.id());
                Manager::handle_generic(self, mgr, event)
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
        frame_size: Size,
        frame_offset: Offset,
        ideal_size: Size,
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
        fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
            let frame_rules = size_handle.button_surround(axis.is_vertical());
            let content_rules = size_handle.text_bound(&mut self.label, TextClass::Button, axis);

            let (rules, offset, size) = frame_rules.surround_as_margin(content_rules);
            self.frame_size.set_component(axis, size);
            self.frame_offset.set_component(axis, offset);
            self.ideal_size.set_component(axis, rules.ideal_size());
            rules
        }

        fn set_rect(&mut self, _: &mut Manager, rect: Rect, align: AlignHints) {
            let rect = align
                .complete(Align::Stretch, Align::Centre)
                .aligned_rect(self.ideal_size, rect);
            self.core.rect = rect;
            let size = rect.size - self.frame_size;
            self.label.update_env(|env| {
                env.set_bounds(size.into());
                env.set_align((Align::Centre, Align::Centre));
            });
        }

        fn draw(&self, theme: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
            theme.button(self.core.rect, self.color, self.input_state(mgr, disabled));
            let pos = self.core.rect.pos + self.frame_offset;
            let accel = mgr.show_accel_labels();
            let state = self.input_state(mgr, disabled);
            theme.text_accel(pos, &self.label, accel, TextClass::Button, state);
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
                frame_size: Default::default(),
                frame_offset: Default::default(),
                ideal_size: Default::default(),
                color: None,
                label: text,
                on_push: None,
            }
        }

        /// Set event handler `f`
        ///
        /// On activation (through user input events or [`Event::Activate`]) the
        /// closure `f` is called. The result of `f` is converted to
        /// [`Response::Msg`] or [`Response::None`] and returned to the parent.
        #[inline]
        pub fn on_push<M, F>(self, f: F) -> TextButton<M>
        where
            F: Fn(&mut Manager) -> Option<M> + 'static,
        {
            TextButton {
                core: self.core,
                keys1: self.keys1,
                frame_size: self.frame_size,
                frame_offset: self.frame_offset,
                ideal_size: self.ideal_size,
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
        /// [`Response::Msg`] or [`Response::None`] and returned to the parent.
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
            let avail = self.core.rect.size.clamped_sub(self.frame_size);
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
                Event::Activate => Response::none_or_msg(self.on_push.as_ref().and_then(|f| f(mgr))),
                _ => Response::Unhandled,
            }
        }
    }
}
