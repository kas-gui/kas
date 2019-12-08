// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Control widgets accept simple input

use std::any::TypeId;
use std::fmt::{self, Debug};

use crate::class::{HasBool, HasText};
use crate::event::{self, err_unhandled, Action, EmptyMsg, Handler, VirtualKeyCode};
use crate::layout::{AxisInfo, SizeRules};
use crate::macros::Widget;
use crate::theme::{Align, DrawHandle, SizeHandle, TextClass, TextProperties};
use crate::{CoreData, TkWindow, Widget, WidgetCore};
use kas::geom::{Coord, Rect};

/// A checkable box with optional label
#[widget]
#[derive(Clone, Default, Widget)]
pub struct CheckBox<OT: 'static> {
    #[core]
    core: CoreData,
    box_pos: Coord,
    text_pos_x: i32,
    label: String,
    state: bool,
    on_toggle: OT,
}

impl<H> Debug for CheckBox<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "CheckBox {{ core: {:?}, state: {:?}, label: {:?}, ... }}",
            self.core, self.state, self.label
        )
    }
}

impl<OT: 'static> Widget for CheckBox<OT> {
    fn allow_focus(&self) -> bool {
        true
    }

    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let mut r = SizeRules::fixed(axis.extract_size(size_handle.checkbox()));
        if !self.label.is_empty() {
            if !axis.vertical() {
                r += SizeRules::fixed(size_handle.outer_margin().0);
            }
            r += size_handle.text_bound(&self.label, TextClass::Label, true, axis);
        }
        r
    }

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, rect: Rect) {
        // We center the box vertically and align to the left
        let box_size = size_handle.checkbox();
        let mut pos = rect.pos;
        let extra_height = rect.size.1 as i32 - box_size.1 as i32;
        pos.1 += extra_height / 2;
        self.box_pos = pos;
        // Text is drawn in the area to the right of this
        let margin = size_handle.outer_margin().0;
        self.text_pos_x = pos.0 + (margin + box_size.0) as i32;
        self.core_data_mut().rect = rect;
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, ev_mgr: &event::Manager) {
        let highlights = ev_mgr.highlight_state(self.id());
        draw_handle.checkbox(self.box_pos, self.state, highlights);
        let mut text_rect = self.core.rect;
        text_rect.pos.0 = self.text_pos_x;
        if !self.label.is_empty() {
            let props = TextProperties {
                class: TextClass::Label,
                multi_line: true,
                horiz: Align::Begin,
                vert: Align::Centre,
            };
            draw_handle.text(text_rect, &self.label, props);
        }
    }
}

impl<M, OT: Fn(bool) -> M> CheckBox<OT> {
    /// Construct a checkbox with a given `label` which calls `f` when toggled.
    ///
    /// This is a shortcut for `CheckBox::new(label).on_toggle(f)`.
    ///
    /// Checkbox labels are optional; if no label is desired, use an empty
    /// string.
    ///
    /// The closure `f` is called with the new state of the checkbox when
    /// toggled, and the result of `f` is returned from the event handler.
    pub fn new_on<S: Into<String>>(label: S, f: OT) -> Self {
        CheckBox {
            core: Default::default(),
            box_pos: Default::default(),
            text_pos_x: 0,
            label: label.into(),
            state: false,
            on_toggle: f,
        }
    }
}

impl CheckBox<()> {
    /// Construct a checkbox with a given `label`.
    ///
    /// CheckBox labels are optional; if no label is desired, use an empty
    /// string.
    pub fn new<S: Into<String>>(label: S) -> Self {
        CheckBox {
            core: Default::default(),
            box_pos: Default::default(),
            text_pos_x: 0,
            label: label.into(),
            state: false,
            on_toggle: (),
        }
    }

    /// Set the event handler to be called on toggle.
    ///
    /// The closure `f` is called with the new state of the checkbox when
    /// toggled, and the result of `f` is returned from the event handler.
    pub fn on_toggle<M, OT: Fn(bool) -> M>(self, f: OT) -> CheckBox<OT> {
        CheckBox {
            core: self.core,
            box_pos: self.box_pos,
            text_pos_x: self.text_pos_x,
            label: self.label,
            state: self.state,
            on_toggle: f,
        }
    }
}

impl<OT: 'static> CheckBox<OT> {
    /// Set the initial state of the checkbox.
    pub fn set_state(&mut self, state: bool) {
        self.state = state;
    }

    /// Set the initial state of the checkbox.
    pub fn state(mut self, state: bool) -> Self {
        self.state = state;
        self
    }

    /// Whether this checkbox has a handler set on toggle actions.
    // TODO: this needs to be defined on a trait that the toolkit can access
    pub fn has_on_toggle(&self) -> bool {
        TypeId::of::<OT>() != TypeId::of::<()>()
    }
}

