// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Menu Entries

use std::fmt::{self, Debug};

use super::Menu;
use kas::draw::TextClass;
use kas::event;
use kas::layout::{self, RulesSetter, RulesSolver};
use kas::prelude::*;
use kas::widget::{AccelLabel, CheckBoxBare};

/// A standard menu entry
#[widget(config=noauto)]
#[handler(handle=noauto)]
#[derive(Clone, Debug, Default, Widget)]
pub struct MenuEntry<M: Clone + Debug + 'static> {
    #[widget_core]
    core: kas::CoreData,
    label: Text<AccelString>,
    label_off: Coord,
    msg: M,
}

impl<M: Clone + Debug + 'static> WidgetConfig for MenuEntry<M> {
    fn configure(&mut self, mgr: &mut Manager) {
        mgr.add_accel_keys(self.id(), &self.label.text().keys());
    }

    fn key_nav(&self) -> bool {
        true
    }
}

impl<M: Clone + Debug + 'static> Layout for MenuEntry<M> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let size = size_handle.menu_frame();
        self.label_off = size.into();
        let frame_rules = SizeRules::extract_fixed(axis.is_vertical(), size + size, Margins::ZERO);
        let text_rules = size_handle.text_bound(&mut self.label, TextClass::LabelSingle, axis);
        text_rules.surrounded_by(frame_rules, true)
    }

    fn set_rect(&mut self, rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        self.label.update_env(|env| {
            env.set_bounds(rect.size.into());
            env.set_align(align.unwrap_or(Align::Default, Align::Centre));
        });
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        draw_handle.menu_entry(self.core.rect, self.input_state(mgr, disabled));
        let pos = self.core.rect.pos + self.label_off;
        // TODO: draw state is mgr.show_accel_labels()
        draw_handle.text(pos, &self.label, TextClass::LabelSingle);
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
            label_off: Coord::ZERO,
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
        kas::text::util::set_text_and_prepare(&mut self.label, string)
    }
}

impl<M: Clone + Debug + 'static> event::Handler for MenuEntry<M> {
    type Msg = M;

    fn handle(&mut self, _: &mut Manager, event: Event) -> Response<M> {
        match event {
            Event::Activate => self.msg.clone().into(),
            event => Response::Unhandled(event),
        }
    }
}

impl<M: Clone + Debug> Menu for MenuEntry<M> {}

/// A menu entry which can be toggled
#[handler(msg = M, generics = <> where M: From<VoidMsg>)]
#[widget(config=noauto)]
#[derive(Clone, Default, Widget)]
pub struct MenuToggle<M: 'static> {
    #[widget_core]
    core: CoreData,
    layout_data: layout::FixedRowStorage<[SizeRules; 3], [u32; 2]>,
    #[widget]
    checkbox: CheckBoxBare<M>,
    #[widget]
    label: AccelLabel,
}

impl<M: 'static> Debug for MenuToggle<M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "MenuToggle {{ core: {:?}, layout_data: {:?}, checkbox: {:?}, label: {:?} }}",
            self.core, self.layout_data, self.checkbox, self.label,
        )
    }
}

impl<M: 'static> MenuToggle<M> {
    /// Construct a togglable menu entry with a given `label` and closure
    ///
    /// This is a shortcut for `MenuToggle::new(label).on_toggle(f)`.
    /// The closure `f` is called with the new state of the checkbox when
    /// toggled, and the result of `f` is returned from the event handler.
    #[inline]
    pub fn new_on<T: Into<AccelString>, F>(f: F, label: T) -> Self
    where
        F: Fn(bool) -> M + 'static,
    {
        MenuToggle {
            core: Default::default(),
            layout_data: Default::default(),
            checkbox: CheckBoxBare::new_on(f),
            label: AccelLabel::new(label.into()),
        }
    }

    /// Set the initial state of the checkbox.
    #[inline]
    pub fn state(mut self, state: bool) -> Self {
        self.checkbox = self.checkbox.state(state);
        self
    }
}

impl MenuToggle<VoidMsg> {
    /// Construct a togglable menu entry with a given `label`
    #[inline]
    pub fn new<T: Into<AccelString>>(label: T) -> Self {
        MenuToggle {
            core: Default::default(),
            layout_data: Default::default(),
            checkbox: CheckBoxBare::new(),
            label: AccelLabel::new(label.into()),
        }
    }

    /// Set the event handler to be called on toggle.
    ///
    /// The closure `f` is called with the new state of the checkbox when
    /// toggled, and the result of `f` is returned from the event handler.
    #[inline]
    pub fn on_toggle<M, F>(self, f: F) -> MenuToggle<M>
    where
        F: Fn(bool) -> M + 'static,
    {
        MenuToggle {
            core: self.core,
            layout_data: self.layout_data,
            checkbox: self.checkbox.on_toggle(f),
            label: self.label,
        }
    }
}

impl<M: 'static> WidgetConfig for MenuToggle<M> {
    fn configure(&mut self, mgr: &mut Manager) {
        mgr.add_accel_keys(self.checkbox.id(), self.label.keys());
    }
}

impl<M: 'static> Layout for MenuToggle<M> {
    // NOTE: This code is mostly copied from the macro expansion.
    // Only draw() is significantly different.
    fn size_rules(
        &mut self,
        size_handle: &mut dyn SizeHandle,
        axis: AxisInfo,
    ) -> kas::layout::SizeRules {
        let mut solver = layout::RowSolver::new(axis, (kas::Right, 2usize), &mut self.layout_data);
        let child = &mut self.checkbox;
        solver.for_child(&mut self.layout_data, 0usize, |axis| {
            child.size_rules(size_handle, axis)
        });
        let child = &mut self.label;
        solver.for_child(&mut self.layout_data, 1usize, |axis| {
            child.size_rules(size_handle, axis)
        });
        solver.finish(&mut self.layout_data)
    }

    fn set_rect(&mut self, rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        let mut setter = layout::RowSetter::<_, [u32; 2], _>::new(
            rect,
            (kas::Right, 2usize),
            align,
            &mut self.layout_data,
        );
        let align = kas::AlignHints::NONE;
        self.checkbox.set_rect(
            setter.child_rect(&mut self.layout_data, 0usize),
            align.clone(),
        );
        self.label
            .set_rect(setter.child_rect(&mut self.layout_data, 1usize), align);
    }

    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if !self.rect().contains(coord) {
            return None;
        }
        Some(self.checkbox.id())
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        let state = self.checkbox.input_state(mgr, disabled);
        draw_handle.menu_entry(self.core.rect, state);
        self.checkbox.draw(draw_handle, mgr, state.disabled);
        self.label.draw(draw_handle, mgr, state.disabled);
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
