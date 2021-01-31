// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toggle widgets

use log::trace;
use std::convert::TryFrom;
use std::fmt::{self, Debug};
use std::rc::Rc;

use super::AccelLabel;
use kas::event::{self, UpdateHandle};
use kas::prelude::*;

/// A bare radiobox (no label)
#[handler(handle=noauto)]
#[widget(config=noauto)]
#[derive(Clone, Widget)]
pub struct RadioBoxBare<M: 'static> {
    #[widget_core]
    core: CoreData,
    state: bool,
    handle: UpdateHandle,
    on_activate: Option<Rc<dyn Fn(WidgetId) -> M>>,
}

impl<M: 'static> Debug for RadioBoxBare<M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "RadioBoxBare {{ core: {:?}, state: {:?}, handle: {:?}, ... }}",
            self.core, self.state, self.handle,
        )
    }
}

impl<M: 'static> WidgetConfig for RadioBoxBare<M> {
    fn configure(&mut self, mgr: &mut Manager) {
        mgr.update_on_handle(self.handle, self.id());
    }

    fn key_nav(&self) -> bool {
        true
    }
}

impl<M: 'static> event::Handler for RadioBoxBare<M> {
    type Msg = M;

    #[inline]
    fn activation_via_press(&self) -> bool {
        true
    }

    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<M> {
        match event {
            Event::Activate => {
                if !self.state {
                    trace!("RadioBoxBare: set {}", self.id());
                    self.state = true;
                    mgr.redraw(self.id());
                    mgr.trigger_update(self.handle, self.id().into());
                    if let Some(ref f) = self.on_activate {
                        f(self.id()).into()
                    } else {
                        Response::None
                    }
                } else {
                    Response::None
                }
            }
            Event::HandleUpdate { payload, .. } => {
                let id = WidgetId::try_from(payload).unwrap();
                if self.state && id != self.id() {
                    trace!("RadioBoxBare: unset {}", self.id());
                    self.state = false;
                    mgr.redraw(self.id());
                }
                Response::None
            }
            event => Response::Unhandled(event),
        }
    }
}

impl<M: 'static> Layout for RadioBoxBare<M> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let size = size_handle.radiobox();
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
        draw_handle.radiobox(self.core.rect, self.state, self.input_state(mgr, disabled));
    }
}

impl RadioBoxBare<VoidMsg> {
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
            on_activate: None,
        }
    }

    /// Set the event handler to be called on toggle.
    ///
    /// The closure `f` is called with the new state of the radiobox when
    /// toggled, and the result of `f` is returned from the event handler.
    #[inline]
    pub fn on_activate<M, F>(self, f: F) -> RadioBoxBare<M>
    where
        F: Fn(WidgetId) -> M + 'static,
    {
        RadioBoxBare {
            core: self.core,
            state: self.state,
            handle: self.handle,
            on_activate: Some(Rc::new(f)),
        }
    }
}

impl<M: 'static> RadioBoxBare<M> {
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
    pub fn new_on<F>(f: F, handle: UpdateHandle) -> Self
    where
        F: Fn(WidgetId) -> M + 'static,
    {
        RadioBoxBare {
            core: Default::default(),
            state: false,
            handle,
            on_activate: Some(Rc::new(f)),
        }
    }

    /// Set the initial state of the radiobox.
    #[inline]
    pub fn state(mut self, state: bool) -> Self {
        self.state = state;
        self
    }
}

impl<M: 'static> HasBool for RadioBoxBare<M> {
    fn get_bool(&self) -> bool {
        self.state
    }

    fn set_bool(&mut self, state: bool) -> TkAction {
        self.state = state;
        TkAction::REDRAW
    }
}

/// A radiobox with optional label
#[layout(row, area=radiobox)]
#[handler(msg = M, generics = <> where M: From<VoidMsg>)]
#[widget(config=noauto)]
#[derive(Clone, Widget)]
pub struct RadioBox<M: 'static> {
    #[widget_core]
    core: CoreData,
    #[layout_data]
    layout_data: <Self as kas::LayoutData>::Data,
    #[widget]
    radiobox: RadioBoxBare<M>,
    #[widget]
    label: AccelLabel,
}

impl<M: 'static> Debug for RadioBox<M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "RadioBox {{ core: {:?}, layout_data: {:?}, radiobox: {:?}, label: {:?} }}",
            self.core, self.layout_data, self.radiobox, self.label,
        )
    }
}

impl<M: 'static> RadioBox<M> {
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
    pub fn new_on<T: Into<AccelString>, F>(f: F, handle: UpdateHandle, label: T) -> Self
    where
        F: Fn(WidgetId) -> M + 'static,
    {
        RadioBox {
            core: Default::default(),
            layout_data: Default::default(),
            radiobox: RadioBoxBare::new_on(f, handle),
            label: AccelLabel::new(label.into()),
        }
    }
}

impl RadioBox<VoidMsg> {
    /// Construct a radiobox with a given `label`.
    ///
    /// All instances of [`RadioBoxBare`] and [`RadioBox`] constructed over the
    /// same `handle` will be considered part of a single group.
    ///
    /// RadioBox labels are optional; if no label is desired, use an empty
    /// string.
    #[inline]
    pub fn new<T: Into<AccelString>>(handle: UpdateHandle, label: T) -> Self {
        RadioBox {
            core: Default::default(),
            layout_data: Default::default(),
            radiobox: RadioBoxBare::new(handle),
            label: AccelLabel::new(label.into()),
        }
    }

    /// Set the event handler to be called on toggle.
    ///
    /// The closure `f` is called with the new state of the radiobox when
    /// toggled, and the result of `f` is returned from the event handler.
    #[inline]
    pub fn on_activate<M, F>(self, f: F) -> RadioBox<M>
    where
        F: Fn(WidgetId) -> M + 'static,
    {
        RadioBox {
            core: self.core,
            layout_data: self.layout_data,
            radiobox: self.radiobox.on_activate(f),
            label: self.label,
        }
    }
}

impl<M: 'static> RadioBox<M> {
    /// Set the initial state of the radiobox.
    #[inline]
    pub fn state(mut self, state: bool) -> Self {
        self.radiobox = self.radiobox.state(state);
        self
    }
}

impl<M: 'static> WidgetConfig for RadioBox<M> {
    fn configure(&mut self, mgr: &mut Manager) {
        mgr.add_accel_keys(self.radiobox.id(), self.label.keys());
    }
}

impl<M: 'static> HasBool for RadioBox<M> {
    #[inline]
    fn get_bool(&self) -> bool {
        self.radiobox.get_bool()
    }

    #[inline]
    fn set_bool(&mut self, state: bool) -> TkAction {
        self.radiobox.set_bool(state)
    }
}
