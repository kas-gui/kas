// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toggle widgets

use std::convert::TryFrom;
use std::fmt::{self, Debug};

use super::Label;
use crate::class::HasBool;
use crate::event::{Action, Handler, Manager, Response, UpdateHandle, VoidMsg};
use crate::layout::{AxisInfo, SizeRules};
use crate::macros::Widget;
use crate::theme::{DrawHandle, SizeHandle};
use crate::{Align, Alignment, CoreData, Layout, Widget, WidgetCore, WidgetId};

/// A bare radiobox (no label)
#[derive(Clone, Widget)]
pub struct RadioBoxBare<OT: 'static> {
    #[core]
    core: CoreData,
    state: bool,
    handle: UpdateHandle,
    on_activate: OT,
}

impl<H> Debug for RadioBoxBare<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "RadioBoxBare {{ core: {:?}, state: {:?}, handle: {:?}, ... }}",
            self.core, self.state, self.handle,
        )
    }
}

impl<OT: 'static> Widget for RadioBoxBare<OT> {
    fn alignment(&self) -> Alignment {
        Alignment {
            halign: Align::Centre,
            valign: Align::Centre,
            ideal: self.rect().size,
        }
    }

    fn configure(&mut self, mgr: &mut Manager) {
        mgr.update_on_handle(self.handle, self.id());
    }

    fn update_handle(&mut self, mgr: &mut Manager, _: UpdateHandle, payload: u64) {
        let id = WidgetId::try_from(payload).unwrap();
        let state = id == self.id();
        if state != self.state {
            self.state = state;
            mgr.redraw(self.id());
        }
    }

    fn allow_focus(&self) -> bool {
        true
    }
}

impl<OT: 'static> Layout for RadioBoxBare<OT> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let size = size_handle.radiobox();
        self.core_data_mut().rect.size = size;
        SizeRules::fixed(axis.extract_size(size))
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &Manager) {
        let highlights = mgr.highlight_state(self.id());
        draw_handle.radiobox(self.core.rect, self.state, highlights);
    }
}

impl<M, OT: Fn(WidgetId) -> M> RadioBoxBare<OT> {
    /// Construct a radiobox which calls `f` when toggled
    ///
    /// This is a shortcut for `RadioBoxBare::new().on_activate(f)`.
    ///
    /// All instances of [`RadioBoxBare`] and [`RadioBox`] constructed over the
    /// same `handle` will be considered part of a single group.
    ///
    /// The closure `f` is called with the new state of the radiobox when
    /// toggled, and the result of `f` is returned from the event handler.
    #[inline]
    pub fn new_on(f: OT, handle: UpdateHandle) -> Self {
        RadioBoxBare {
            core: Default::default(),
            state: false,
            handle,
            on_activate: f,
        }
    }
}

impl RadioBoxBare<()> {
    /// Construct a radiobox
    ///
    /// All instances of [`RadioBoxBare`] and [`RadioBox`] constructed over the
    /// same `handle` will be considered part of a single group.
    #[inline]
    pub fn new(handle: UpdateHandle) -> Self {
        RadioBoxBare {
            core: Default::default(),
            state: false,
            handle,
            on_activate: (),
        }
    }

    /// Set the event handler to be called on toggle.
    ///
    /// The closure `f` is called with the new state of the radiobox when
    /// toggled, and the result of `f` is returned from the event handler.
    #[inline]
    pub fn on_activate<M, OT: Fn(WidgetId) -> M>(self, f: OT) -> RadioBoxBare<OT> {
        RadioBoxBare {
            core: self.core,
            state: self.state,
            handle: self.handle,
            on_activate: f,
        }
    }
}

impl<OT: 'static> RadioBoxBare<OT> {
    /// Set the initial state of the radiobox.
    #[inline]
    pub fn state(mut self, state: bool) -> Self {
        self.state = state;
        self
    }
}

impl<H> HasBool for RadioBoxBare<H> {
    fn get_bool(&self) -> bool {
        self.state
    }

    fn set_bool(&mut self, mgr: &mut Manager, state: bool) {
        self.state = state;
        mgr.redraw(self.id());
        if state {
            mgr.trigger_update(self.handle, self.id().into());
        }
    }
}

impl Handler for RadioBoxBare<()> {
    type Msg = VoidMsg;

