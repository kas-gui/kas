// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Menu Entries

use std::fmt::{self, Debug};

use super::Menu;
use kas::draw::TextClass;
use kas::prelude::*;
use kas::widget::{AccelLabel, CheckBoxBare};

/// A standard menu entry
#[derive(Clone, Debug, Default, Widget)]
#[widget(config=noauto)]
#[handler(handle=noauto)]
pub struct MenuEntry<M: Clone + Debug + 'static> {
    #[widget_core]
    core: kas::CoreData,
    label: Text<AccelString>,
    label_off: Offset,
    frame_size: Size,
    msg: M,
}

impl<M: Clone + Debug + 'static> WidgetConfig for MenuEntry<M> {
    fn configure(&mut self, mgr: &mut Manager) {
        mgr.add_accel_keys(self.id(), self.label.text().keys());
    }

    fn key_nav(&self) -> bool {
        true
    }
    fn hover_highlight(&self) -> bool {
        true
    }
}

impl<M: Clone + Debug + 'static> Layout for MenuEntry<M> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let frame_rules = size_handle.menu_frame(axis.is_vertical());
        let text_rules = size_handle.text_bound(&mut self.label, TextClass::MenuLabel, axis);
        let (rules, offset, size) = frame_rules.surround(text_rules);
        self.label_off.set_component(axis, offset);
        self.frame_size.set_component(axis, size);
        rules
    }

    fn set_rect(&mut self, _: &mut Manager, rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        let size = rect.size - self.frame_size;
        self.label.update_env(|env| {
            env.set_bounds(size.into());
            env.set_align(align.unwrap_or(Align::Default, Align::Centre));
        });
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
        draw_handle.menu_entry(self.core.rect, self.input_state(mgr, disabled));
        let pos = self.core.rect.pos + self.label_off;
        draw_handle.text_accel(
            pos,
            &self.label,
            mgr.show_accel_labels(),
            TextClass::MenuLabel,
        );
    }
}

impl<M: Clone + Debug + 'static> MenuEntry<M> {
    /// Construct a menu item with a given `label` and `msg`
    ///
    /// The message `msg` is emitted on activation. Any
    /// type supporting `Clone` is valid, though it is recommended to use a
    /// simple `Copy` type (e.g. an enum).
    pub fn new<S: Into<AccelString>>(label: S, msg: M) -> Self {
        MenuEntry {
            core: Default::default(),
            label: Text::new_single(label.into()),
            label_off: Offset::ZERO,
            frame_size: Size::ZERO,
            msg,
        }
    }

    /// Replace the message value
    pub fn set_msg(&mut self, msg: M) {
        self.msg = msg;
    }
}

impl<M: Clone + Debug + 'static> HasStr for MenuEntry<M> {
    fn get_str(&self) -> &str {
        self.label.as_str()
    }
}

impl<M: Clone + Debug + 'static> SetAccel for MenuEntry<M> {
    fn set_accel_string(&mut self, string: AccelString) -> TkAction {
        let mut action = TkAction::empty();
        if self.label.text().keys() != string.keys() {
            action |= TkAction::RECONFIGURE;
        }
        let avail = self.core.rect.size.clamped_sub(self.frame_size);
        action | kas::text::util::set_text_and_prepare(&mut self.label, string, avail)
    }
}

impl<M: Clone + Debug + 'static> Handler for MenuEntry<M> {
    type Msg = M;

    fn handle(&mut self, _: &mut Manager, event: Event) -> Response<M> {
        match event {
            Event::Activate => self.msg.clone().into(),
            _ => Response::Unhandled,
        }
    }
}

impl<M: Clone + Debug> Menu for MenuEntry<M> {}

/// A menu entry which can be toggled
#[derive(Clone, Default, Widget)]
#[handler(msg = M, generics = <> where M: From<VoidMsg>)]
#[widget(config=noauto)]
#[layout(row, area=checkbox, draw=draw)]
pub struct MenuToggle<M: 'static> {
    #[widget_core]
    core: CoreData,
    #[layout_data]
    layout_data: <Self as kas::LayoutData>::Data,
    #[widget]
    checkbox: CheckBoxBare<M>,
    // TODO: label should use TextClass::MenuLabel
    #[widget]
    label: AccelLabel,
}

impl<M: 'static> Debug for MenuToggle<M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MenuToggle")
            .field("core", &self.core)
            .field("layout_data", &self.layout_data)
            .field("checkbox", &self.checkbox)
            .field("label", &self.label)
            .finish()
    }
}

impl MenuToggle<VoidMsg> {
    /// Construct a toggleable menu entry with a given `label`
    #[inline]
    pub fn new<T: Into<AccelString>>(label: T) -> Self {
        MenuToggle {
            core: Default::default(),
            layout_data: Default::default(),
            checkbox: CheckBoxBare::new(),
            label: AccelLabel::new(label.into()),
        }
    }

    /// Set event handler `f`
    ///
    /// On toggle (through user input events or [`Event::Activate`]) the
    /// closure `f` is called. The message generated by `f`, if any,
    /// is returned for handling through the parent widget (or other ancestor).
    #[inline]
    pub fn on_toggle<M, F>(self, f: F) -> MenuToggle<M>
    where
        F: Fn(&mut Manager, bool) -> Option<M> + 'static,
    {
        MenuToggle {
            core: self.core,
            layout_data: self.layout_data,
            checkbox: self.checkbox.on_toggle(f),
            label: self.label,
        }
    }
}

impl<M: 'static> MenuToggle<M> {
    /// Construct a toggleable menu entry with a given `label` and event handler `f`
    ///
    /// On toggle (through user input events or [`Event::Activate`]) the
    /// closure `f` is called. The message generated by `f`, if any,
    /// is returned for handling through the parent widget (or other ancestor).
    #[inline]
    pub fn new_on<T: Into<AccelString>, F>(label: T, f: F) -> Self
    where
        F: Fn(&mut Manager, bool) -> Option<M> + 'static,
    {
        MenuToggle::new(label).on_toggle(f)
    }

    /// Set the initial state of the checkbox.
    #[inline]
    pub fn with_state(mut self, state: bool) -> Self {
        self.checkbox = self.checkbox.with_state(state);
        self
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
        let state = self.checkbox.input_state(mgr, disabled);
        draw_handle.menu_entry(self.core.rect, state);
        self.checkbox.draw(draw_handle, mgr, state.disabled);
        self.label.draw(draw_handle, mgr, state.disabled);
    }
}

impl<M: 'static> WidgetConfig for MenuToggle<M> {
    fn configure(&mut self, mgr: &mut Manager) {
        mgr.add_accel_keys(self.checkbox.id(), self.label.keys());
    }
}

impl<M: From<VoidMsg>> Menu for MenuToggle<M> {}

impl<M: 'static> HasBool for MenuToggle<M> {
    #[inline]
    fn get_bool(&self) -> bool {
        self.checkbox.get_bool()
    }

    #[inline]
    fn set_bool(&mut self, state: bool) -> TkAction {
        self.checkbox.set_bool(state)
    }
}