impl<H> HasBool for CheckBox<H> {
    fn get_bool(&self) -> bool {
        self.state
    }

    fn set_bool(&mut self, _tk: &mut dyn TkWindow, state: bool) {
        self.state = state;
    }
}

impl<H> HasText for CheckBox<H> {
    fn get_text(&self) -> &str {
        &self.label
    }

    fn set_string(&mut self, tk: &mut dyn TkWindow, text: String) {
        self.label = text;
        tk.redraw(self.id());
    }
}

impl Handler for CheckBox<()> {
    type Msg = EmptyMsg;

    fn handle_action(&mut self, tk: &mut dyn TkWindow, action: Action) -> EmptyMsg {
        match action {
            Action::Activate => {
                self.state = !self.state;
                tk.redraw(self.id());
                EmptyMsg
            }
            a @ _ => err_unhandled(a),
        }
    }
}

impl<M: From<EmptyMsg>, H: Fn(bool) -> M> Handler for CheckBox<H> {
    type Msg = M;

    fn handle_action(&mut self, tk: &mut dyn TkWindow, action: Action) -> M {
        match action {
            Action::Activate => {
                self.state = !self.state;
                tk.redraw(self.id());
                ((self.on_toggle)(self.state)).into()
            }
            a @ _ => err_unhandled(a),
        }
    }
}

/// A push-button with a text label
// TODO: abstract out text part?
#[widget]
#[derive(Clone, Debug, Default, Widget)]
pub struct TextButton<M: Clone + Debug + From<EmptyMsg>> {
    #[core]
    core: CoreData,
    text_rect: Rect,
    label: String,
    msg: M,
}

impl<M: Clone + Debug + From<EmptyMsg>> Widget for TextButton<M> {
    fn allow_focus(&self) -> bool {
        true
    }

    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let sides = size_handle.button_surround();
        SizeRules::fixed(axis.extract_size(sides.0 + sides.1))
            + size_handle.text_bound(&self.label, TextClass::Button, false, axis)
    }

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, rect: Rect) {
        let sides = size_handle.button_surround();
        self.text_rect = Rect {
            pos: rect.pos + sides.0,
            size: rect.size - (sides.0 + sides.1),
        };
        self.core_data_mut().rect = rect;
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, ev_mgr: &event::Manager) {
        draw_handle.button(self.core.rect, ev_mgr.highlight_state(self.id()));
        let props = TextProperties {
            class: TextClass::Button,
            multi_line: false,
            horiz: Align::Centre,
            vert: Align::Centre,
        };
        draw_handle.text(self.text_rect, &self.label, props);
    }
}

impl<M: Clone + Debug + From<EmptyMsg>> TextButton<M> {
    /// Construct a button with a given `label` and `msg`
    ///
    /// The message `msg` is returned to the parent widget on activation. Any
    /// type supporting `Clone` is valid, though it is recommended to use a
    /// simple `Copy` type (e.g. an enum). Click actions must be implemented on
    /// the parent (or other ancestor).
    pub fn new<S: Into<String>>(label: S, msg: M) -> Self {
        TextButton {
            core: Default::default(),
            text_rect: Default::default(),
            label: label.into(),
            msg,
        }
    }

    /// Set accelerator keys (chain style)
    pub fn with_keys(mut self, keys: &[VirtualKeyCode]) -> Self {
        self.set_keys(keys);
        self
    }

    /// Replace the message value
    pub fn set_msg(&mut self, msg: M) {
        self.msg = msg;
    }

    /// Set accelerator keys
    pub fn set_keys(&mut self, keys: &[VirtualKeyCode]) {
        self.core.set_keys(keys);
    }
}

impl<M: Clone + Debug + From<EmptyMsg>> HasText for TextButton<M> {
    fn get_text(&self) -> &str {
        &self.label
    }

    fn set_string(&mut self, tk: &mut dyn TkWindow, text: String) {
        self.label = text;
        tk.redraw(self.id());
    }
}

impl<M: Clone + Debug + From<EmptyMsg>> Handler for TextButton<M> {
    type Msg = M;

    fn handle_action(&mut self, _: &mut dyn TkWindow, action: Action) -> M {
        match action {
            Action::Activate => self.msg.clone().into(),
            a @ _ => err_unhandled(a),
        }
    }
}
