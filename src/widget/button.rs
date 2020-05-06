// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Push-buttons

use smallvec::SmallVec;
use std::fmt::Debug;

use kas::class::HasText;
use kas::draw::TextClass;
use kas::event::{VirtualKeyCode, VirtualKeyCodes};
use kas::prelude::*;

/// A push-button with a text label
#[handler(handle=noauto)]
#[widget(config=noauto)]
#[derive(Clone, Debug, Default, Widget)]
pub struct TextButton<M: Clone + Debug + 'static> {
    #[widget_core]
    core: kas::CoreData,
    keys: VirtualKeyCodes,
    // text_rect: Rect,
    label: AccelString,
    msg: M,
}

impl<M: Clone + Debug + 'static> WidgetConfig for TextButton<M> {
    fn configure(&mut self, mgr: &mut Manager) {
        // TODO: consider merging these two lists?
        mgr.add_accel_keys(self.id(), &self.keys);
        mgr.add_accel_keys(self.id(), self.label.keys());
    }

    fn key_nav(&self) -> bool {
        true
    }
}

impl<M: Clone + Debug + 'static> Layout for TextButton<M> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let sides = size_handle.button_surround();
        let margins = size_handle.outer_margins();
        let frame_rules = SizeRules::extract_fixed(axis.is_vertical(), sides.0 + sides.1, margins);

        let content_rules = size_handle.text_bound(self.label.get(false), TextClass::Button, axis);
        content_rules.surrounded_by(frame_rules, true)
    }

    fn set_rect(&mut self, rect: Rect, _align: AlignHints) {
        self.core.rect = rect;

        // In theory, text rendering should be restricted as in EditBox.
        // In practice, it sometimes overflows a tiny bit, and looks better if
        // we let it overflow. Since the text is centred this is okay.
        // self.text_rect = ...
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        draw_handle.button(self.core.rect, self.input_state(mgr, disabled));
        let text = self.label.get(mgr.show_accel_labels());
        let align = (Align::Centre, Align::Centre);
        draw_handle.text(self.core.rect, text, TextClass::Button, align);
    }
}

impl<M: Clone + Debug + 'static> TextButton<M> {
    /// Construct a button with a given `label` and `msg`
    ///
    /// The message `msg` is returned to the parent widget on activation. Any
    /// type supporting `Clone` is valid, though it is recommended to use a
    /// simple `Copy` type (e.g. an enum). Click actions must be implemented on
    /// the parent (or other ancestor).
    pub fn new<S: Into<AccelString>>(label: S, msg: M) -> Self {
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

impl<M: Clone + Debug + 'static> HasText for TextButton<M> {
    fn get_text(&self) -> &str {
        self.label.get(false)
    }

    fn set_cow_string(&mut self, text: CowString) -> TkAction {
        self.label = text.into();
        TkAction::Redraw
    }
}

impl<M: Clone + Debug + 'static> event::Handler for TextButton<M> {
    type Msg = M;

    #[inline]
    fn activation_via_press(&self) -> bool {
        true
    }

    fn handle(&mut self, _: &mut Manager, event: Event) -> Response<M> {
        match event {
            Event::Activate => self.msg.clone().into(),
            event => Response::Unhandled(event),
        }
    }
}
