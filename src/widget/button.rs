// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Push-buttons

use std::fmt::Debug;

use kas::class::{CloneText, SetAccel};
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
    keys1: VirtualKeyCodes,
    keys2: VirtualKeyCodes,
    // label_rect: Rect,
    label: PreparedText,
    msg: M,
}

impl<M: Clone + Debug + 'static> WidgetConfig for TextButton<M> {
    fn configure(&mut self, mgr: &mut Manager) {
        mgr.add_accel_keys(self.id(), &self.keys1);
        mgr.add_accel_keys(self.id(), &self.keys2);
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

        let content_rules = size_handle.text_bound(&mut self.label, TextClass::Button, axis);
        content_rules.surrounded_by(frame_rules, true)
    }

    fn set_rect(&mut self, rect: Rect, align: AlignHints) {
        self.core.rect = rect;

        // In theory, text rendering should be restricted as in EditBox.
        // In practice, it sometimes overflows a tiny bit, and looks better if
        // we let it overflow. Since the text is centred this is okay.
        // self.label_rect = ...
        self.label.set_size(rect.size);
        self.label.set_alignment(
            align.horiz.unwrap_or(Align::Centre),
            align.vert.unwrap_or(Align::Centre),
        );
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        draw_handle.button(self.core.rect, self.input_state(mgr, disabled));
        // TODO: mgr.show_accel_labels();
        draw_handle.text(self.core.rect.pos, &self.label, TextClass::Button);
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
        let text = PreparedText::new(label.get(false).into(), false);
        let keys2 = label.take_keys();
        TextButton {
            core: Default::default(),
            keys1: Default::default(),
            keys2,
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

impl<M: Clone + Debug + 'static> CloneText for TextButton<M> {
    fn clone_text(&self) -> kas::text::RichText {
        self.label.clone_text()
    }
}

impl<M: Clone + Debug + 'static> SetAccel for TextButton<M> {
    fn set_accel_string(&mut self, label: AccelString) -> TkAction {
        let text = label.get(false).to_string();
        self.keys2 = label.take_keys();
        self.label.set_text(text)
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
