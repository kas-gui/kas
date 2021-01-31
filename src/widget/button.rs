// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Push-buttons

use std::fmt::Debug;

use kas::draw::TextClass;
use kas::event::{self, VirtualKeyCode, VirtualKeyCodes};
use kas::prelude::*;

/// A push-button with a text label
#[handler(handle=noauto)]
#[widget(config=noauto)]
#[derive(Clone, Debug, Default, Widget)]
pub struct TextButton<M: Clone + Debug + 'static> {
    #[widget_core]
    core: kas::CoreData,
    keys1: VirtualKeyCodes,
    frame_size: Size,
    // label_rect: Rect,
    label: Text<AccelString>,
    msg: M,
}

impl<M: Clone + Debug + 'static> WidgetConfig for TextButton<M> {
    fn configure(&mut self, mgr: &mut Manager) {
        mgr.add_accel_keys(self.id(), &self.keys1);
        mgr.add_accel_keys(self.id(), &self.label.text().keys());
    }

    fn key_nav(&self) -> bool {
        true
    }
}

impl<M: Clone + Debug + 'static> Layout for TextButton<M> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let sides = size_handle.button_surround();
        self.frame_size = sides.0 + sides.1;
        let margins = size_handle.outer_margins();
        let frame_rules = SizeRules::extract_fixed(axis, self.frame_size, margins);

        let content_rules = size_handle.text_bound(&mut self.label, TextClass::Button, axis);
        content_rules.surrounded_by(frame_rules, true)
    }

    fn set_rect(&mut self, _: &mut Manager, rect: Rect, align: AlignHints) {
        self.core.rect = rect;

        // In theory, text rendering should be restricted as in EditBox.
        // In practice, it sometimes overflows a tiny bit, and looks better if
        // we let it overflow. Since the text is centred this is okay.
        // self.label_rect = ...
        self.label.update_env(|env| {
            env.set_bounds(rect.size.into());
            env.set_align(align.unwrap_or(Align::Centre, Align::Centre));
        });
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        draw_handle.button(self.core.rect, self.input_state(mgr, disabled));
        let state = mgr.show_accel_labels();
        draw_handle.text_accel(self.core.rect.pos, &self.label, state, TextClass::Button);
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
        let label = label.into();
        let text = Text::new_single(label);
        TextButton {
            core: Default::default(),
            keys1: Default::default(),
            frame_size: Default::default(),
            // label_rect: Default::default(),
            label: text,
            msg,
        }
    }

    /// Add accelerator keys (chain style)
    ///
    /// These keys are added to those inferred from the label via `&` marks.
    pub fn with_keys(mut self, keys: &[VirtualKeyCode]) -> Self {
        self.keys1.clear();
        self.keys1.extend_from_slice(keys);
        self
    }

    /// Replace the message value
    pub fn set_msg(&mut self, msg: M) {
        self.msg = msg;
    }
}

impl<M: Clone + Debug + 'static> HasStr for TextButton<M> {
    fn get_str(&self) -> &str {
        self.label.as_str()
    }
}

impl<M: Clone + Debug + 'static> SetAccel for TextButton<M> {
    fn set_accel_string(&mut self, string: AccelString) -> TkAction {
        let mut action = TkAction::empty();
        if self.label.text().keys() != string.keys() {
            action |= TkAction::RECONFIGURE;
        }
        let avail = self.core.rect.size.saturating_sub(self.frame_size);
        action | kas::text::util::set_text_and_prepare(&mut self.label, string, avail)
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
