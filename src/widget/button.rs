// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Push-buttons

use smallvec::SmallVec;
use std::fmt::Debug;

use kas::class::HasText;
use kas::draw::{DrawHandle, SizeHandle, TextClass};
use kas::event::{Action, Manager, Response, VirtualKeyCode};
use kas::layout::{AxisInfo, SizeRules};
use kas::prelude::*;

/// A push-button with a text label
#[handler(event)]
#[widget(config=noauto)]
#[derive(Clone, Debug, Default, Widget)]
pub struct TextButton<M: Clone + Debug> {
    #[widget_core]
    core: kas::CoreData,
    keys: SmallVec<[VirtualKeyCode; 4]>,
    // text_rect: Rect,
    label: CowString,
    msg: M,
}

impl<M: Clone + Debug> WidgetConfig for TextButton<M> {
    fn configure(&mut self, mgr: &mut Manager) {
        for key in &self.keys {
            mgr.add_accel_key(*key, self.id());
        }
    }

    fn key_nav(&self) -> bool {
        true
    }
}

impl<M: Clone + Debug> Layout for TextButton<M> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let sides = size_handle.button_surround();
        let margins = size_handle.outer_margins();
        let frame_rules = SizeRules::extract_fixed(axis.dir(), sides.0 + sides.1, margins);

        let content_rules = size_handle.text_bound(&self.label, TextClass::Button, axis);
        content_rules.surrounded_by(frame_rules, true)
    }

    fn set_rect(&mut self, _size_handle: &mut dyn SizeHandle, rect: Rect, _align: AlignHints) {
        self.core.rect = rect;

        // In theory, text rendering should be restricted as in EditBox.
        // In practice, it sometimes overflows a tiny bit, and looks better if
        // we let it overflow. Since the text is centred this is okay.
        // self.text_rect = ...
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState) {
        draw_handle.button(self.core.rect, mgr.highlight_state(self.id()));
        let align = (Align::Centre, Align::Centre);
        draw_handle.text(self.core.rect, &self.label, TextClass::Button, align);
    }
}

impl<M: Clone + Debug> TextButton<M> {
    /// Construct a button with a given `label` and `msg`
    ///
    /// The message `msg` is returned to the parent widget on activation. Any
    /// type supporting `Clone` is valid, though it is recommended to use a
    /// simple `Copy` type (e.g. an enum). Click actions must be implemented on
    /// the parent (or other ancestor).
    pub fn new<S: Into<CowString>>(label: S, msg: M) -> Self {
        TextButton {
            core: Default::default(),
            keys: SmallVec::new(),
            // text_rect: Default::default(),
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

    fn set_cow_string(&mut self, mgr: &mut Manager, text: CowString) {
        self.label = text;
        mgr.redraw(self.id());
    }
}

impl<M: Clone + Debug> event::Handler for TextButton<M> {
    type Msg = M;

    #[inline]
    fn activation_via_press(&self) -> bool {
        true
    }

    fn action(&mut self, _: &mut Manager, action: Action) -> Response<M> {
        match action {
            Action::Activate => self.msg.clone().into(),
            a @ _ => Response::unhandled_action(a),
        }
    }
}
