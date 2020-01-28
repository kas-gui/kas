// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Push-buttons

use smallvec::SmallVec;
use std::fmt::Debug;

use crate::class::HasText;
use crate::event::{Action, Handler, Manager, Response, VirtualKeyCode};
use crate::layout::{AxisInfo, SizeRules};
use crate::macros::Widget;
use crate::theme::{Align, DrawHandle, SizeHandle, TextClass, TextProperties};
use crate::{CoreData, Layout, Widget, WidgetCore, WidgetId};
use kas::geom::Rect;

/// A push-button with a text label
#[derive(Clone, Debug, Default, Widget)]
pub struct TextButton<M: Clone + Debug> {
    #[core]
    core: CoreData,
    keys: SmallVec<[VirtualKeyCode; 4]>,
    text_rect: Rect,
    label: String,
    msg: M,
}

impl<M: Clone + Debug> Widget for TextButton<M> {
    fn configure(&mut self, id: WidgetId, mgr: &mut Manager) {
        self.core_data_mut().id = id;
        for key in &self.keys {
            mgr.add_accel_key(*key, id);
        }
    }

    fn allow_focus(&self) -> bool {
        true
    }
}

impl<M: Clone + Debug> Layout for TextButton<M> {
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

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &Manager) {
        draw_handle.button(self.core.rect, mgr.highlight_state(self.id()));
        let props = TextProperties {
            class: TextClass::Button,
            multi_line: false,
            horiz: Align::Centre,
            vert: Align::Centre,
        };
        draw_handle.text(self.text_rect, &self.label, props);
    }
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
            keys: SmallVec::new(),
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
        self.keys = SmallVec::from_slice(keys);
    }
}

impl<M: Clone + Debug> HasText for TextButton<M> {
    fn get_text(&self) -> &str {
        &self.label
    }

    fn set_string(&mut self, mgr: &mut Manager, text: String) {
        self.label = text;
        mgr.redraw(self.id());
    }
}

impl<M: Clone + Debug> Handler for TextButton<M> {
    type Msg = M;

    #[inline]
    fn activation_via_press(&self) -> bool {
        true
    }

    fn handle_action(&mut self, _: &mut Manager, action: Action) -> Response<M> {
        match action {
            Action::Activate => self.msg.clone().into(),
            a @ _ => Response::unhandled_action(a),
        }
    }
}