    #[inline]
    fn activation_via_press(&self) -> bool {
        true
    }

    fn handle_action(&mut self, mgr: &mut Manager, action: Action) -> Response<VoidMsg> {
        match action {
            Action::Activate => {
                if !self.state {
                    self.state = true;
                    mgr.redraw(self.id());
                    mgr.trigger_update(self.handle, self.id().into());
                }
                Response::None
            }
            a @ _ => Response::unhandled_action(a),
        }
    }
}

impl<M, H: Fn(WidgetId) -> M> Handler for RadioBoxBare<H> {
    type Msg = M;

    #[inline]
    fn activation_via_press(&self) -> bool {
        true
    }

    fn handle_action(&mut self, mgr: &mut Manager, action: Action) -> Response<M> {
        match action {
            Action::Activate => {
                if !self.state {
                    self.state = true;
                    mgr.redraw(self.id());
                    mgr.trigger_update(self.handle, self.id().into());
                    ((self.on_activate)(self.id())).into()
                } else {
                    Response::None
                }
            }
            a @ _ => Response::unhandled_action(a),
        }
    }
}

/// A radiobox with optional label
#[layout(horizontal, area=radiobox)]
#[widget]
#[handler(substitutions = (OT = ()))]
#[handler(msg = M, generics = <M: From<VoidMsg>> where OT: Fn(WidgetId) -> M)]
#[derive(Clone, Widget)]
pub struct RadioBox<OT: 'static> {
    #[core]
    core: CoreData,
    #[layout_data]
    layout_data: <Self as kas::LayoutData>::Data,
    #[widget]
    radiobox: RadioBoxBare<OT>,
    #[widget]
    label: Label,
}

impl<H> Debug for RadioBox<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "RadioBox {{ core: {:?}, layout_data: {:?}, radiobox: {:?}, label: {:?} }}",
            self.core, self.layout_data, self.radiobox, self.label,
        )
    }
}

impl<M, OT: Fn(WidgetId) -> M> RadioBox<OT> {
    /// Construct a radiobox with a given `label` which calls `f` when toggled.
    ///
    /// This is a shortcut for `RadioBox::new(label).on_activate(f)`.
    ///
    /// Checkbox labels are optional; if no label is desired, use an empty
    /// string.
    ///
    /// The closure `f` is called with the new state of the radiobox when
    /// toggled, and the result of `f` is returned from the event handler.
    #[inline]
    pub fn new_on<T: ToString>(f: OT, handle: UpdateHandle, label: T) -> Self {
        RadioBox {
            core: Default::default(),
            layout_data: Default::default(),
            radiobox: RadioBoxBare::new_on(f, handle),
            label: Label::new(label),
        }
    }
}

impl RadioBox<()> {
    /// Construct a radiobox with a given `label`.
    ///
    /// All instances of [`RadioBoxBare`] and [`RadioBox`] constructed over the
    /// same `handle` will be considered part of a single group.
    ///
    /// RadioBox labels are optional; if no label is desired, use an empty
    /// string.
    #[inline]
    pub fn new<T: ToString>(handle: UpdateHandle, label: T) -> Self {
        RadioBox {
            core: Default::default(),
            layout_data: Default::default(),
            radiobox: RadioBoxBare::new(handle),
            label: Label::new(label),
        }
    }

    /// Set the event handler to be called on toggle.
    ///
    /// The closure `f` is called with the new state of the radiobox when
    /// toggled, and the result of `f` is returned from the event handler.
    #[inline]
    pub fn on_activate<M, OT: Fn(WidgetId) -> M>(self, f: OT) -> RadioBox<OT> {
        RadioBox {
            core: self.core,
            layout_data: self.layout_data,
            radiobox: self.radiobox.on_activate(f),
            label: self.label,
        }
    }
}

impl<OT: 'static> RadioBox<OT> {
    /// Set the initial state of the radiobox.
    #[inline]
    pub fn state(mut self, state: bool) -> Self {
        self.radiobox = self.radiobox.state(state);
        self
    }
}

impl<H> HasBool for RadioBox<H> {
    #[inline]
    fn get_bool(&self) -> bool {
        self.radiobox.get_bool()
    }

    #[inline]
    fn set_bool(&mut self, mgr: &mut Manager, state: bool) {
        self.radiobox.set_bool(mgr, state);
    }
}
