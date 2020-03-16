// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toggle widgets

use std::fmt::{self, Debug};

use super::Label;
use crate::class::HasBool;
use crate::draw::{DrawHandle, SizeHandle};
use crate::event::{self, Action, Manager, Response, VoidMsg};
use crate::geom::Rect;
use crate::layout::{AxisInfo, SizeRules};
use crate::macros::Widget;
use crate::{Align, AlignHints, CoreData, CowString, Layout, Widget, WidgetCore};

/// A bare checkbox (no label)
#[derive(Clone, Default, Widget)]
pub struct CheckBoxBare<OT: 'static> {
    #[core]
    core: CoreData,
    state: bool,
    on_toggle: OT,
}

impl<H> Debug for CheckBoxBare<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "CheckBoxBare {{ core: {:?}, state: {:?}, ... }}",
            self.core, self.state
        )
    }
}

impl<OT: 'static> Widget for CheckBoxBare<OT> {
    fn allow_focus(&self) -> bool {
        true
    }
}

impl<OT: 'static> Layout for CheckBoxBare<OT> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let size = size_handle.checkbox();
        self.core.rect.size = size;
        let margins = size_handle.outer_margins();
        SizeRules::extract_fixed(axis.dir(), size, margins)
    }

    fn set_rect(&mut self, _size_handle: &mut dyn SizeHandle, rect: Rect, align: AlignHints) {
        let rect = align
            .complete(Align::Centre, Align::Centre, self.rect().size)
            .apply(rect);
        self.core.rect = rect;
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState) {
        let highlights = mgr.highlight_state(self.id());
        draw_handle.checkbox(self.core.rect, self.state, highlights);
    }
}

impl<M, OT: Fn(bool) -> M> CheckBoxBare<OT> {
    /// Construct a checkbox which calls `f` when toggled
    ///
    /// This is a shortcut for `CheckBoxBare::new().on_toggle(f)`.
    ///
    /// The closure `f` is called with the new state of the checkbox when
    /// toggled, and the result of `f` is returned from the event handler.
    #[inline]
    pub fn new_on(f: OT) -> Self {
        CheckBoxBare {
            core: Default::default(),
            state: false,
            on_toggle: f,
        }
    }
}

impl CheckBoxBare<()> {
    /// Construct a checkbox
    #[inline]
    pub fn new() -> Self {
        CheckBoxBare {
            core: Default::default(),
            state: false,
            on_toggle: (),
        }
    }

    /// Set the event handler to be called on toggle.
    ///
    /// The closure `f` is called with the new state of the checkbox when
    /// toggled, and the result of `f` is returned from the event handler.
    #[inline]
    pub fn on_toggle<M, OT: Fn(bool) -> M>(self, f: OT) -> CheckBoxBare<OT> {
        CheckBoxBare {
            core: self.core,
            state: self.state,
            on_toggle: f,
        }
    }
}

impl<OT: 'static> CheckBoxBare<OT> {
    /// Set the initial state of the checkbox.
    #[inline]
    pub fn state(mut self, state: bool) -> Self {
        self.state = state;
        self
    }
}

impl<H> HasBool for CheckBoxBare<H> {
    fn get_bool(&self) -> bool {
        self.state
    }

    fn set_bool(&mut self, mgr: &mut Manager, state: bool) {
        self.state = state;
        mgr.redraw(self.id());
    }
}

impl event::Handler for CheckBoxBare<()> {
    type Msg = VoidMsg;

    #[inline]
    fn activation_via_press(&self) -> bool {
        true
    }

    fn handle_action(&mut self, mgr: &mut Manager, action: Action) -> Response<VoidMsg> {
        match action {
            Action::Activate => {
                self.state = !self.state;
                mgr.redraw(self.id());
                Response::None
            }
            a @ _ => Response::unhandled_action(a),
        }
    }
}

impl<M, H: Fn(bool) -> M> event::Handler for CheckBoxBare<H> {
    type Msg = M;

    #[inline]
    fn activation_via_press(&self) -> bool {
        true
    }

    fn handle_action(&mut self, mgr: &mut Manager, action: Action) -> Response<M> {
        match action {
            Action::Activate => {
                self.state = !self.state;
                mgr.redraw(self.id());
                ((self.on_toggle)(self.state)).into()
            }
            a @ _ => Response::unhandled_action(a),
        }
    }
}

/// A checkable box with optional label
// TODO: use a generic wrapper for CheckBox and RadioBox?
#[layout(horizontal, area=checkbox)]
#[widget]
#[handler(substitutions = (OT = ()))]
#[handler(msg = M, generics = <M: From<VoidMsg>> where OT: Fn(bool) -> M)]
#[derive(Clone, Default, Widget)]
pub struct CheckBox<OT: 'static> {
    #[core]
    core: CoreData,
    #[layout_data]
    layout_data: <Self as kas::LayoutData>::Data,
    #[widget]
    checkbox: CheckBoxBare<OT>,
    #[widget]
    label: Label,
}

impl<H> Debug for CheckBox<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "CheckBox {{ core: {:?}, layout_data: {:?}, checkbox: {:?}, label: {:?} }}",
            self.core, self.layout_data, self.checkbox, self.label,
        )
    }
}

impl<M, OT: Fn(bool) -> M> CheckBox<OT> {
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
    pub fn new_on<T: Into<CowString>>(f: OT, label: T) -> Self {
        CheckBox {
            core: Default::default(),
            layout_data: Default::default(),
            checkbox: CheckBoxBare::new_on(f),
            label: Label::new(label),
        }
    }
}

impl CheckBox<()> {
    /// Construct a checkbox with a given `label`.
    ///
    /// CheckBox labels are optional; if no label is desired, use an empty
    /// string.
    #[inline]
    pub fn new<T: Into<CowString>>(label: T) -> Self {
        CheckBox {
            core: Default::default(),
            layout_data: Default::default(),
            checkbox: CheckBoxBare::new(),
            label: Label::new(label),
        }
    }

    /// Set the event handler to be called on toggle.
    ///
    /// The closure `f` is called with the new state of the checkbox when
    /// toggled, and the result of `f` is returned from the event handler.
    #[inline]
    pub fn on_toggle<M, OT: Fn(bool) -> M>(self, f: OT) -> CheckBox<OT> {
        CheckBox {
            core: self.core,
            layout_data: self.layout_data,
            checkbox: self.checkbox.on_toggle(f),
            label: self.label,
        }
    }
}

impl<OT: 'static> CheckBox<OT> {
    /// Set the initial state of the checkbox.
    #[inline]
    pub fn state(mut self, state: bool) -> Self {
        self.checkbox = self.checkbox.state(state);
        self
    }
}

impl<H> HasBool for CheckBox<H> {
    #[inline]
    fn get_bool(&self) -> bool {
        self.checkbox.get_bool()
    }

    #[inline]
    fn set_bool(&mut self, mgr: &mut Manager, state: bool) {
        self.checkbox.set_bool(mgr, state);
    }
}
