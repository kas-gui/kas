// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Control widgets accept simple input

use std::any::TypeId;
use std::fmt::{self, Debug};

use crate::class::{Class, HasBool, HasText};
use crate::event::{err_unhandled, Action, Handler, Response, VirtualKeyCode};
use crate::macros::Widget;
use crate::{CoreData, TkWindow};

/// A checkable box with optional label
#[widget(class = Class::CheckBox(self), layout = derive)]
#[derive(Clone, Default, Widget)]
pub struct CheckBox<OT: 'static> {
    #[core]
    core: CoreData,
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
            label: label.into(),
            state: false,
            on_toggle: (),
        }
    }

    /// Set the event handler to be called on toggle.
    ///
    /// The closure `f` is called with the new state of the checkbox when
    /// toggled, and the result of `f` is returned from the event handler.
    pub fn on_toggle<M, OT: Fn() -> M>(self, f: OT) -> CheckBox<OT> {
        CheckBox {
            core: self.core,
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
        tk.redraw(self);
    }
}

impl Handler for CheckBox<()> {
    type Msg = ();

    fn handle_action(&mut self, tk: &mut dyn TkWindow, action: Action) -> Response<()> {
        match action {
            Action::Activate => {
                self.state = !self.state;
                tk.redraw(self);
                Response::None
            }
            a @ _ => err_unhandled(a),
        }
    }
}

impl<M, H: Fn(bool) -> M> Handler for CheckBox<H> {
    type Msg = M;

    fn handle_action(&mut self, tk: &mut dyn TkWindow, action: Action) -> Response<M> {
        match action {
            Action::Activate => {
                self.state = !self.state;
                tk.redraw(self);
                ((self.on_toggle)(self.state)).into()
            }
            a @ _ => err_unhandled(a),
        }
    }
}

/// A push-button with a text label
// TODO: abstract out text part?
#[widget(class = Class::Button(self), layout = derive)]
#[derive(Clone, Debug, Default, Widget)]
pub struct TextButton<M: Clone + Debug> {
    #[core]
    core: CoreData,
    label: String,
    msg: M,
}

impl<M: Clone + Debug> TextButton<M> {
    /// Construct a button with a given `label` and `msg`
    ///
    /// The message `msg` is returned to the parent widget on activation. Any
    /// type supporting `Clone` is valid, though it is recommended to use a
    /// simple `Copy` type (e.g. an enum). Click actions must be implemented on
    /// the parent (or other ancestor).
    pub fn new<S: Into<String>>(label: S, msg: M) -> Self {
        TextButton {
            core: Default::default(),
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

impl<M: Clone + Debug> HasText for TextButton<M> {
    fn get_text(&self) -> &str {
        &self.label
    }

    fn set_string(&mut self, tk: &mut dyn TkWindow, text: String) {
        self.label = text;
        tk.redraw(self);
    }
}

impl<M: Clone + Debug> Handler for TextButton<M> {
    type Msg = M;

    fn handle_action(&mut self, _: &mut dyn TkWindow, action: Action) -> Response<M> {
        match action {
            Action::Activate => self.msg.clone().into(),
            a @ _ => err_unhandled(a),
        }
    }
}
