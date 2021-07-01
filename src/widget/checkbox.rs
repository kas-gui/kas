// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toggle widgets

use std::fmt::{self, Debug};
use std::rc::Rc;

use super::AccelLabel;
use kas::{event, prelude::*};

/// A bare checkbox (no label)
#[derive(Clone, Default, Widget)]
#[widget(config(key_nav = true, hover_highlight = true))]
#[handler(handle=noauto)]
pub struct CheckBoxBare<M: 'static> {
    #[widget_core]
    core: CoreData,
    state: bool,
    on_toggle: Option<Rc<dyn Fn(&mut Manager, bool) -> Option<M>>>,
}

impl<M: 'static> Debug for CheckBoxBare<M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CheckBoxBare")
            .field("core", &self.core)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl<M: 'static> Layout for CheckBoxBare<M> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let size = size_handle.checkbox();
        self.core.rect.size = size;
        let margins = size_handle.outer_margins();
        SizeRules::extract_fixed(axis, size, margins)
    }

    fn set_rect(&mut self, _: &mut Manager, rect: Rect, align: AlignHints) {
        let rect = align
            .complete(Align::Centre, Align::Centre)
            .aligned_rect(self.rect().size, rect);
        self.core.rect = rect;
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        draw_handle.checkbox(self.core.rect, self.state, self.input_state(mgr, disabled));
    }
}

impl CheckBoxBare<VoidMsg> {
    /// Construct a checkbox
    #[inline]
    pub fn new() -> Self {
        CheckBoxBare {
            core: Default::default(),
            state: false,
            on_toggle: None,
        }
    }

    /// Set event handler `f`
    ///
    /// On toggle (through user input events or [`Event::Activate`]) the
    /// closure `f` is called. The result of `f` is converted to
    /// [`Response::Msg`] or [`Response::Update`] and returned to the parent.
    #[inline]
    pub fn on_toggle<M, F>(self, f: F) -> CheckBoxBare<M>
    where
        F: Fn(&mut Manager, bool) -> Option<M> + 'static,
    {
        CheckBoxBare {
            core: self.core,
            state: self.state,
            on_toggle: Some(Rc::new(f)),
        }
    }
}

impl<M: 'static> CheckBoxBare<M> {
    /// Construct a checkbox with event handler `f`
    ///
    /// On activation (through user input events or [`Event::Activate`]) the
    /// closure `f` is called. The result of `f` is converted to
    /// [`Response::Msg`] or [`Response::Update`] and returned to the parent.
    #[inline]
    pub fn new_on<F>(f: F) -> Self
    where
        F: Fn(&mut Manager, bool) -> Option<M> + 'static,
    {
        CheckBoxBare::new().on_toggle(f)
    }
}

impl<M: 'static> CheckBoxBare<M> {
    /// Set the initial state of the checkbox.
    #[inline]
    pub fn with_state(mut self, state: bool) -> Self {
        self.state = state;
        self
    }
}

impl<M: 'static> HasBool for CheckBoxBare<M> {
    fn get_bool(&self) -> bool {
        self.state
    }

    fn set_bool(&mut self, state: bool) -> TkAction {
        self.state = state;
        TkAction::REDRAW
    }
}

impl<M: 'static> event::Handler for CheckBoxBare<M> {
    type Msg = M;

    #[inline]
    fn activation_via_press(&self) -> bool {
        true
    }

    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<M> {
        match event {
            Event::Activate => {
                self.state = !self.state;
                mgr.redraw(self.id());
                Response::update_or_msg(self.on_toggle.as_ref().and_then(|f| f(mgr, self.state)))
            }
            _ => Response::Unhandled,
        }
    }
}

/// A checkable box with optional label
#[derive(Clone, Default, Widget)]
#[layout(row, area=checkbox)]
#[handler(msg = M, generics = <> where M: From<VoidMsg>)]
#[widget(config=noauto)]
#[widget_derive(HasBool)]
pub struct CheckBox<M: 'static> {
    #[widget_core]
    core: CoreData,
    #[layout_data]
    layout_data: <Self as kas::LayoutData>::Data,
    #[widget_derive]
    #[widget]
    checkbox: CheckBoxBare<M>,
    #[widget]
    label: AccelLabel,
}

impl<M: 'static> Debug for CheckBox<M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CheckBox")
            .field("core", &self.core)
            .field("layout_data", &self.layout_data)
            .field("checkbox", &self.checkbox)
            .field("label", &self.label)
            .finish()
    }
}

impl CheckBox<VoidMsg> {
    /// Construct a checkbox with a given `label`
    ///
    /// CheckBox labels are optional; if no label is desired, use an empty
    /// string.
    #[inline]
    pub fn new<T: Into<AccelString>>(label: T) -> Self {
        CheckBox {
            core: Default::default(),
            layout_data: Default::default(),
            checkbox: CheckBoxBare::new(),
            label: AccelLabel::new(label.into()),
        }
    }

    /// Set event handler `f`
    ///
    /// On toggle (through user input events or [`Event::Activate`]) the
    /// closure `f` is called. The result of `f` is converted to
    /// [`Response::Msg`] or [`Response::Update`] and returned to the parent.
    #[inline]
    pub fn on_toggle<M, F>(self, f: F) -> CheckBox<M>
    where
        F: Fn(&mut Manager, bool) -> Option<M> + 'static,
    {
        CheckBox {
            core: self.core,
            layout_data: self.layout_data,
            checkbox: self.checkbox.on_toggle(f),
            label: self.label,
        }
    }
}

impl<M: 'static> CheckBox<M> {
    /// Construct a checkbox with a given `label` and event handler `f`
    ///
    /// Checkbox labels are optional; if no label is desired, use an empty
    /// string.
    ///
    /// On toggle (through user input events or [`Event::Activate`]) the
    /// closure `f` is called. The result of `f` is converted to
    /// [`Response::Msg`] or [`Response::Update`] and returned to the parent.
    #[inline]
    pub fn new_on<T: Into<AccelString>, F>(label: T, f: F) -> Self
    where
        F: Fn(&mut Manager, bool) -> Option<M> + 'static,
    {
        CheckBox::new(label).on_toggle(f)
    }
}

impl<M: 'static> CheckBox<M> {
    /// Set the initial state of the checkbox.
    #[inline]
    pub fn with_state(mut self, state: bool) -> Self {
        self.checkbox = self.checkbox.with_state(state);
        self
    }
}

impl<M: 'static> WidgetConfig for CheckBox<M> {
    fn configure(&mut self, mgr: &mut Manager) {
        mgr.add_accel_keys(self.checkbox.id(), self.label.keys());
    }
}
