// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Push-buttons

use std::fmt::Debug;

use crate::class::HasText;
use crate::event::{self, err_unhandled, Action, EmptyMsg, Handler, VirtualKeyCode};
use crate::layout::{AxisInfo, SizeRules};
use crate::macros::Widget;
use crate::theme::{Align, DrawHandle, SizeHandle, TextClass, TextProperties};
use crate::{CoreData, TkWindow, Widget, WidgetCore};
use kas::geom::Rect;

/// A push-button with a text label
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

    fn draw(
        &self,
        draw_handle: &mut dyn DrawHandle,
        offset: kas::geom::Coord,
        ev_mgr: &event::Manager,
    ) {
        draw_handle.button(self.core.rect + offset, ev_mgr.highlight_state(self.id()));
        let props = TextProperties {
            class: TextClass::Button,
            multi_line: false,
            horiz: Align::Centre,
            vert: Align::Centre,
        };
        draw_handle.text(self.text_rect + offset, &self.label, props);
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
