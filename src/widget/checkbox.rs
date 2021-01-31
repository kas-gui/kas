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
#[widget(config(key_nav = true))]
#[handler(handle=noauto)]
#[derive(Clone, Default, Widget)]
pub struct CheckBoxBare<M: 'static> {
    #[widget_core]
    core: CoreData,
    state: bool,
    on_toggle: Option<Rc<dyn Fn(bool) -> M>>,
}

impl<M: 'static> Debug for CheckBoxBare<M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "CheckBoxBare {{ core: {:?}, state: {:?}, ... }}",
            self.core, self.state
        )
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
            .complete(Align::Centre, Align::Centre, self.rect().size)
            .apply(rect);
        self.core.rect = rect;
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        draw_handle.checkbox(self.core.rect, self.state, self.input_state(mgr, disabled));
    }
}

impl<M: 'static> CheckBoxBare<M> {
    /// Construct a checkbox which calls `f` when toggled
    ///
    /// This is a shortcut for `CheckBoxBare::new().on_toggle(f)`.
    ///
    /// The closure `f` is called with the new state of the checkbox when
    /// toggled, and the result of `f` is returned from the event handler.
    #[inline]
    pub fn new_on<F: Fn(bool) -> M + 'static>(f: F) -> Self {
        CheckBoxBare {
            core: Default::default(),
            state: false,
            on_toggle: Some(Rc::new(f)),
        }
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

    /// Set the event handler to be called on toggle.
    ///
    /// The closure `f` is called with the new state of the checkbox when
    /// toggled, and the result of `f` is returned from the event handler.
    #[inline]
    pub fn on_toggle<M, F>(self, f: F) -> CheckBoxBare<M>
    where
        F: Fn(bool) -> M + 'static,
    {
        CheckBoxBare {
            core: self.core,
            state: self.state,
            on_toggle: Some(Rc::new(f)),
        }
    }
}

impl<M: 'static> CheckBoxBare<M> {
    /// Set the initial state of the checkbox.
    #[inline]
    pub fn state(mut self, state: bool) -> Self {
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
                if let Some(ref f) = self.on_toggle {
                    f(self.state).into()
                } else {
                    Response::None
                }
            }
            event => Response::Unhandled(event),
        }
    }
}

/// A checkable box with optional label
// TODO: use a generic wrapper for CheckBox and RadioBox?
#[layout(row, area=checkbox)]
#[handler(msg = M, generics = <> where M: From<VoidMsg>)]
#[widget(config=noauto)]
#[derive(Clone, Default, Widget)]
pub struct CheckBox<M: 'static> {
    #[widget_core]
    core: CoreData,
    #[layout_data]
    layout_data: <Self as kas::LayoutData>::Data,
    #[widget]
    checkbox: CheckBoxBare<M>,
    #[widget]
    label: AccelLabel,
}

impl<M: 'static> Debug for CheckBox<M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "CheckBox {{ core: {:?}, layout_data: {:?}, checkbox: {:?}, label: {:?} }}",
            self.core, self.layout_data, self.checkbox, self.label,
        )
    }
}

impl<M: 'static> CheckBox<M> {
    /// Construct a checkbox with a given `label` which calls `f` when toggled.
    ///
    /// This is a shortcut for `CheckBox::new(label).on_toggle(f)`.
    ///
    /// Checkbox labels are optional; if no label is desired, use an empty
    /// string.
    ///
    /// The closure `f` is called with the new state of the checkbox when
    /// toggled, and the result of `f` is returned from the event handler.
    #[inline]
    pub fn new_on<T: Into<AccelString>, F>(f: F, label: T) -> Self
    where
        F: Fn(bool) -> M + 'static,
    {
        CheckBox {
            core: Default::default(),
            layout_data: Default::default(),
            checkbox: CheckBoxBare::new_on(f),
            label: AccelLabel::new(label.into()),
        }
    }
}

impl CheckBox<VoidMsg> {
    /// Construct a checkbox with a given `label`.
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

    /// Set the event handler to be called on toggle.
    ///
    /// The closure `f` is called with the new state of the checkbox when
    /// toggled, and the result of `f` is returned from the event handler.
    #[inline]
    pub fn on_toggle<M, F>(self, f: F) -> CheckBox<M>
    where
        F: Fn(bool) -> M + 'static,
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
    /// Set the initial state of the checkbox.
    #[inline]
    pub fn state(mut self, state: bool) -> Self {
        self.checkbox = self.checkbox.state(state);
        self
    }
}

impl<M: 'static> WidgetConfig for CheckBox<M> {
    fn configure(&mut self, mgr: &mut Manager) {
        mgr.add_accel_keys(self.checkbox.id(), self.label.keys());
    }
}

impl<M: 'static> HasBool for CheckBox<M> {
    #[inline]
    fn get_bool(&self) -> bool {
        self.checkbox.get_bool()
    }

    #[inline]
    fn set_bool(&mut self, state: bool) -> TkAction {
        self.checkbox.set_bool(state)
    }
}
