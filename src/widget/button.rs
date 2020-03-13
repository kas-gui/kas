// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Push-buttons

use smallvec::SmallVec;
use std::fmt::Debug;

use crate::class::HasText;
use crate::draw::{DrawHandle, SizeHandle, TextClass};
use crate::event::{self, Action, Manager, Response, VirtualKeyCode};
use crate::geom::{Coord, Rect};
use crate::layout::{AxisInfo, SizeRules};
use crate::macros::Widget;
use crate::{Align, AlignHints, CoreData, Layout, Widget, WidgetCore, WidgetId};

/// A push-button with a text label
#[derive(Clone, Debug, Default, Widget)]
pub struct TextButton<M: Clone + Debug> {
    #[core]
    core: CoreData,
    keys: SmallVec<[VirtualKeyCode; 4]>,
    b_rect: Rect,
    // text_rect: Rect,
    label: String,
    msg: M,
}

impl<M: Clone + Debug> Widget for TextButton<M> {
    fn configure(&mut self, mgr: &mut Manager) {
        for key in &self.keys {
            mgr.add_accel_key(*key, self.id());
        }
    }

    fn allow_focus(&self) -> bool {
        true
    }
}

impl<M: Clone + Debug> Layout for TextButton<M> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let margin = size_handle.outer_margin();
        let sides = size_handle.button_surround();
        let rules = SizeRules::fixed(axis.extract_size(sides.0 + sides.1 + margin))
            + size_handle.text_bound(&self.label, TextClass::Button, axis);
        if axis.is_horizontal() {
            self.core.rect.size.0 = rules.ideal_size();
        } else {
            self.core.rect.size.1 = rules.ideal_size();
        }
        rules
    }

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, rect: Rect, align: AlignHints) {
        let rect = align
            .complete(Align::Stretch, Align::Stretch, self.rect().size)
            .apply(rect);
        self.core.rect = rect;

        // Add a margin around the button.
        // TODO: may be better to add margins in layout.
        let margin = size_handle.outer_margin();
        self.b_rect = Rect {
            pos: rect.pos + margin,
            size: rect.size - margin - margin,
        };

        // In theory, text rendering *should* be restricted to this rect. In
        // practice, it sometimes overflows a tiny bit, and looks better if we
        // do let it overflow. Since the text is centred this is okay
        // (assuming the theme's frame is symmetric).
        // let sides = size_handle.button_surround();
        // self.text_rect = Rect {
        //     pos: self.b_rect.pos + sides.0,
        //     size: self.b_rect.size - (sides.0 + sides.1),
        // };
    }

    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if self.b_rect.contains(coord) {
            Some(self.id())
        } else {
            None
        }
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState) {
        draw_handle.button(self.b_rect, mgr.highlight_state(self.id()));
        let align = (Align::Centre, Align::Centre);
        draw_handle.text(self.b_rect, &self.label, TextClass::Button, align);
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
            b_rect: Default::default(),
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

    fn set_string(&mut self, mgr: &mut Manager, text: String) {
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

    fn handle_action(&mut self, _: &mut Manager, action: Action) -> Response<M> {
        match action {
            Action::Activate => self.msg.clone().into(),
            a @ _ => Response::unhandled_action(a),
        }
    }
}
