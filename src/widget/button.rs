// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Push-buttons

use std::fmt::{self, Debug};
use std::rc::Rc;

use kas::draw::TextClass;
use kas::event::{self, VirtualKeyCode, VirtualKeyCodes};
use kas::prelude::*;

/// A push-button with a generic label
#[derive(Clone, Widget)]
#[handler(noauto)]
#[widget(config=noauto)]
#[widget_derive(class_traits)]
pub struct Button<L: Widget<Msg = VoidMsg>, M: 'static> {
    #[widget_core]
    core: kas::CoreData,
    keys1: VirtualKeyCodes,
    frame_size: Size,
    frame_offset: Offset,
    #[widget_derive]
    #[widget]
    pub label: L,
    on_push: Option<Rc<dyn Fn(&mut Manager) -> Option<M>>>,
}

impl<L: Widget<Msg = VoidMsg>, M: 'static> Debug for Button<L, M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Button")
            .field("core", &self.core)
            .field("keys1", &self.keys1)
            .field("frame_size", &self.frame_size)
            .field("frame_offset", &self.frame_offset)
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

impl<L: Widget<Msg = VoidMsg>, M: 'static> WidgetConfig for Button<L, M> {
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

impl<L: Widget<Msg = VoidMsg>, M: 'static> Layout for Button<L, M> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let frame_rules = size_handle.button_surround(axis.is_vertical());
        let content_rules = self.label.size_rules(size_handle, axis);

        let (rules, offset, size) = frame_rules.surround_as_margin(content_rules);
        self.frame_size.set_component(axis, size);
        self.frame_offset.set_component(axis, offset);
        rules
    }

    fn set_rect(&mut self, mgr: &mut Manager, mut rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        rect.pos += self.frame_offset;
        rect.size -= self.frame_size;
        self.label.set_rect(mgr, rect, align);
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        draw_handle.button(self.core.rect, self.input_state(mgr, disabled));
        self.label.draw(draw_handle, mgr, disabled);
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
            label: self.label,
            on_push: Some(Rc::new(f)),
        }
    }
}

impl<L: Widget<Msg = VoidMsg>, M: 'static> Button<L, M> {
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
}

impl<L: Widget<Msg = VoidMsg>, M: 'static> Handler for Button<L, M> {
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

impl<L: Widget<Msg = VoidMsg>, M: 'static> SendEvent for Button<L, M> {
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<M> {
        if id < self.label.id() {
            self.label.send(mgr, id, event).void_into()
        } else {
            debug_assert_eq!(id, self.id());
            Manager::handle_generic(self, mgr, event)
        }
    }
}

/// A push-button with a text label
///
/// This is a specialised variant of [`Button`] supporting key shortcuts from an
/// [`AccelString`] label and using a custom text class (and thus theme colour).
#[derive(Clone, Widget)]
#[handler(handle=noauto)]
#[widget(config=noauto)]
pub struct TextButton<M: 'static> {
    #[widget_core]
    core: kas::CoreData,
    keys1: VirtualKeyCodes,
    frame_size: Size,
    frame_offset: Offset,
    label: Text<AccelString>,
    on_push: Option<Rc<dyn Fn(&mut Manager) -> Option<M>>>,
}

impl<M: 'static> Debug for TextButton<M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TextButton")
            .field("core", &self.core)
            .field("keys1", &self.keys1)
            .field("frame_size", &self.frame_size)
            .field("frame_offset", &self.frame_offset)
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

impl<M: 'static> WidgetConfig for TextButton<M> {
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

impl<M: 'static> Layout for TextButton<M> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let frame_rules = size_handle.button_surround(axis.is_vertical());
        let content_rules = size_handle.text_bound(&mut self.label, TextClass::Button, axis);

        let (rules, offset, size) = frame_rules.surround_as_margin(content_rules);
        self.frame_size.set_component(axis, size);
        self.frame_offset.set_component(axis, offset);
        rules
    }

    fn set_rect(&mut self, _: &mut Manager, rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        let size = rect.size - self.frame_size;
        self.label.update_env(|env| {
            env.set_bounds(size.into());
            env.set_align(align.unwrap_or(Align::Centre, Align::Centre));
        });
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        draw_handle.button(self.core.rect, self.input_state(mgr, disabled));
        let pos = self.core.rect.pos + self.frame_offset;
        let state = mgr.show_accel_labels();
        draw_handle.text_accel(pos, &self.label, state, TextClass::Button);
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
            label: self.label,
            on_push: Some(Rc::new(f)),
        }
    }
}

impl<M: 'static> TextButton<M> {
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
}

impl<M: 'static> HasStr for TextButton<M> {
    fn get_str(&self) -> &str {
        self.label.as_str()
    }
}

impl<M: 'static> SetAccel for TextButton<M> {
    fn set_accel_string(&mut self, string: AccelString) -> TkAction {
        let mut action = TkAction::empty();
        if self.label.text().keys() != string.keys() {
            action |= TkAction::RECONFIGURE;
        }
        let avail = self.core.rect.size.clamped_sub(self.frame_size);
        action | kas::text::util::set_text_and_prepare(&mut self.label, string, avail)
    }
}

impl<M: 'static> event::Handler for TextButton<M> {
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
